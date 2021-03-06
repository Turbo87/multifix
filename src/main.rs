extern crate colored;
extern crate cursive;
#[macro_use] extern crate failure;
#[macro_use] extern crate human_panic;
extern crate ignore;
#[macro_use] extern crate lazy_static;
extern crate rayon;
extern crate regex;
extern crate webbrowser;

use std::env;
use std::fs;
use std::path::PathBuf;
use std::process::Command;

use colored::*;
use failure::Error;
use ignore::WalkBuilder;
use rayon::prelude::*;
use regex::bytes::Regex;

use ::commands::SuccessOutput;

mod commands;
mod project_select;

const GLOBAL_STEP_COUNT: u32 = 3;
const PROJECT_STEP_COUNT: u32 = 7;

fn main() {
    setup_panic!();

    let args: Vec<_> = env::args().collect();

    let root_path = args.get(1)
        .map(|it| PathBuf::from(it))
        .unwrap_or_else(|| env::current_dir().unwrap());

    let root_path = fs::canonicalize(root_path).unwrap();

    print!("{} 🔍  Searching for git projects in {}...", step(1, GLOBAL_STEP_COUNT), root_path.display());
    let git_projects: Vec<_> = find_git_projects(&root_path);
    println!(" ({} projects found)", git_projects.len());

    print!("{} 🔬  Checking projects for search pattern...", step(2, GLOBAL_STEP_COUNT));
    let relevant_projects: Vec<_> = git_projects.iter()
        .filter(|path| check_project(path))
        .collect();

    // Convert path list to path+label list and sort it by label
    let relevant_projects = add_labels_and_sort(&relevant_projects, &root_path);
    println!(" ({} projects found)", relevant_projects.len());

    // Show checkboxed list of potentially fixable projects
    print!("{} ✅  Select projects to fix...", step(3, GLOBAL_STEP_COUNT));
    let selected_projects = project_select::show_list(&relevant_projects);
    println!(" ({} projects selected)", selected_projects.len());
    println!();

    let pool = rayon::ThreadPoolBuilder::new().num_threads(5).build().unwrap();

    pool.install(|| {
        selected_projects.par_iter().for_each(|(path, label)| {
            println!("{} 📡  {} Updating project...", step(1, PROJECT_STEP_COUNT), label.dimmed());
            if let Err(err) = update_project(&path) {
                println!("ERROR: {}", err);
                return;
            }
            if let Err(err) = checkout_master(&path) {
                println!("ERROR: {}", err);
                return;
            }

            println!("{} 🔬  {} Checking project for search pattern again...", step(2, PROJECT_STEP_COUNT), label.dimmed());
            if !check_project(&path) {
                println!("Search pattern is no longer found. Skipping project!");
                return;
            }

            println!("{} 📎  {} Creating new `travis-sudo` branch...", step(3, PROJECT_STEP_COUNT), label.dimmed());
            if let Err(err) = create_branch("travis-sudo", &path) {
                println!("ERROR: {}", err);
                return;
            }

            println!("{} 🛠   {} Fixing the project...", step(4, PROJECT_STEP_COUNT), label.dimmed());
            if let Err(err) = fix_project(&path) {
                println!("ERROR: {}", err);
                return;
            }

            println!("{} 💾  {} Committing changes...", step(5, PROJECT_STEP_COUNT), label.dimmed());
            let message = "TravisCI: Remove deprecated `sudo: false` option\n\nsee https://blog.travis-ci.com/2018-11-19-required-linux-infrastructure-migration";
            if let Err(err) = commit_changes(message, &path) {
                println!("ERROR: {}", err);
                return;
            }

            println!("{} ☁️   {} Uploading changes...", step(6, PROJECT_STEP_COUNT), label.dimmed());
            let url = match push_as_new_branch(&path) {
                Err(err) => {
                    println!("ERROR: {}", err);
                    return;
                }
                Ok(Some(url)) => url,
                Ok(None) => return,
            };

            println!("{} 📬️  {} Opening browser for PR...", step(7, PROJECT_STEP_COUNT), label.dimmed());
            if let Err(err) = webbrowser::open(&url) {
                println!("ERROR: {}", err);
            }
        });
    });
}

fn step(n: u32, total: u32) -> ColoredString {
    format!("[{}/{}]", n, total).dimmed()
}

