pub mod constants;
pub mod grade;
pub mod java;
pub mod util;

use anyhow::{Context, Result};
use constants::BUILD_DIR;
use tabled::Table;

use crate::{grade::*, java::Project};
/// Defined for convenience
type Dict = std::collections::HashMap<String, String>;

pub fn grade() -> Result<()> {
    let project = Project::new()?;

    let req_1 = grade_by_tests(
        vec![String::from("DataStructures.LinkedStackTest")],
        vec![
            String::from("DataStructures.LinkedStackTest#testPop"),
            String::from("DataStructures.LinkedStackTest#testPush"),
            String::from("DataStructures.LinkedStackTest#testPeek"),
            String::from("DataStructures.LinkedStackTest#testSize"),
            String::from("DataStructures.LinkedStackTest#testToString"),
            String::from("DataStructures.LinkedStackTest#testIsEmpty"),
        ],
        &project,
        50.0,
        "1".to_string(),
    )?;

    let req_2 = grade_docs(vec!["DataStructures.LinkedStack"], &project, 20, "2".into())?;

    let req_3 = grade_unit_tests(
        "3".to_string(),
        30.0,
        vec![String::from("DataStructures.LinkedStackTest")],
        vec![String::from("DataStructures.LinkedStack")],
        vec![
            String::from("LinkedStack"),
            String::from("isEmpty"),
            String::from("size"),
            String::from("toString"),
            String::from("main"),
        ],
    )?;
    println!(
        "{}",
        Table::new(vec![req_1, req_2, req_3]).with(tabled::Style::modern())
    );
    Ok(())
}

pub fn clean() -> Result<()> {
    std::fs::remove_dir_all(BUILD_DIR.as_path())
        .with_context(|| format!("Could not delete {}", BUILD_DIR.display()))
}

// TODO: Add documentations everywhere
// TODO: replace std::Command with cmd_lib
// TODO: Fix java mod impls
// TODO: remove fncmd
// TODO: use reedline for shell-like interface
// TODO: update classpath when discovering project
// TODO: fix grading api, move to grade module.
// TODO: add rhai scripting for grading
// TODO: find a way to generate a rhai wrapper for all methods
// TODO: add rhai scripting for project init
// TODO: update tabled to 0.6
