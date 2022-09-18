//! # umm
//!
//! A scriptable build tool/grader/test runner for Java projects that don't use
//! package managers.

#![warn(missing_docs)]
#![warn(clippy::missing_docs_in_private_items)]
#![feature(iterator_try_collect)]

/// A module defining a bunch of constant values to be used throughout
pub mod constants;
/// For all things related to grading
pub mod grade;
/// For discovering Java projects, analyzing them, and generating/executing
/// build tasks
pub mod java;
/// Utility functions for convenience
pub mod util;
/// For structs and enums related to VSCode Tasks
pub mod vscode;

use anyhow::{
    anyhow,
    Context,
    Result,
};
use constants::{
    BUILD_DIR,
    COURSE,
    GRADING_SCRIPTS_URL,
    LIB_DIR,
    ROOT_DIR,
    TERM,
};
use grade::*;
use java::{
    File,
    FileType,
    Project,
};
use rhai::{
    Engine,
    EvalAltResult,
};
use umm_derive::generate_rhai_variant;
use util::download_to_string;

/// Defined for convenience
type Dict = std::collections::HashMap<String, String>;

/// Prints the result of grading
pub fn grade(assignment_name: &str) -> Result<()> {
    let mut engine = Engine::new();
    engine
        .register_type::<GradeResult>()
        .register_type_with_name::<FileType>("JavaFileType")
        .register_type_with_name::<File>("JavaFile")
        .register_type_with_name::<Project>("JavaProject")
        .register_fn("show_results", show_result)
        .register_result_fn("clean", clean_script)
        .register_result_fn("new_project", Project::new_script)
        .register_result_fn("identify", Project::identify_script)
        .register_result_fn("check", File::check_script)
        .register_result_fn("run", File::run_script)
        .register_result_fn("test", File::test_script)
        .register_result_fn("grade_docs", grade_docs_script)
        .register_result_fn("grade_unit_tests", grade_unit_tests_script)
        .register_result_fn("grade_by_hidden_tests", grade_by_hidden_tests_script)
        .register_result_fn("grade_by_tests", grade_by_tests_script);

    // println!("{}", engine.gen_fn_signatures(false).join("\n"));

    let grading_scripts = download_to_string(GRADING_SCRIPTS_URL)?;
    let grading_scripts: serde_json::Value = serde_json::from_str(&grading_scripts)?;
    let script_url = grading_scripts
        .get(COURSE)
        .ok_or_else(|| anyhow!("Could not find course: {}", COURSE))?
        .get(TERM)
        .ok_or_else(|| anyhow!("Could not find term {} in {}", TERM, COURSE))?
        .get(assignment_name)
        .ok_or_else(|| {
            anyhow!(
                "No grading script found for {} in {}-{}",
                assignment_name,
                COURSE,
                TERM
            )
        })?;
    let script_url = script_url.as_str().ok_or_else(|| {
        anyhow!(
            "Script URL for {} in {}-{} is not a string",
            assignment_name,
            COURSE,
            TERM
        )
    })?;

    let script = download_to_string(script_url)?;
    // Run the script
    engine.run(&script)?;

    Ok(())
}

#[generate_rhai_variant]
/// Deletes all java compiler artefacts
pub fn clean() -> Result<()> {
    if BUILD_DIR.as_path().exists() {
        std::fs::remove_dir_all(BUILD_DIR.as_path())
            .with_context(|| format!("Could not delete {}", BUILD_DIR.display()))?;
    }
    if LIB_DIR.as_path().exists() {
        std::fs::remove_dir_all(LIB_DIR.as_path())
            .with_context(|| format!("Could not delete {}", LIB_DIR.display()))?;
    }
    if ROOT_DIR.join(".vscode").as_path().exists() {
        std::fs::remove_dir_all(ROOT_DIR.join(".vscode").as_path())
            .with_context(|| format!("Could not delete {}", ROOT_DIR.join(".vscode").display()))?;
    }

    Ok(())
}

// TODO: replace std::Command with cmd_lib
// TODO: Lazily load all constants from rhai scripts instead
// TODO: Fix java mod impls
// TODO: update classpath when discovering project
// TODO: fix grading api
// TODO: add rhai scripting for grading
// TODO: find a way to generate a rhai wrapper for all methods
// TODO: add rhai scripting for project init
// TODO: update tabled to 0.6
// TODO: make reedline shell optional behind a feature
// TODO: Download jars only if required OR remove jar requirement altogether.
