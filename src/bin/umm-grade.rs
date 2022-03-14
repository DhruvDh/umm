use anyhow::{anyhow, Result};
use std::path::PathBuf;
use umm::*;
use tabled::{Table, Tabled};

#[derive(Tabled)]
/// A struct to store grading results and display them
///
/// * `Requirement`: refers to Requirement ID  
/// * `Grade`: grade received for above Requirement
/// * `Reason`: the reason for penalties applied, if any  
struct GradeResult {
    Requirement: i32,
    Grade: String,
    Reason: String,
}

peg::parser! {
    grammar junit_summary_parser() for str {
        rule number() -> u32
            = n:$(['0'..='9']+) {? n.parse().or(Err("u32")) }
        rule whitespace() = quiet!{[' ' | '\n' | '\t']+}
        rule successful_tests()
            = "tests successful"
        rule total_tests()
            = "tests found"
        pub rule num_tests_passed() -> u32
            = "[" whitespace()? l:number() whitespace()? successful_tests() whitespace()? "]" { l }

        pub rule num_tests_found() -> u32
            = "[" whitespace()? l:number() whitespace()? total_tests() whitespace()? "]" { l }
    }
}

/// Run JUnit tests from a JUnit test class (source) file
#[fncmd::fncmd]
pub fn main() -> Result<()> {
    let project = JavaProject::new()?;
    Ok(())
}