fn find_git_projects(path: &PathBuf) -> Vec<PathBuf> {
    let walker = WalkBuilder::new(path)
        .hidden(false)
        .max_depth(Some(3))
        .build();

    walker
        .inspect(|result| if let Err(err) = result {
            println!("ERROR: {}", err);
        })
        .filter_map(|result| result.ok())
        .filter(|entry| entry.file_type().map(|ft| ft.is_dir()).unwrap_or(false))
        .filter(|entry| entry.file_name() == ".git")
        .map(|entry| entry.path().parent().unwrap().to_path_buf())
        .collect()
}

fn check_project(path: &PathBuf) -> bool {
    let travis_path = {
        let mut path = path.clone();
        path.push(".travis.yml");
        path
    };

    if !travis_path.exists() {
        return false;
    }

    lazy_static! {
        static ref RE: Regex = Regex::new(r"sudo: false").unwrap();
    }

    let content = match fs::read(travis_path) {
        Ok(content) => content,
        Err(err) => {
            println!("ERROR: {}", err);
            return false;
        }
    };

    RE.is_match(&content)
}

fn add_labels_and_sort<'a>(paths: &'a Vec<&'a PathBuf>, root_path: &PathBuf) -> Vec<(&'a &'a PathBuf, String)> {
    let mut list: Vec<_> = paths.iter()
        .map(|path| {
            let relative_path = path.strip_prefix(&root_path).unwrap_or(path);
            let label = relative_path.display().to_string();
            (path, label)
        })
        .collect();

    list.sort();

    list
}

fn update_project(path: &PathBuf) -> Result<(), Error> {
    let mut git_fetch_upstream = git_fetch("upstream", &path);
    let mut git_fetch_origin = git_fetch("origin", &path);

    git_fetch_upstream.success_output()
        .or_else(|_| git_fetch_origin.success_output())
        .map(|_| ())
}

fn git_fetch(remote: &str, path: &PathBuf) -> Command {
    let mut command = Command::new("git");
    command.arg("fetch").arg(remote).current_dir(path);
    command
}


fn checkout_master(path: &PathBuf) -> Result<(), Error> {
    let mut git_checkout_upstream = git_checkout("upstream", &path);
    let mut git_checkout_origin = git_checkout("origin", &path);

    git_checkout_upstream.success_output()
        .or_else(|_| git_checkout_origin.success_output())
        .map(|_| ())
}

fn git_checkout(remote: &str, path: &PathBuf) -> Command {
    let mut command = Command::new("git");
    command.arg("checkout").arg(format!("{}/master", remote)).current_dir(path);
    command
}

fn create_branch(name: &str, path: &PathBuf) -> Result<(), Error> {
    Command::new("git")
        .arg("checkout").arg("-b").arg(name)
        .current_dir(path)
        .success_output()
        .map(|_| ())
}

fn fix_project(path: &PathBuf) -> Result<(), Error> {
    let travis_path = {
        let mut path = path.clone();
        path.push(".travis.yml");
        path
    };

    lazy_static! {
        static ref RE1: Regex = Regex::new(r"sudo: false\ndist: .*\n\n?").unwrap();
        static ref RE2: Regex = Regex::new(r"sudo: false\n\n?").unwrap();
    }

    let content = fs::read(&travis_path)?;
    let content = RE1.replace_all(&content, &b""[..]);
    let content = RE2.replace_all(&content, &b""[..]);

    fs::write(&travis_path, content)?;

    Ok(())
}

fn commit_changes(message: &str, path: &PathBuf) -> Result<(), Error> {
    Command::new("git")
        .arg("add").arg(".")
        .current_dir(path)
        .success_output()?;

    Command::new("git")
        .arg("commit").arg("-m").arg(message)
        .current_dir(path)
        .success_output()
        .map(|_| ())
}

fn push_as_new_branch(path: &PathBuf) -> Result<Option<String>, Error> {
    let output = Command::new("git")
        .arg("push").arg("origin").arg("HEAD").arg("-u")
        .current_dir(path)
        .success_output()?;

    lazy_static! {
        static ref RE: Regex = Regex::new(r"https://github.com/.*\n").unwrap();
    }

    let cap = match RE.find(&output.stderr) {
        Some(cap) => cap,
        None => {
            println!("ERROR: Could not find PR URL in:\n{}", std::str::from_utf8(&output.stderr).unwrap());
            return Ok(None);
        }
    };

    Ok(Some(std::str::from_utf8(cap.as_bytes()).unwrap().trim().to_owned()))
}
