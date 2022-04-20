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
    let project = JavaProject::new()?;

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
        vec![
            String::from("pyramid_scheme.LinkedTreeTest#testGetRootElement"),
            "pyramid_scheme.LinkedTreeTest#testAddChild".into(),
            "pyramid_scheme.LinkedTreeTest#testFindNode".into(),
            "pyramid_scheme.LinkedTreeTest#testContains".into(),
            "pyramid_scheme.LinkedTreeTest#testSize".into(),
        ],
        vec![],
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

    println!(
        "{}",
        Table::new(vec![req_1, req_2, req_3, req_4, req_5]).with(tabled::Style::modern())
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
