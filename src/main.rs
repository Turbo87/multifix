extern crate colored;
extern crate ignore;
#[macro_use] extern crate lazy_static;
extern crate regex;

use std::env;
use std::fs;
use std::path::PathBuf;

use colored::*;
use ignore::WalkBuilder;
use regex::bytes::Regex;

fn main() {
    let args: Vec<_> = env::args().collect();

    let path = args.get(1)
        .map(|it| PathBuf::from(it))
        .unwrap_or_else(|| env::current_dir().unwrap());

    let path = fs::canonicalize(path).unwrap();

    println!("{} 🔍  Searching for git projects in {}...", step(1), path.display());
    let git_projects: Vec<_> = find_git_projects(&path);

    println!("{} 🔬  Checking {} projects for search pattern...", step(2), git_projects.len());
    let relevant_projects: Vec<_> = git_projects.iter()
        .filter(|path| check_project(path))
        .collect();

    println!("\nFound {} relevant projects", relevant_projects.len())

    // - show checkboxed list of potentially fixable projects
    // - update checked projects (git fetch)
    // - check again on `upstream/master` or `origin/master`
    // - create branch at `upstream/master` or `origin/master`
    // - fix code
    // - commit changes
    // - push new branch to `origin`
    // - open browser with PR URL
}

fn step(n: u32) -> ColoredString {
    const STEP_COUNT: u32 = 2;
    format!("[{}/{}]", n, STEP_COUNT).dimmed()
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
        static ref RE: Regex = Regex::new(r"sudo: false\ndist: trusty").unwrap();
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
