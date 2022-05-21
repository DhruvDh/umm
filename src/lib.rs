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
use rhai::{
    Engine,
    EvalAltResult,
};
use tabled::Table;

/// Defined for convenience
type Dict = std::collections::HashMap<String, String>;

/// Prints the result of grading
pub fn grade() -> Result<()> {
    let mut engine = Engine::new();
    engine.register_result_fn("clean", clean_script);

    // Download grading script

    // Your first Rhai Script
    let script = "clean();";

    // Run the script - prints "42"
    engine.run(script)?;

    Ok(())
}

/// Deletes all java compiler artefacts
pub fn clean() -> Result<()> {
    std::fs::remove_dir_all(BUILD_DIR.as_path())
        .with_context(|| format!("Could not delete {}", BUILD_DIR.display()))
}

fn clean_script() -> Result<(), Box<EvalAltResult>> {
    match clean() {
        Ok(_) => Ok(()),
        Err(e) => Err(e.to_string().into()),
    }
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
