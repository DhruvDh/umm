use anyhow::{ensure, Context, Result};
use lazy_static::__Deref;
use std::{
    fs::File,
    io::{BufRead, BufReader},
    process::{Command, Stdio},
};
use tabled::{
    display::ExpandedDisplay, Alignment, Footer, Full, Header, MaxWidth, Modify, Row, Table, Tabled,
};
use umm::*;

#[derive(Tabled, Clone)]
#[allow(non_snake_case)]
/// A struct to store grading results and display them
///
/// * `Requirement`: refers to Requirement ID  
/// * `Grade`: grade received for above Requirement
/// * `Reason`: the reason for penalties applied, if any  
struct GradeResult {
    Requirement: String,
    Grade: String,
    Reason: String,
}

#[derive(Tabled)]
/// A struct representing a javac diagnostic message
///
/// * `path`: path to the file diagnostic is referring to  
/// * `file_name`: name of the file the diagnostic is about
/// * `line_number`: line number
/// * `is_error`: boolean value, is true if error or false if the diagnostic is a warning
/// * `message`: the diagnostic message
pub struct JavacDiagnostic {
    #[header(hidden = true)]
    path: String,
    #[header("File")]
    file_name: String,
    #[header("Line")]
    line_number: u32,
    #[header(hidden = true)]
    is_error: bool,
    #[header("Message")]
    message: String,
}

#[derive(Debug, Tabled)]
/// A struct representing a PIT diagnostic message
///
/// * `mutator`: name of the mutator in question
/// * `source_method`: name of the source method being mutated
/// * `line_number`: source line number where mutation occured
/// * `test_method`: name of the test examined
/// * `result`: result of mutation testing
pub struct MutationDiagnostic {
    #[header("Mutation type")]
    mutator: String,
    #[header("Source method mutated")]
    source_method: String,
    #[header("Line no. of mutation")]
    line_number: u32,
    #[header("Test examined")]
    test_method: String,
    #[header("Result")]
    result: String,
    #[header(hidden = true)]
    source_file_name: String,
    #[header(hidden = true)]
    test_file_name: String,
}
peg::parser! {
    grammar parser() for str {
        /// matches any sequeuce of 1 or more numbers
        rule number() -> u32
            = n:$(['0'..='9']+) {? n.parse().or(Err("u32")) }

        /// matches any number of whitespace characters
        rule whitespace() = quiet!{[' ' | '\n' | '\t' | '\r']+}

        /// matches the keyword "tests successful"
        rule successful_tests()
            = "tests successful"

        /// matches the keyword "tests found"
        rule found_tests()
            = "tests found"

        /// parses and returns the number of tests passed
        pub rule num_tests_passed() -> u32
            = "[" whitespace()? l:number() whitespace()? successful_tests() whitespace()? "]" { l }

        /// parses and returns the number of tests found
        pub rule num_tests_found() -> u32
            = "[" whitespace()? l:number() whitespace()? found_tests() whitespace()? "]" { l }

        /// matches any path separator, hopefully cross-platform
        rule path_separator() =
            whitespace()?
            "."?
            "/" / "\\" / "\\\\"
            whitespace()?

        /// matches any sequence of upper and lowercase alphabets
        rule word() -> String
            = whitespace()?
                w:[
                    'a'..='z' |
                    'A'..='Z' |
                    '0'..='9' |
                    '-' | '.' | ' ' |
                    '[' | ']' | '_'
                ]+
                whitespace()?
            { w.iter().collect::<String>() }

        /// matches any sequence of upper and lowercase alphabets
        rule mutations_csv_word() -> String
            = whitespace()?
                w:[
                    'a'..='z' |
                    'A'..='Z' |
                    '0'..='9' |
                    '-' | '.' | ' ' |
                    '[' | ']' | ':' |
                    '<' | '>' | '_' |
                    '(' | ')'
                ]+
                whitespace()?
            { w.iter().collect::<String>() }

        /// matches any valid path, hopefully
        rule path() -> String
            = whitespace()?
              path_separator()?
              p:(word() ++ path_separator())
              whitespace()?
            { p.iter().fold(String::new(), |acc, w| format!("{}/{}", acc, w)) }

        /// matches line numbers (colon followed by numbers, eg. :23)
        rule line_number() -> u32
            = ":" n:number() ":" whitespace()? { n }

        /// matches "error" or "warning", returns true if error
        rule diag_type() -> bool
            = whitespace()?
              a:"error"? b:"warning"?
              ":"
              whitespace()?
            { a.is_some() }

        /// mactches anything, placed where diagnostic should be
        rule diagnostic() -> String
            = a:([_]+)
            { a.iter().collect::<String>() }

        /// parses the first line of a javac diagnostic message and returns a `JavacDiagnostic`
        pub rule parse_diag() -> JavacDiagnostic
            = p:path() l:line_number() d:diag_type() m:diagnostic()
            {
                let p = std::path::PathBuf::from(p);
            let name = p.file_name().expect("Could not parse path to file in javac error/warning");
                JavacDiagnostic {
                path: p.display().to_string(),
                file_name: name.to_string_lossy().to_string(),
                line_number: l,
                is_error: d,
                message: if d { format!("Error: {}", m) } else { m }
            }
            }

        rule mutation_test_examined_path() -> Vec<String>
            = a:mutations_csv_word()? "/"? b:mutations_csv_word()? "/"?  c:mutations_csv_word()?
            {
                let mut res = vec![];
                if let Some(a) = a { res.push(a.to_string()); }
                if let Some(b) = b { res.push(b.to_string()); }
                if let Some(c) = c { res.push(c.to_string()); }
                res
            }

        rule mutation_test_examined_none() -> &'input str
            = $("none")

        pub rule mutation_report_row() -> MutationDiagnostic
            = file_name:word()
              ","
              source_file_name:word()
              ","
              mutation:word()
              ","
              source_method:mutations_csv_word()
              ","
              line_no:number()
              ","
              result:word()
              ","
              test_method:mutation_test_examined_path()?
              whitespace()?
                {
                let test = test_method.expect(format!("Had trouble parsing last column for mutation at {}#{}:{}", source_file_name, source_method, line_no).as_str());
                let mut test_file_name;
                let mut test_method;

    if test.len() == 3 {
                    let splitter = if test.get(1).unwrap().contains("[runner:") { "[runner:" } else { "[class:" };
                    test_file_name = test.get(1)
                                .unwrap()
                                .to_string()
                                .split_once(splitter)
                                .expect(format!("had trouble parsing test_file_class for mutation at {}#{}:{}", source_file_name, source_method, line_no).as_str())
                                .1
                                .replace("]", "");

                    let splitter = if test.get(2).unwrap().contains("[test:") { "[test:" } else { "[method:" };
                    test_method = test.get(2)
                                    .unwrap()
                                    .to_string()
                                    .split_once(splitter)
                                    .expect(format!("Had trouble parsing test_file_method for mutation at {}#{}:{}", source_file_name, source_method, line_no).as_str())
                                    .1
                                    .replace("()]", "");
                } else {
                    test_file_name = "NA".to_string();
                    test_method = "None".to_string()
                }
                let mutator = mutation
                                .to_string()
                                .split_once(".mutators.")
                                .expect("Could not split mutators while parsing mutations.csv.")
                                .1.to_string();
                MutationDiagnostic {
                    line_number: line_no,
                    mutator,
                    source_file_name,
                    source_method,
                    test_file_name,
                    test_method,
                    result
                }
            }
    }
}

