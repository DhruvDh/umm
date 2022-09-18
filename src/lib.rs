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
    Context,
    Result,
};
use constants::{
    BUILD_DIR,
    COURSE,
    LIB_DIR,
    POSTGREST_CLIENT,
    ROOT_DIR,
    RUNTIME_HANDLE,
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

/// Defined for convenience
type Dict = std::collections::HashMap<String, String>;

/// Prints the result of grading
pub fn grade(assignment_name: &str) -> Result<()> {
    let assignment_name = assignment_name.to_string();
    let assignment_name = assignment_name.replace(['\"', '\\'], "");
    dbg!(&assignment_name);
    let mut engine = Engine::new();
    engine
        .register_type::<GradeResult>()
        .register_type_with_name::<FileType>("JavaFileType")
        .register_type_with_name::<File>("JavaFile")
        .register_type_with_name::<Project>("JavaProject")
        .register_fn("show_results", show_result)
        .register_fn("clean", clean_script)
        .register_fn("new_project", Project::new_script)
        .register_fn("identify", Project::identify_script)
        .register_fn("check", File::check_script)
        .register_fn("run", File::run_script)
        .register_fn("test", File::test_script)
        .register_fn("grade_docs", grade_docs_script)
        .register_fn("grade_unit_tests", grade_unit_tests_script)
        .register_fn("grade_by_hidden_tests", grade_by_hidden_tests_script)
        .register_fn("grade_by_tests", grade_by_tests_script);

    // println!("{}", engine.gen_fn_signatures(false).join("\n"));
    let rt = RUNTIME_HANDLE.handle().clone();

    let resp = rt.block_on(async {
        POSTGREST_CLIENT
            .from("grading_scripts")
            .eq("course", COURSE)
            .eq("term", TERM)
            .eq("assignment", &assignment_name)
            .select("url")
            .single()
            .execute()
            .await?
            .text()
            .await
            .context(format!(
                "Could not get grading script for {assignment_name}"
            ))
    });
    let resp: serde_json::Value = serde_json::from_str(resp?.as_str())?;
    let resp = resp.as_object().unwrap();

    if let Some(message) = resp.get("message") {
        anyhow::bail!("Error: {message}");
    }

    let script_url = resp.get("url").unwrap().as_str().unwrap();

    let script = reqwest::blocking::get(script_url)
        .context(format!("Cannot get url: {script_url}"))?
        .text()
        .context(format!(
            "Could not parse the response from {script_url} to text."
        ))?;
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
