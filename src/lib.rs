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

use std::io::Read;

use anyhow::{
    Context,
    Result,
};
use constants::BUILD_DIR;
use rhai::{
    Engine,
    EvalAltResult,
};

/// Defined for convenience
type Dict = std::collections::HashMap<String, String>;

/// Prints the result of grading
pub fn grade(script_url: &str) -> Result<()> {
    let mut engine = Engine::new();
    engine.register_result_fn("clean", clean_script);

    // Download grading script
    let script = {
        let resp = ureq::get(script_url)
            .call()
            .context(format!("Failed to download {}", script_url))?;

        let len = resp
            .header("Content-Length")
            .and_then(|s| s.parse::<usize>().ok())
            .unwrap();

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
    // Your first Rhai Script

    // Run the script - prints "42"
    engine.run(&script)?;

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
