//! # umm
//!
//! A scriptable build tool/grader/test runner for Java projects that don't use
//! package managers.

#![warn(missing_docs)]
#![warn(clippy::missing_docs_in_private_items)]
#![feature(label_break_value)]

/// A module defining a bunch of constant values to be used throughout
pub mod constants;
/// For all things related to grading
pub mod grade;
/// For discovering Java projects, analyzing them, and generating/executing
/// build tasks
pub mod java;
/// Utility functions for convenience
pub mod util;

use anyhow::{
    Context,
    Result,
};
use constants::BUILD_DIR;
use tabled::Table;

use crate::{
    grade::*,
    java::Project,
};
/// Defined for convenience
type Dict = std::collections::HashMap<String, String>;

/// Prints the result of grading
pub fn grade() -> Result<()> {
    let project = Project::new()?;

    let req_1 = grade_docs(vec!["pyramid_scheme.LinkedTree"], &project, 10, "1".into())?;

    let req_2 = grade_by_tests(
        vec![String::from("pyramid_scheme.LinkedTreeTest")],
        vec![
            String::from("pyramid_scheme.LinkedTreeTest#testGetRootElement"),
            "pyramid_scheme.LinkedTreeTest#testAddChild".into(),
            "pyramid_scheme.LinkedTreeTest#testFindNode".into(),
            "pyramid_scheme.LinkedTreeTest#testContains".into(),
            "pyramid_scheme.LinkedTreeTest#testSize".into(),
        ],
        &project,
        20.0,
        "2".to_string(),
    )?;

    let req_3 = grade_unit_tests(
        "2".into(),
        20.0,
        vec![String::from("pyramid_scheme.LinkedTreeTest")],
        vec![String::from("pyramid_scheme.LinkedTree")],
        vec![
            String::from("toString"),
            "LinkedTree".into(),
            "getRootElement".into(),
            "findNode".into(),
            "size".into(),
            "leafCounter".into(),
            "isEmpty".into(),
        ],
        vec![
            String::from("pyramid_scheme.MultiNodeTree"),
            "DataStructures.*".into(),
            "pyramid_scheme.Person".into(),
            "pyramid_scheme.PyramidScheme".into(),
            "pyramid_scheme.PyramidSchemeSim".into(),
        ],
    )?;

    let req_4 = grade_docs(
        vec!["pyramid_scheme.PyramidScheme"],
        &project,
        10,
        "3".into(),
    )?;

    let req_5 = grade_by_tests(
        vec![String::from("pyramid_scheme.PyramidSchemeTest")],
        vec![
            String::from("pyramid_scheme.PyramidSchemeTest#testWhoBenefits"),
            String::from("pyramid_scheme.PyramidSchemeTest#testAddChild"),
            String::from("pyramid_scheme.PyramidSchemeTest#testInitiateCollapse"),
        ],
        &project,
        30.0,
        "3".into(),
    )?;
    let grades = &vec![req_1, req_2, req_3, req_4, req_5];
    println!("{}", Table::new(grades).with(tabled::Style::modern()));

    let total = grades
        .iter()
        .fold(0u32, |acc, x| acc + x.grade().parse::<u32>().unwrap_or(0));

    println!("Total: {}", total);
    Ok(())
}

/// Deletes all java compiler artefacts
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
