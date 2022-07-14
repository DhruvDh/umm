//! # umm
//!
//! A scriptable build tool/grader/test runner for Java projects that don't use
//! package managers.

#![warn(missing_docs)]
#![warn(clippy::missing_docs_in_private_items)]
#![feature(label_break_value)]
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

use std::io::Read;

use anyhow::{
    Context,
    Result,
};
use constants::BUILD_DIR;
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
    // Download grading script
    let script = {
        let resp = ureq::get(script_url)
            .call()
            .context(format!("Failed to download {}", script_url))?;

        let len = resp
            .header("Content-Length")
            .and_then(|s| s.parse::<usize>().ok())
            .unwrap_or(1024);

        let mut bytes: Vec<u8> = Vec::with_capacity(len);

        resp.into_reader()
            .take(10_000_000)
            .read_to_end(&mut bytes)
            .context(format!(
                "Failed to read response till the end while downloading file at {}",
                script_url,
            ))?;

        String::from_utf8(bytes)?
    };

    // Run the script
    engine.run(&script)?;

    Ok(())
}

#[generate_rhai_variant]
/// Deletes all java compiler artefacts
pub fn clean() -> Result<()> {
    std::fs::remove_dir_all(BUILD_DIR.as_path())
        .with_context(|| format!("Could not delete {}", BUILD_DIR.display()))
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