fn grade_docs(
    files: Vec<&str>,
    project: &JavaProject,
    out_of: u32,
    req_name: String,
) -> Result<GradeResult> {
    let mut diags = vec![];

    for name in &files {
        let file = project.identify(*name)?;
        let output = file.doc_check()?;
        for line in output.lines() {
            let result = parser::parse_diag(line);
            match result {
                Ok(res) => {
                    if file.file_name == res.file_name {
                        diags.push(res);
                    }
                }
                Err(_) => continue,
            }
        }
    }

    let penalty = diags.len() as u32 * 3;
    let grade = out_of.saturating_sub(penalty);
    let num_diags = diags.len();
    println!(
        "{}",
        Table::new(diags)
            .with(Header(format!("Check javadoc for {}", files.join(", "))))
            .with(Footer(format!("-{} due to {} nits", penalty, num_diags)))
            .with(Modify::new(Row(1..)).with(MaxWidth::wrapping(36)))
            .with(Modify::new(Full).with(Alignment::center_horizontal()))
            .with(tabled::Style::modern())
    );

    Ok(GradeResult {
        Requirement: req_name,
        Grade: format!("{}/{}", grade, out_of),
        Reason: String::from("See above."),
    })
}

fn grade_by_tests(
    test_files: Vec<String>,
    expected_tests: Vec<String>,
    project: &JavaProject,
    out_of: f32,
    req_name: String,
) -> Result<GradeResult> {
    let mut actual_tests = vec![];
    let mut expected_tests = expected_tests.clone();
    let mut reasons = vec![];
    expected_tests.sort();

    for test_file in &test_files {
        let test_file = project.identify(test_file)?;

        actual_tests.append(&mut test_file.test_methods.clone());
    }
    actual_tests.sort();

    for expected in &expected_tests {
        let n = expected.split_once('#').unwrap().1;
        if !actual_tests.contains(expected) {
            reasons.push(format!("- {} not found.", n));
        }
    }

    for actual in &actual_tests {
        let n = actual.split_once('#').unwrap().1;
        if !expected_tests.contains(actual) {
            reasons.push(format!("- Unexpected test called {}", n));
        }
    }

    if !reasons.is_empty() {
        Ok(GradeResult {
            Requirement: req_name,
            Grade: format!("0.00/{:.2}", out_of),
            Reason: reasons.join("\n"),
        })
    } else {
        let mut num_tests_passed = 0.0;
        let mut num_tests_total = 0.0;
        for test_file in test_files {
            let res = project.identify(test_file)?.test()?;

            for line in res.lines() {
                let parse_result =
                    parser::num_tests_passed(line).context("While parsing Junit summary table");
                if let Ok(n) = parse_result {
                    num_tests_passed = n as f32;
                }
                let parse_result =
                    parser::num_tests_found(line).context("While parsing Junit summary table");
                if let Ok(n) = parse_result {
                    num_tests_total = n as f32;
                }
            }
        }
        let grade = if num_tests_total != 0.0 {
            (num_tests_passed / num_tests_total) * out_of
        } else {
            0.0
        };

        Ok(GradeResult {
            Requirement: req_name,
            Grade: format!("{:.2}/{:.2}", grade, out_of),
            Reason: format!("- {}/{} tests passing.", num_tests_passed, num_tests_total),
        })
    }
}

