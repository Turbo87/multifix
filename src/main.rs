extern crate colored;
extern crate cursive;
extern crate ignore;
#[macro_use] extern crate lazy_static;
extern crate regex;

use std::env;
use std::fs;
use std::path::PathBuf;

use colored::*;
use cursive::Cursive;
use cursive::event::Key;
use cursive::traits::{Boxable, Identifiable, Scrollable};
use cursive::views::{Checkbox, DummyView, ListView};
use ignore::WalkBuilder;
use regex::bytes::Regex;

fn main() {
    let args: Vec<_> = env::args().collect();

    let root_path = args.get(1)
        .map(|it| PathBuf::from(it))
        .unwrap_or_else(|| env::current_dir().unwrap());

    let root_path = fs::canonicalize(root_path).unwrap();

    println!("{} 🔍  Searching for git projects in {}...", step(1), root_path.display());
    let git_projects: Vec<_> = find_git_projects(&root_path);

    println!("{} 🔬  Checking {} projects for search pattern...", step(2), git_projects.len());
    let relevant_projects: Vec<_> = git_projects.iter()
        .filter(|path| check_project(path))
        .collect();

    // Convert path list to path+label list and sort it by label
    let relevant_projects = add_labels_and_sort(&relevant_projects, &root_path);

    // Show checkboxed list of potentially fixable projects
    let selected_projects = show_project_list(&relevant_projects);

    println!("\nSelected {} relevant projects", selected_projects.len())

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

fn show_project_list<'a>(input: &'a Vec<(&'a &'a PathBuf, String)>) -> Vec<&'a (&'a &'a PathBuf, String)> {
    let mut siv = Cursive::default();

    let list_view = {
        let mut list = ListView::new();

        list.add_child("Please select the projects to fix. Confirm selection with <F5>.", DummyView);
        list.add_delimiter();

        for (_, label) in input.iter() {
            list.add_child(&label, Checkbox::new().with_id(label.clone()));
        }

        list.with_id("list").scrollable().full_screen()
    };

    siv.add_fullscreen_layer(list_view);

    siv.add_global_callback(Key::F5, Cursive::quit);

    siv.run();

    input.iter()
        .filter(|(_, label)| siv.find_id::<Checkbox>(&label).unwrap().is_checked())
        .collect()
}
