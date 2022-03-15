use anyhow::{anyhow, ensure, Context, Result};
use junit_summary_parser::num_tests_found;
use std::{
    path::PathBuf,
    process::{Command, Stdio},
};
use tabled::{Table, Tabled};
use umm::*;

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
        rule found_tests()
            = "tests found"
        pub rule num_tests_passed() -> u32
            = "[" whitespace()? l:number() whitespace()? successful_tests() whitespace()? "]" { l }

        pub rule num_tests_found() -> u32
            = "[" whitespace()? l:number() whitespace()? found_tests() whitespace()? "]" { l }
    }
}

#[fncmd::fncmd]
/// Run JUnit tests from a JUnit test class (source) file
pub fn main() -> Result<()> {
    let project = JavaProject::new()?;

    let req_2 = {
        let mut grade = 20;
        let mut reasons = vec![];
        if project
            .doc_check("Shopping.ShoppingListArrayList".to_string())
            .is_err()
        {
            grade = grade - 10;
            reasons.push("- Incomplete documentation for ShoppingListArrayList.");
        }
        if project
            .doc_check("Shopping.ShoppingListArray".to_string())
            .is_err()
        {
            grade = grade - 10;
            reasons.push("- Incomplete documentation for ShoppingListArray.");
        }

        GradeResult {
            Requirement: 2,
            Grade: format!("{}/20", grade),
            Reason: reasons.join("\n"),
        }
    };

    let req_1 = {
        let name = "Shopping.ShoppingListArrayListTest";
        let res = project.test(name.to_string())?;
        let mut expected_tests = vec![
            String::from("Shopping.ShoppingListArrayListTest#testAdd"),
            String::from("Shopping.ShoppingListArrayListTest#testRemove"),
            String::from("Shopping.ShoppingListArrayListTest#testFind"),
            String::from("Shopping.ShoppingListArrayListTest#testIndexOf"),
            String::from("Shopping.ShoppingListArrayListTest#testContains"),
            String::from("Shopping.ShoppingListArrayListTest#testSize"),
            String::from("Shopping.ShoppingListArrayListTest#testIsEmpty"),
        ];
        expected_tests.sort();

        let index = project.names.iter().position(|x| x == &name);
        ensure!(
            index.is_some(),
            "Could not find class/interface with name {}.",
            name
        );
        let file = &project.files[index.unwrap()];
        let name = file.proper_name.clone().unwrap();

        let mut tests = file.test_methods.clone();
        tests.sort();

        let mut reasons = vec![];
        for expected in &expected_tests {
            let n = expected.split_once('#').unwrap().1;
            if !tests.contains(expected) {
                reasons.push(format!("- {} not found.", n));
            }
        }

        for actual in &tests {
            let n = actual.split_once('#').unwrap().1;
            if !expected_tests.contains(actual) {
                reasons.push(format!("- Unexpected test called {}", n));
            }
        }

        if reasons.len() != 0 {
            GradeResult {
                Requirement: 1,
                Grade: format!("0/40"),
                Reason: reasons.join("\n"),
            }
        } else {
            let mut num_tests_passed = 0.0;
            let mut num_tests_total = 0.0;

            for line in res.lines() {
                let parse_result = junit_summary_parser::num_tests_passed(line)
                    .context("While parsing Junit summary table");
                if let Ok(n) = parse_result {
                    num_tests_passed = n as f32;
                }
                let parse_result = junit_summary_parser::num_tests_found(line)
                    .context("While parsing Junit summary table");
                if let Ok(n) = parse_result {
                    num_tests_total = n as f32;
                }
            }
            let grade = if num_tests_total != 0.0 {
                (num_tests_passed / num_tests_total) * 80.0
            } else {
                0.0
            };
            GradeResult {
                Requirement: 1,
                Grade: format!("{:.2}/80.0", grade),
                Reason: format!("- {}/{} tests passing.", num_tests_passed, num_tests_total),
            }
        }
    };

    let req_3 = {
        let child = Command::new(java_path()?)
            .stdin(Stdio::inherit())
            .stdout(Stdio::inherit())
            .stderr(Stdio::inherit())
            .args([
                "--class-path",
                classpath()?.as_str(),
                "org.pitest.mutationtest.commandline.MutationCoverageReport",
                "--reportDir",
                "test_reports",
                "--failWhenNoMutations",
                "false",
                "--targetClasses",
                "Shopping.ShoppingListArrayList",
                "--targetTests",
                "Shopping.*",
                "--sourceDirs",
                SOURCE_DIR.to_str().unwrap(),
            ])
            .output()
            .context("Failed to spawn javac process.")?;

        GradeResult {
            Requirement: 3,
            Grade: format!("40/40"),
            Reason: format!(""),
        }
    };
    println!(
        "{}",
        Table::new(vec![req_1, req_2]).with(tabled::Style::modern())
    );
    Ok(())
}