fn grade_unit_tests(
    req_name: String,
    out_of: f32,
    target_test: Vec<String>,
    target_class: Vec<String>,
    excluded_methods: Vec<String>,
) -> Result<GradeResult> {
    let child = Command::new(java_path()?)
        .args([
            "--class-path",
            classpath()?.as_str(),
            "org.pitest.mutationtest.commandline.MutationCoverageReport",
            "--reportDir",
            "test_reports",
            "--failWhenNoMutations",
            "true",
            "--targetClasses",
            target_class.join(",").as_str(),
            "--targetTests",
            target_test.join(",").as_str(),
            "--sourceDirs",
            SOURCE_DIR.to_str().unwrap(),
            "--timestampedReports",
            "false",
            "--outputFormats",
            "HTML,CSV",
            "--mutators",
            "STRONGER",
            "--excludedMethods",
            excluded_methods.join(",").as_str(),
        ])
        .output()
        .context("Failed to spawn javac process.")?;

    if child.status.success() {
        std::fs::create_dir_all("test_reports")?;
        let file = File::open(&ROOT_DIR.join("test_reports").join("mutations.csv"))
            .context("Could not read ./test_reports/mutations.csv file")?;
        let reader = BufReader::new(file);
        let mut diags = vec![];
        let mut not_killed = 0;
        for line in reader.lines() {
            let line = line?;
            let parse_result = parser::mutation_report_row(&line)
                .context("While parsing test_reports/mutations.csv");

            match parse_result {
                Ok(r) => {
                    if r.result != "KILLED" {
                        if r.test_method != "None" {
                            not_killed += 1;
                        }
                        diags.push(r);
                    }
                }
                Err(e) => {
                    anyhow::bail!(e);
                }
            };
        }
        let penalty = diags.len() as u32 * 4;
        println!("Ran mutation tests for {} -", target_test.join(", "));
        println!("{}", ExpandedDisplay::new(diags));
        println!("Problematic mutation test failures printed about.");

        Ok(GradeResult {
            Requirement: req_name,
            Grade: format!("{}/{}", (out_of as u32).saturating_sub(penalty), out_of),
            Reason: format!("-{} Penalty due to surviving muations", penalty),
        })
    } else {
        let output = format!(
            "{}{}",
            String::from_utf8(child.stderr)?,
            String::from_utf8(child.stdout)?
        );

        println!("{}", output);
        Ok(GradeResult {
            Requirement: req_name,
            Grade: format!("0/{}", out_of),
            Reason: String::from("Something went wrong while running mutation tests, skipping."),
        })
    }
}

#[fncmd::fncmd]
/// Run JUnit tests from a JUnit test class (source) file
pub fn main() -> Result<()> {
    let project = JavaProject::new()?;

    let req_1 = grade_by_tests(
        vec![String::from("DataStructures.WorkAheadQueueTest")],
        vec![
            String::from("DataStructures.WorkAheadQueueTest#testDequeue_int"),
            String::from("DataStructures.WorkAheadQueueTest#testFirst_int"),
            String::from("DataStructures.WorkAheadQueueTest#testFirstThreeNodes"),
            String::from("DataStructures.WorkAheadQueueTest#testFirstThreeElements"),
            String::from("DataStructures.WorkAheadQueueTest#testEnqueue"),
            String::from("DataStructures.WorkAheadQueueTest#testDequeue_0args"),
            String::from("DataStructures.WorkAheadQueueTest#testFirst_0args"),
            String::from("DataStructures.WorkAheadQueueTest#testIsEmpty"),
            String::from("DataStructures.WorkAheadQueueTest#testSize"),
            String::from("DataStructures.WorkAheadQueueTest#testDequeue_int_ECE"),
            String::from("DataStructures.WorkAheadQueueTest#testDequeue_int_IAE"),
            String::from("DataStructures.WorkAheadQueueTest#testDequeue_int2"),
        ],
        &project,
        50.0,
        "1".to_string(),
    )?;

    let req_2 = grade_docs(
        vec!["DataStructures.WorkAheadQueue"],
        &project,
        20,
        "2".into(),
    )?;

    println!(
        "{}",
        Table::new(vec![req_1, req_2]).with(tabled::Style::modern())
    );
    Ok(())
}
