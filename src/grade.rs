#![warn(missing_docs)]
#![warn(clippy::missing_docs_in_private_items)]

use std::{
    collections::HashSet,
    fmt::Display,
    fs::{
        self,
        File,
    },
    io::{
        BufRead,
        BufReader,
        Write,
    },
    process::Command,
};

use anyhow::{
    anyhow,
    Context,
    Result,
};
use async_openai::types::{
    ChatCompletionRequestMessage,
    Role,
};
use futures::{
    future::try_join_all,
    stream::FuturesUnordered,
};
use reqwest::{
    Error,
    Response,
};
#[allow(deprecated)]
use rhai::{
    Array,
    CustomType,
    Dynamic,
    EvalAltResult,
};
use serde::{
    Deserialize,
    Serialize,
};
use snailquote::unescape;
use tabled::{
    display::ExpandedDisplay,
    object::Rows,
    Alignment,
    Modify,
    Panel,
    TableIteratorExt,
    Tabled,
    Width,
};
use termdiff::{
    DrawDiff,
    SignsTheme,
};
use typed_builder::TypedBuilder;
use umm_derive::generate_rhai_variant;

use crate::{
    constants::{
        POSTGREST_CLIENT,
        PROMPT_TRUNCATE,
        ROOT_DIR,
        RUNTIME,
        SOURCE_DIR,
        SYSTEM_MESSAGE_INTRO,
        SYSTEM_MESSAGE_OUTRO,
    },
    java::{
        FileType,
        Project,
    },
    parsers::parser,
    util::{
        classpath,
        java_path,
    },
};
#[derive(Debug, Hash, PartialEq, Eq)]
/// A struct representing a line in a stack trace
pub struct LineRef {
    /// The line number
    pub line_number: u32,
    /// The file name
    pub file_name:   String,
}

#[derive(Clone, Default)]
/// A struct representing a grade
pub struct Grade {
    /// The actual grade recieved
    pub grade:  f64,
    /// The maximum grade possible
    pub out_of: f64,
}

impl Grade {
    /// Creates a new grade -
    /// * `grade` - The actual grade recieved
    /// * `out_of` - The maximum grade possible
    pub fn new(grade: f64, out_of: f64) -> Self {
        Self {
            grade,
            out_of,
        }
    }

    #[generate_rhai_variant(Impl, Fallible)]
    /// Creates a new grade from a string -
    /// * `grade_string` - A string in the format `grade/out_of`, eg. `10/20`
    pub fn grade_from_string(grade_string: String) -> Result<Grade> {
        let (grade, out_of) = grade_string.split_once('/').unwrap_or(("0", "0"));
        Ok(Grade::new(
            grade.parse::<f64>().context("Failed to parse grade")?,
            out_of.parse::<f64>().context("Failed to parse out of")?,
        ))
    }

    /// a getter for the grade
    pub fn grade(&mut self) -> f64 {
        self.grade
    }

    /// a getter for the out_of
    pub fn out_of(&mut self) -> f64 {
        self.out_of
    }

    /// a setter for the grade
    pub fn set_grade(mut self, grade: f64) -> Self {
        self.grade = grade;
        self
    }

    /// a setter for the out_of
    pub fn set_out_of(mut self, out_of: f64) -> Self {
        self.grade = out_of;
        self
    }
}

impl Display for Grade {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:.2}/{:.2}", self.grade, self.out_of)
    }
}

#[derive(Tabled, Clone, Default)]
/// A struct to store grading results and display them
pub struct GradeResult {
    #[tabled(rename = "Requirement")]
    /// * `requirement`: refers to Requirement ID
    requirement: String,
    #[tabled(rename = "Grade")]
    /// * `grade`: grade received for above Requirement
    grade:       Grade,
    #[tabled(rename = "Reason")]
    /// * `reason`: the reason for penalties applied, if any
    reason:      String,
    #[tabled(skip)]
    /// * `prompt`: the prompt for the AI TA
    prompt:      Option<Vec<ChatCompletionRequestMessage>>,
}

impl GradeResult {
    /// a getter for Requirement
    pub fn requirement(&mut self) -> String {
        self.requirement.clone()
    }

    /// a setter for Requirement
    pub fn set_requirement(mut self, requirement: String) -> Self {
        self.requirement = requirement;
        self
    }

    /// a getter for Reason
    pub fn reason(&mut self) -> String {
        self.reason.clone()
    }

    /// a setter for Reason
    pub fn set_reason(mut self, reason: String) -> Self {
        self.reason = reason;
        self
    }

    /// a getter for the self.grade.grade
    pub fn grade(&mut self) -> f64 {
        self.grade.grade()
    }

    /// a getter for the self.grade.out_of
    pub fn out_of(&mut self) -> f64 {
        self.grade.out_of()
    }

    /// a setter for the self.grade.grade
    pub fn set_grade(mut self, grade: f64) -> Self {
        self.grade = self.grade.set_grade(grade);
        self
    }

    /// a setter for the self.grade.out_of
    pub fn set_out_of(mut self, out_of: f64) -> Self {
        self.grade = self.grade.set_out_of(out_of);
        self
    }

    /// a getter for the prompt
    pub fn prompt(&mut self) -> Option<Vec<ChatCompletionRequestMessage>> {
        self.prompt.clone()
    }

    /// a setter for the prompt
    pub fn set_prompt(mut self, prompt: Option<Vec<ChatCompletionRequestMessage>>) -> Self {
        self.prompt = prompt;
        self
    }
}

#[derive(Tabled, Serialize, Deserialize, TypedBuilder, Clone, Debug)]
#[builder(field_defaults(setter(into)))]
#[builder(doc)]
/// A struct representing a javac diagnostic message
pub struct JavacDiagnostic {
    /// * `path`: path to the file diagnostic is referring to
    #[tabled(rename = "File")]
    path:        String,
    /// * `file_name`: name of the file the diagnostic is about
    #[tabled(skip)]
    file_name:   String,
    /// * `line_number`: line number
    #[tabled(rename = "Line")]
    line_number: u32,
    /// * `is_error`: boolean value, is true if error or false if the diagnostic
    ///   is a warning
    #[tabled(skip)]
    is_error:    bool,
    /// * `message`: the diagnostic message
    #[tabled(rename = "Message")]
    message:     String,
}

impl From<JavacDiagnostic> for LineRef {
    /// Converts a JavacDiagnostic to a LineRef
    fn from(val: JavacDiagnostic) -> Self {
        LineRef {
            file_name:   val.file_name,
            line_number: val.line_number,
        }
    }
}

#[derive(Tabled, Serialize, Deserialize, TypedBuilder, Clone)]
#[builder(field_defaults(setter(into)))]
#[builder(doc)]
/// A struct representing a PIT diagnostic message
pub struct MutationDiagnostic {
    /// * `mutator`: name of the mutator in question
    #[tabled(rename = "Mutation type")]
    mutator:          String,
    /// * `source_method`: name of the source method being mutated
    #[tabled(rename = "Source method mutated")]
    source_method:    String,
    /// * `line_number`: source line number where mutation occured
    #[tabled(rename = "Line no. of mutation")]
    line_number:      u32,
    /// * `test_method`: name of the test examined
    #[tabled(rename = "Test examined")]
    test_method:      String,
    /// * `result`: result of mutation testing
    #[tabled(rename = "Result")]
    result:           String,
    /// * `source_file_name`: name of the source file
    #[tabled(skip)]
    source_file_name: String,
    /// * `test_file_name`: name of the test file
    #[tabled(skip)]
    test_file_name:   String,
}

impl From<MutationDiagnostic> for LineRef {
    /// Converts a MutationDiagnostic to a LineRef
    fn from(val: MutationDiagnostic) -> Self {
        LineRef {
            file_name:   val.source_file_name,
            line_number: val.line_number,
        }
    }
}

/// Returns a ChatCompletionRequestMessage with the given line references that
/// include contextual lines of code from the source
///
/// * `line_refs`: a vector of LineRef objects
/// * `proj`: a Project object
/// * `start_offset`: the number of lines of code to include before the line
/// * `num_lines`: the number of lines of code to include after the line
pub fn get_source_context<T: Into<LineRef>>(
    line_refs: Vec<T>,
    proj: Project,
    start_offset: u32,
    num_lines: u32,
) -> Result<ChatCompletionRequestMessage> {
    let mut line_refs = line_refs
        .into_iter()
        .map(|x| x.into())
        .collect::<HashSet<LineRef>>()
        .into_iter()
        .collect::<Vec<LineRef>>();

    line_refs.sort_by(|lhs, rhs| {
        lhs.file_name
            .cmp(&rhs.file_name)
            .then(lhs.line_number.cmp(&rhs.line_number))
    });

    let mut context = Vec::new();
    context.push(
        "You cannot see all of the student's submission as you are an AI language model, with \
         limited context length. Here are some snippets of code the stacktrace indicates might be \
         relevant:
:\n"
        .to_string(),
    );
    let end_ticks = "\n```".to_string();

    for re in &line_refs {
        if let Ok(file) = proj.identify(&re.file_name) {
            let start = match file.kind() {
                FileType::Test => re.line_number.saturating_sub(num_lines) as usize,
                _ => re.line_number.saturating_sub(start_offset) as usize,
            };

            context.push(format!(
                "- Lines {} to {} from {} -\n```",
                start,
                start + num_lines as usize,
                re.file_name
            ));
            context.append(
                &mut file
                    .parser()
                    .code()
                    .lines()
                    .skip(start)
                    .filter(|line| !line.trim().is_empty())
                    .take(num_lines as usize)
                    .map(|x| x.to_string().replace("\\\\", "\\").replace("\\\"", "\""))
                    .collect::<Vec<String>>(),
            );
            context.push(end_ticks.clone());
        }
    }
    let mut context = context.join("\n");
    context.truncate(PROMPT_TRUNCATE);

    Ok(ChatCompletionRequestMessage {
        role:    Role::System,
        content: context,
        name:    Some(String::from("Student")),
    })
}

#[derive(Clone, Default)]
/// A struct representing arguements to grade_docs function
pub struct DocsGrader {
    /// * `project`: the project to grade
    pub project:  Project,
    /// * `files`: the files to grade
    pub files:    Array,
    /// * `out_of`: the total points for the requirement
    pub out_of:   f64,
    /// * `req_name`: the name of the requirement
    pub req_name: String,
    /// * `penalty`: the penalty to apply for each instance of a violation.
    ///   Optional, default is 3
    pub penalty:  f64,
}

impl DocsGrader {
    /// Getter for project
    pub fn project(&mut self) -> Project {
        self.project.clone()
    }

    /// Setter for project
    pub fn set_project(mut self, project: Project) -> Self {
        self.project = project;
        self
    }

    /// Getter for files
    pub fn files(&mut self) -> Array {
        self.files.clone()
    }

    /// Setter for files
    pub fn set_files(mut self, files: Array) -> Self {
        self.files = files;
        self
    }

    /// Getter for out_of
    pub fn out_of(&mut self) -> f64 {
        self.out_of
    }

    /// Setter for out_of
    pub fn set_out_of(mut self, out_of: f64) -> Self {
        self.out_of = out_of;
        self
    }

    /// Getter for req_name
    pub fn req_name(&mut self) -> String {
        self.req_name.clone()
    }

    /// Setter for req_name
    pub fn set_req_name(mut self, req_name: String) -> Self {
        self.req_name = req_name;
        self
    }

    /// Getter for penalty
    pub fn penalty(&mut self) -> f64 {
        self.penalty
    }

    /// Setter for penalty
    pub fn set_penalty(mut self, penalty: f64) -> Self {
        self.penalty = penalty;
        self
    }

    /// Grades documentation by using the -Xdoclint javac flag.
    /// Scans javac output for generated warnings and grades accordingly.
    #[generate_rhai_variant(Fallible)]
    pub fn grade_docs(self) -> Result<GradeResult> {
        let mut diags = vec![];
        let mut all_diags = vec![];
        let files: Vec<String> = self
            .files
            .iter()
            .map(|f| match f.clone().into_string() {
                Ok(n) => Ok(n),
                Err(e) => Err(anyhow!(
                    "files array has something that's not a string: {}",
                    e
                )),
            })
            .try_collect()?;
        let out_of = self.out_of;
        let mut outputs = vec![];
        for name in &files {
            let file = self.project.identify(name)?;
            let output = file.doc_check()?;
            outputs.push(output.clone());
            for line in output.lines() {
                let result = parser::parse_diag(line);
                match result {
                    Ok(res) => {
                        if file.file_name() == res.file_name {
                            diags.push(res.clone());
                        }
                        all_diags.push(res);
                    }
                    Err(_) => continue,
                }
            }
        }

        let penalty = diags.len() as f64 * self.penalty;
        let grade = if out_of - penalty > 0.0 {
            out_of - penalty
        } else {
            0.0
        };

        let num_diags = diags.len();
        eprintln!(
            "{}",
            diags
                .table()
                .with(Panel::header(format!(
                    "Check javadoc for {}",
                    files.join(", ")
                )))
                .with(Panel::footer(format!("-{penalty} due to {num_diags} nits")))
                .with(Modify::new(Rows::new(1..)).with(Width::wrap(24).keep_words()))
                .with(
                    Modify::new(Rows::first())
                        .with(Alignment::center())
                        .with(Alignment::center_vertical()),
                )
                .with(
                    Modify::new(Rows::last())
                        .with(Alignment::center())
                        .with(Alignment::center_vertical()),
                )
                .with(tabled::Style::modern())
        );

        let context = get_source_context(all_diags, self.project, 1, 3)?;

        let prompt = if num_diags > 0 {
            let mut outputs = outputs
                .iter()
                .map(|output| format!("```\n{output}\n```"))
                .collect::<Vec<String>>()
                .join("\n\n---\n\n");

            outputs.truncate(PROMPT_TRUNCATE);

            Some(vec![
                ChatCompletionRequestMessage {
                    role:    Role::System,
                    content: SYSTEM_MESSAGE_INTRO.to_string(),
                    name:    Some("Instructor".into()),
                },
                ChatCompletionRequestMessage {
                    role:    Role::User,
                    content: outputs,
                    name:    Some("Student".into()),
                },
                context,
                ChatCompletionRequestMessage {
                    role:    Role::System,
                    content: include_str!("prompts/javadoc.md").to_string(),
                    name:    Some("Instructor".into()),
                },
                ChatCompletionRequestMessage {
                    role:    Role::System,
                    content: SYSTEM_MESSAGE_OUTRO.to_string(),
                    name:    Some("Instructor".into()),
                },
            ])
        } else {
            None
        };
        Ok(GradeResult {
            requirement: self.req_name,
            grade: Grade::new(grade, out_of),
            reason: String::from("See above."),
            prompt,
        })
    }
}

#[derive(Clone, Default)]
/// Grades by running tests, and reports how many tests pass.
/// Final grade is the same percentage of maximum grade as the number of tests
/// passing.
pub struct ByUnitTestGrader {
    /// A list of test files to run.
    test_files:     Array,
    /// A list of test names that should be found. Grade returned is 0 if any
    /// are not found.
    expected_tests: Array,
    /// A reference to the project the test files belong to.
    project:        Project,
    /// Maximum possible grade.
    out_of:         f64,
    /// Display name for requirement to use while displaying grade result
    req_name:       String,
}

impl ByUnitTestGrader {
    /// Getter for test_files
    pub fn test_files(&mut self) -> Array {
        self.test_files.clone()
    }

    /// Setter for test_files
    pub fn set_test_files(mut self, test_files: Array) -> Self {
        self.test_files = test_files;
        self
    }

    /// Getter for expected_tests
    pub fn expected_tests(&mut self) -> Array {
        self.expected_tests.clone()
    }

    /// Setter for expected_tests
    pub fn set_expected_tests(mut self, expected_tests: Array) -> Self {
        self.expected_tests = expected_tests;
        self
    }

    /// Getter for project
    pub fn project(&mut self) -> Project {
        self.project.clone()
    }

    /// Setter for project
    pub fn set_project(mut self, project: Project) -> Self {
        self.project = project;
        self
    }

    /// Getter for out_of
    pub fn out_of(&mut self) -> f64 {
        self.out_of
    }

    /// Setter for out_of
    pub fn set_out_of(mut self, out_of: f64) -> Self {
        self.out_of = out_of;
        self
    }

    /// Getter for req_name
    pub fn req_name(&mut self) -> String {
        self.req_name.clone()
    }

    /// Setter for req_name
    pub fn set_req_name(mut self, req_name: String) -> Self {
        self.req_name = req_name;
        self
    }

    #[generate_rhai_variant(Fallible)]
    /// Grades by running tests, and reports how many tests pass.
    /// Final grade is the same percentage of maximum grade as the number of
    /// tests passing.
    pub fn grade_by_tests(self) -> Result<GradeResult> {
        let test_files = self.test_files;
        let expected_tests = self.expected_tests;
        let project = self.project.clone();
        let out_of = self.out_of;
        let req_name = self.req_name;

        let test_files: Vec<String> = test_files
            .iter()
            .map(|f| match f.clone().into_string() {
                Ok(n) => Ok(n),
                Err(e) => Err(anyhow!(
                    "test_files array has something that's not a string: {}",
                    e
                )),
            })
            .try_collect()?;

        let expected_tests: Vec<String> = expected_tests
            .iter()
            .map(|f| match f.clone().into_string() {
                Ok(n) => Ok(n),
                Err(e) => Err(anyhow!(
                    "expected_tests array has something that's not a string: {}",
                    e
                )),
            })
            .try_collect()?;

        let mut actual_tests = vec![];
        let mut expected_tests = expected_tests;
        let mut reasons = vec![];
        expected_tests.sort();

        for test_file in &test_files {
            let test_file = project.identify(test_file)?;

            actual_tests.append(&mut test_file.test_methods());
        }
        actual_tests.sort();

        if !expected_tests.is_empty() {
            for expected in &expected_tests {
                let n = expected.split_once('#').unwrap().1;
                if !actual_tests.contains(expected) {
                    reasons.push(format!("- {n} not found."));
                }
            }

            for actual in &actual_tests {
                let n = actual.split_once('#').unwrap().1;
                if !expected_tests.contains(actual) {
                    reasons.push(format!("- Unexpected test called {n}"));
                }
            }
        }

        if !reasons.is_empty() {
            reasons.push("Tests will not be run until above is fixed.".into());
            let reasons = reasons.join("\n");
            let messages = vec![
                ChatCompletionRequestMessage {
                    role:    Role::System,
                    content: SYSTEM_MESSAGE_INTRO.to_string(),
                    name:    Some("Instructor".into()),
                },
                ChatCompletionRequestMessage {
                    role:    Role::System,
                    content: self.project.describe(),
                    name:    Some("Instructor".into()),
                },
                ChatCompletionRequestMessage {
                    role:    Role::User,
                    content: reasons.clone(),
                    name:    Some("Student".into()),
                },
                ChatCompletionRequestMessage {
                    role:    Role::System,
                    content: SYSTEM_MESSAGE_OUTRO.to_string(),
                    name:    Some("Instructor".into()),
                },
            ];
            Ok(GradeResult {
                requirement: req_name,
                grade:       Grade::new(0.0, out_of),
                reason:      reasons,
                prompt:      Some(messages),
            })
        } else {
            let mut num_tests_passed = 0.0;
            let mut num_tests_total = 0.0;
            let mut messages = vec![];

            for test_file in test_files {
                let res = match project.identify(test_file.as_str())?.test(Vec::new()) {
                    Ok(res) => [
                        String::from_utf8(res.stderr)?,
                        String::from_utf8(res.stdout)?,
                    ]
                    .concat(),
                    Err(e) => {
                        let errors = match e.source() {
                            Some(e) => unescape(&format!("{:#?}", e)).unwrap(),
                            None => String::new(),
                        };
                        let mut all_diags = vec![];

                        for line in errors.lines() {
                            if let Ok(diag) = parser::parse_diag(line) {
                                all_diags.push(diag);
                            }
                        }
                        let context = get_source_context(all_diags, self.project.clone(), 3, 6)?;

                        let messages = vec![
                            ChatCompletionRequestMessage {
                                role:    Role::System,
                                content: SYSTEM_MESSAGE_INTRO.to_string(),
                                name:    Some("Instructor".into()),
                            },
                            ChatCompletionRequestMessage {
                                role:    Role::System,
                                content: self.project.describe(),
                                name:    Some("Instructor".into()),
                            },
                            ChatCompletionRequestMessage {
                                role:    Role::User,
                                content: format!("```\n{:#?}\n```", e),
                                name:    Some("Student".into()),
                            },
                            context,
                            // ChatCompletionRequestMessage {
                            //     role:    Role::System,
                            //     content: include_str!("prompts/unit_testing.md").to_string(),
                            //     name:    Some("Instructor".into()),
                            // },
                            ChatCompletionRequestMessage {
                                role:    Role::System,
                                content: SYSTEM_MESSAGE_OUTRO.to_string(),
                                name:    Some("Instructor".into()),
                            },
                        ];
                        return Ok(GradeResult {
                            requirement: req_name,
                            grade:       Grade::new(0.0, out_of),
                            reason:      "Error running tests.".to_string(),
                            prompt:      Some(messages),
                        });
                    }
                };
                let mut current_tests_passed = 0.0;
                let mut current_tests_total = 0.0;

                for line in res.lines() {
                    let parse_result =
                        parser::num_tests_passed(line).context("While parsing Junit summary table");
                    if let Ok(n) = parse_result {
                        current_tests_passed = n as f64;
                    }
                    let parse_result =
                        parser::num_tests_found(line).context("While parsing Junit summary table");
                    if let Ok(n) = parse_result {
                        current_tests_total = n as f64;
                    }
                }

                if current_tests_passed < current_tests_total {
                    let user_message = res.clone();
                    let mut all_diags = vec![];
                    let mut new_user_message = Vec::new();

                    for line in user_message.lines() {
                        if line.contains("MethodSource") {
                            continue;
                        }

                        if line.contains("Test run finished after") {
                            break;
                        }

                        if let Ok(diag) = parser::junit_stacktrace_line_ref(line) {
                            if project.identify(&diag.file_name).is_ok() {
                                new_user_message.push(
                                    line.replace("\\\\", "\\").replace("\\\"", "\"").to_string(),
                                );
                            }
                            all_diags.push(diag);
                        } else {
                            new_user_message
                                .push(line.replace("\\\\", "\\").replace("\\\"", "\"").to_string());
                        }
                    }

                    let context = get_source_context(all_diags, self.project.clone(), 3, 6)?;

                    let mut user_message = new_user_message.join("\n");
                    user_message.truncate(PROMPT_TRUNCATE);
                    user_message = format!("```\n{user_message}\n```");

                    messages = vec![
                        ChatCompletionRequestMessage {
                            role:    Role::System,
                            content: SYSTEM_MESSAGE_INTRO.to_string(),
                            name:    Some("Instructor".into()),
                        },
                        ChatCompletionRequestMessage {
                            role:    Role::System,
                            content: self.project.describe(),
                            name:    Some("Instructor".into()),
                        },
                        ChatCompletionRequestMessage {
                            role:    Role::User,
                            content: user_message,
                            name:    Some("Student".into()),
                        },
                        context,
                        ChatCompletionRequestMessage {
                            role:    Role::System,
                            content: SYSTEM_MESSAGE_OUTRO.to_string(),
                            name:    Some("Instructor".into()),
                        },
                    ];
                }

                num_tests_passed += current_tests_passed;
                num_tests_total += current_tests_total;
            }
            let grade = if num_tests_total != 0.0 {
                (num_tests_passed / num_tests_total) * out_of
            } else {
                0.0
            };

            Ok(GradeResult {
                requirement: req_name,
                grade:       Grade::new(grade, out_of),
                reason:      format!("- {num_tests_passed}/{num_tests_total} tests passing."),
                prompt:      Some(messages),
            })
        }
    }
}

#[derive(Clone, Default)]
/// Runs mutation tests using ![Pitest](http://pitest.org/) to grade unit tests written by students.
pub struct UnitTestGrader {
    /// Name of the requirement.
    pub req_name:         String,
    /// Maximum possible grade.
    pub out_of:           f64,
    /// List of test classes to run.
    pub target_test:      Array,
    /// List of classes to mutate.
    pub target_class:     Array,
    /// List of methods to exclude from mutation.
    pub excluded_methods: Array,
    /// List of classes to avoid mutating.
    pub avoid_calls_to:   Array,
}

impl UnitTestGrader {
    /// A getter for the name of the requirement.
    pub fn get_req_name(&mut self) -> String {
        self.req_name.clone()
    }

    /// A getter for the maximum possible grade.
    pub fn get_out_of(&mut self) -> f64 {
        self.out_of
    }

    /// A getter for the list of test classes to run.
    pub fn get_target_test(&mut self) -> Array {
        self.target_test.clone()
    }

    /// A getter for the list of classes to mutate.
    pub fn get_target_class(&mut self) -> Array {
        self.target_class.clone()
    }

    /// A getter for the list of methods to exclude from mutation.
    pub fn get_excluded_methods(&mut self) -> Array {
        self.excluded_methods.clone()
    }

    /// A getter for the list of classes to avoid mutating.
    pub fn get_avoid_calls_to(&mut self) -> Array {
        self.avoid_calls_to.clone()
    }

    /// A setter for the name of the requirement.
    pub fn set_req_name(mut self, req_name: String) -> Self {
        self.req_name = req_name;
        self
    }

    /// A setter for the maximum possible grade.
    pub fn set_out_of(mut self, out_of: f64) -> Self {
        self.out_of = out_of;
        self
    }

    /// A setter for the list of test classes to run.
    pub fn set_target_test(mut self, target_test: Array) -> Self {
        self.target_test = target_test;
        self
    }

    /// A setter for the list of classes to mutate.
    pub fn set_target_class(mut self, target_class: Array) -> Self {
        self.target_class = target_class;
        self
    }

    /// A setter for the list of methods to exclude from mutation.
    pub fn set_excluded_methods(mut self, excluded_methods: Array) -> Self {
        self.excluded_methods = excluded_methods;
        self
    }

    /// A setter for the list of classes to avoid mutating.
    pub fn set_avoid_calls_to(mut self, avoid_calls_to: Array) -> Self {
        self.avoid_calls_to = avoid_calls_to;
        self
    }

    #[generate_rhai_variant(Fallible)]
    /// Runs mutation tests using ![Pitest](http://pitest.org/) to grade unit tests written by students.
    pub fn grade_unit_tests(&mut self) -> Result<GradeResult> {
        let req_name = self.get_req_name();
        let out_of = self.get_out_of();
        let target_test = self.get_target_test();
        let target_class = self.get_target_class();
        let excluded_methods = self.get_excluded_methods();
        let avoid_calls_to = self.get_avoid_calls_to();
        let project = Project::new()?;

        eprintln!("Running Mutation tests -");
        let target_test: Vec<String> = target_test
            .iter()
            .map(|f| match f.clone().into_string() {
                Ok(n) => Ok(n),
                Err(e) => Err(anyhow!(
                    "target_test array has something that's not a string: {}",
                    e
                )),
            })
            .try_collect()?;
        let target_class: Vec<String> = target_class
            .iter()
            .map(|f| match f.clone().into_string() {
                Ok(n) => Ok(n),
                Err(e) => Err(anyhow!(
                    "target_class array has something that's not a string: {}",
                    e
                )),
            })
            .try_collect()?;
        let excluded_methods: Vec<String> = excluded_methods
            .iter()
            .map(|f| match f.clone().into_string() {
                Ok(n) => Ok(n),
                Err(e) => Err(anyhow!(
                    "excluded_methods array has something that's not a string: {}",
                    e
                )),
            })
            .try_collect()?;
        let avoid_calls_to: Vec<String> = avoid_calls_to
            .iter()
            .map(|f| match f.clone().into_string() {
                Ok(n) => Ok(n),
                Err(e) => Err(anyhow!(
                    "avoid_calls_to array has something that's not a string: {}",
                    e
                )),
            })
            .try_collect()?;

        let child = Command::new(java_path()?)
            .args([
                "--class-path",
                classpath()?.as_str(),
                "org.pitest.mutationtest.commandline.MutationCoverageReport",
                "--reportDir",
                "test_reports",
                "--failWhenNoMutations",
                "true",
                "--threads",
                "4",
                "--targetClasses",
                target_class.join(",").as_str(),
                "--targetTests",
                target_test.join(",").as_str(),
                "--sourceDirs",
                vec![
                    SOURCE_DIR.to_str().unwrap_or("."),
                    ROOT_DIR.to_str().unwrap_or("."),
                ]
                .join(",")
                .as_str(),
                "--timestampedReports",
                "false",
                "--outputFormats",
                "HTML,CSV",
                "--mutators",
                "STRONGER",
                "--excludedMethods",
                excluded_methods.join(",").as_str(),
                "--avoidCallsTo",
                avoid_calls_to.join(",").as_str(),
            ])
            .output()
            .context("Failed to spawn javac process.")?;

        if child.status.success() {
            std::fs::create_dir_all("test_reports")?;
            let file = File::open(ROOT_DIR.join("test_reports").join("mutations.csv"))
                .context("Could not read ./test_reports/mutations.csv file".to_string())?;
            let reader = BufReader::new(file);
            let mut diags = vec![];

            for line in reader.lines() {
                let line = line?;
                let parse_result = parser::mutation_report_row(&line)
                    .context("While parsing test_reports/mutations.csv");

                match parse_result {
                    Ok(r) => {
                        if r.result == "SURVIVED" {
                            diags.push(r);
                        }
                    }
                    Err(e) => {
                        anyhow::bail!(e);
                    }
                };
            }
            let penalty = diags.len() as u32 * 4;
            eprintln!("Ran mutation tests for {} -", target_test.join(", "));
            let num_diags = diags.len();
            eprintln!("Problematic mutation test failures printed about.");

            let prompt = if num_diags > 0 {
                let context = get_source_context(diags.clone(), project.clone(), 3, 6)?;

                let mut feedback = ExpandedDisplay::new(diags).to_string();
                eprintln!("{feedback}");

                feedback.truncate(PROMPT_TRUNCATE);

                Some(vec![
                    ChatCompletionRequestMessage {
                        role:    Role::System,
                        content: SYSTEM_MESSAGE_INTRO.to_string(),
                        name:    Some("Instructor".into()),
                    },
                    ChatCompletionRequestMessage {
                        role:    Role::System,
                        content: project.describe(),
                        name:    Some("Instructor".into()),
                    },
                    ChatCompletionRequestMessage {
                        role:    Role::User,
                        content: feedback,
                        name:    Some("Student".into()),
                    },
                    context,
                    ChatCompletionRequestMessage {
                        role:    Role::System,
                        content: format!(
                            include_str!("prompts/mutation_testing.md"),
                            test = target_test.join(", "),
                            class = target_class.join(", ")
                        ),
                        name:    Some("Instructor".into()),
                    },
                    ChatCompletionRequestMessage {
                        role:    Role::System,
                        content: SYSTEM_MESSAGE_OUTRO.to_string(),
                        name:    Some("Instructor".into()),
                    },
                ])
            } else {
                None
            };

            Ok(GradeResult {
                requirement: req_name,
                grade: Grade::new((out_of as u32).saturating_sub(penalty).into(), out_of),
                reason: format!("-{penalty} Penalty due to surviving muations"),
                prompt,
            })
        } else {
            let mut output = [
                String::from_utf8(child.stderr)?,
                String::from_utf8(child.stdout)?,
            ]
            .concat();
            eprintln!("{output}");
            output.truncate(PROMPT_TRUNCATE);

            let prompt = if !output.is_empty() {
                Some(vec![
                    ChatCompletionRequestMessage {
                        role:    Role::System,
                        content: SYSTEM_MESSAGE_INTRO.to_string(),
                        name:    Some("Instructor".into()),
                    },
                    ChatCompletionRequestMessage {
                        role:    Role::System,
                        content: project.describe(),
                        name:    Some("Instructor".into()),
                    },
                    ChatCompletionRequestMessage {
                        role:    Role::User,
                        content: output,
                        name:    Some("Student".into()),
                    },
                    ChatCompletionRequestMessage {
                        role:    Role::System,
                        content: format!(
                            include_str!("prompts/mutation_testing_2.md"),
                            test = target_test.join(", "),
                            class = target_class.join(", ")
                        ),
                        name:    Some("Instructor".into()),
                    },
                    ChatCompletionRequestMessage {
                        role:    Role::System,
                        content: SYSTEM_MESSAGE_OUTRO.to_string(),
                        name:    Some("Instructor".into()),
                    },
                ])
            } else {
                None
            };
            Ok(GradeResult {
                requirement: req_name,
                grade: Grade::new(0.0, out_of),
                reason: String::from(
                    "Something went wrong while running mutation tests, skipping.",
                ),
                prompt,
            })
        }
    }
}

#[derive(Clone, Default)]
/// Grades using hidden tests. Test file is downloaded, ran, and then cleaned up
/// before returning.
pub struct ByHiddenTestGrader {
    /// URL to download test source from.
    pub url:             String,
    /// name of hidden test class.
    pub test_class_name: String,
    /// points to give if all tests pass.
    pub out_of:          f64,
    /// name of requirement.
    pub req_name:        String,
}

impl ByHiddenTestGrader {
    /// gets the `url` field.
    pub fn url(&mut self) -> String {
        self.url.clone()
    }

    /// sets the `url` field.
    pub fn set_url(mut self, url: String) -> Self {
        self.url = url;
        self
    }

    /// gets the `test_class_name` field
    pub fn test_class_name(&mut self) -> String {
        self.test_class_name.clone()
    }

    /// sets the `test_class_name` field
    pub fn set_test_class_name(mut self, test_class_name: String) -> Self {
        self.test_class_name = test_class_name;
        self
    }

    /// gets the `out_of` field
    pub fn out_of(&mut self) -> f64 {
        self.out_of
    }

    /// sets the `out_of` field
    pub fn set_out_of(mut self, out_of: f64) -> Self {
        self.out_of = out_of;
        self
    }

    /// gets the `req_name` field
    pub fn req_name(&mut self) -> String {
        self.req_name.clone()
    }

    /// sets the `req_name` field
    pub fn set_req_name(mut self, req_name: String) -> Self {
        self.req_name = req_name;
        self
    }

    #[generate_rhai_variant(Fallible)]
    /// Grades using hidden tests. Test file is downloaded, ran, and then
    /// cleaned up before returning.
    pub fn grade_by_hidden_tests(&mut self) -> Result<GradeResult> {
        let url = self.url();
        let test_class_name = self.test_class_name();
        let out_of = self.out_of();
        let req_name = self.req_name();

        let test_source = reqwest::blocking::get(&url)
            .context(format!("Failed to download {url}"))?
            .bytes()
            .context(format!("Failed to get response as bytes: {url}"))?;

        let path = ROOT_DIR.join(format!("{test_class_name}.java"));
        let mut file = File::create(&path)?;
        file.write_all(&test_source)?;

        let project = match Project::new() {
            Ok(a) => a,
            Err(e) => {
                std::fs::remove_file(&path)?;
                return Err(e);
            }
        };

        let grader = ByUnitTestGrader {
            test_files: vec![Dynamic::from(test_class_name)],
            expected_tests: Array::new(),
            project,
            out_of,
            req_name,
        };

        let out = match grader.grade_by_tests() {
            Ok(o) => o,
            Err(e) => {
                std::fs::remove_file(&path)?;
                return Err(e);
            }
        };

        std::fs::remove_file(&path)?;
        Ok(out)
    }
}

/// Print grade result
///
/// * `results`: array of GradeResults to print in a table.
pub fn show_result(results: Array) {
    let results: Vec<GradeResult> = results
        .iter()
        .map(|f| f.clone().cast::<GradeResult>())
        .collect();

    let (grade, out_of) = results.iter().fold((0f64, 0f64), |acc, r| {
        (acc.0 + r.grade.grade, acc.1 + r.grade.out_of)
    });
    eprintln!(
        "{}",
        results
            .table()
            .with(Panel::header("Grading Overview"))
            .with(Panel::footer(format!("Total: {grade:.2}/{out_of:.2}")))
            .with(Modify::new(Rows::new(1..)).with(Width::wrap(24).keep_words()))
            .with(
                Modify::new(Rows::first())
                    .with(Alignment::center())
                    .with(Alignment::center_vertical()),
            )
            .with(
                Modify::new(Rows::last())
                    .with(Alignment::center())
                    .with(Alignment::center_vertical()),
            )
            .with(tabled::Style::modern())
    );
}

#[derive(Clone, Default)]
/// A grader that grades by diffing an `expected` string with an `actual`
/// string. Any difference results in a `0` grade.
pub struct DiffGrader {
    /// name of requirement
    pub req_name: String,
    /// points to give if all tests pass
    pub out_of:   f64,
    /// the expected output
    pub expected: String,
    /// the actual output
    pub actual:   String,
}

impl DiffGrader {
    /// creates a new DiffGrader
    pub fn new() -> Self {
        Self::default()
    }

    /// gets the `req_name` field
    pub fn req_name(&mut self) -> String {
        self.req_name.clone()
    }

    /// sets the `req_name` field
    pub fn set_req_name(mut self, req_name: String) -> Self {
        self.req_name = req_name;
        self
    }

    /// gets the `out_of` field
    pub fn out_of(&mut self) -> f64 {
        self.out_of
    }

    /// sets the `out_of` field
    pub fn set_out_of(mut self, out_of: f64) -> Self {
        self.out_of = out_of;
        self
    }

    /// gets the `expected` field
    pub fn expected(&mut self) -> String {
        self.expected.clone()
    }

    /// sets the `expected` field
    pub fn set_expected(mut self, expected: String) -> Self {
        self.expected = expected;
        self
    }

    /// gets the `actual` field
    pub fn actual(&mut self) -> String {
        self.actual.clone()
    }

    /// sets the `actual` field
    pub fn set_actual(mut self, actual: String) -> Self {
        self.actual = actual;
        self
    }

    #[generate_rhai_variant(Fallible)]
    /// Grades by diffing the `expected` and `actual` strings.
    pub fn grade_by_diff(&mut self) -> Result<GradeResult> {
        let theme = SignsTheme::default();
        eprintln!("{}", DrawDiff::new(&self.expected, &self.actual, &theme));
        Ok(GradeResult::default()) // TODO: implement
    }
}

/// Schema for `prompts` table
#[derive(Serialize, Debug)]
pub struct PromptRow {
    /// UUID of data entry
    id:               String,
    /// ChatGPT message prompt
    messages:         Option<Vec<ChatCompletionRequestMessage>>,
    /// Name of the autograder requirement
    requirement_name: String,
    /// Reasons for penalty
    reason:           String,
    /// Grade/out_of as a string
    grade:            String,
    /// Status of prompt response generation - not_started, started, compeleted
    status:           String,
}

#[generate_rhai_variant(Fallible)]
/// Generates a FEEDBACK file after prompting ChatGPT for feedback.
pub fn generate_feedback(results: Array) -> Result<()> {
    let rt = RUNTIME.handle().clone();
    let mut handles = vec![];
    let mut names = vec![];
    let mut ids = vec![];

    for res in results.iter().map(|f| f.clone().cast::<GradeResult>()) {
        let mut res = res.clone();

        if res.grade.grade < res.grade.out_of {
            let id = uuid::Uuid::new_v4().to_string();
            let body = PromptRow {
                id:               id.clone(),
                messages:         res.prompt(),
                requirement_name: res.requirement(),
                reason:           res.reason(),
                grade:            res.grade.to_string(),
                status:           "not_started".into(),
            };

            let messages = serde_json::to_string(&body)?;

            names.push(res.requirement());
            ids.push(id);
            handles.push(rt.spawn(async {
                POSTGREST_CLIENT
                    .from("prompts")
                    .insert(messages)
                    .execute()
                    .await
            }));
        }
    }

    if !handles.is_empty() {
        let handles = FuturesUnordered::from_iter(handles);
        rt.block_on(async { try_join_all(handles).await })?
            .into_iter()
            .collect::<Result<Vec<Response>, Error>>()?;

        let mut feedback = vec![];
        feedback.push("## Understanding Your Autograder Results\n".to_string());

        for (name, id) in names.into_iter().zip(ids.into_iter()) {
            feedback.push(format!(
                "- For explanation and feedback on `{name}` (refer rubric), please \
                 see this link - https://feedback.dhruvdh.com/{id}",
            ));
        }

        let feedback = feedback.join("\n");
        fs::write("FEEDBACK", &feedback).context("Something went wrong writing FEEDBACK file.")?;
        eprintln!("{}", &feedback);
    } else {
        fs::write(
            "FEEDBACK",
            "Feedback cannot currently be generated for submissions without penalty.",
        )
        .context("Something went wrong writing FEEDBACK file.")?;
    }

    Ok(())
}

// Allowed because CustomType is volatile, not deprecated
#[allow(deprecated)]
/// Allows registering custom types with Rhai.
impl CustomType for Grade {
    fn build(mut builder: rhai::TypeBuilder<Self>) {
        builder
            .with_name("Grade")
            .with_fn("grade", Self::grade)
            .with_fn("grade", Self::set_grade)
            .with_fn("out_of", Self::out_of)
            .with_fn("out_of", Self::set_out_of)
            .with_fn("new_grade", Self::new)
            .with_fn("from_string", Self::grade_from_string_script)
            .with_fn("to_string", Self::to_string);
    }
}

// Allowed because CustomType is volatile, not deprecated
#[allow(deprecated)]
/// Allows registering custom types with Rhai.
impl CustomType for GradeResult {
    fn build(mut builder: rhai::TypeBuilder<Self>) {
        builder
            .with_name("GradeResult")
            .with_fn("requirement", Self::requirement)
            .with_fn("requirement", Self::set_requirement)
            .with_fn("grade", Self::grade)
            .with_fn("grade", Self::set_grade)
            .with_fn("out_of", Self::out_of)
            .with_fn("out_of", Self::set_out_of)
            .with_fn("reason", Self::reason)
            .with_fn("reason", Self::set_reason)
            .with_fn("new_grade_result", Self::default);
    }
}

// Allowed because CustomType is not deprecated, just volatile
#[allow(deprecated)]
/// Allows registering custom types with Rhai
impl CustomType for DocsGrader {
    /// Builds a custom type to be registered with Rhai
    fn build(mut builder: rhai::TypeBuilder<Self>) {
        builder
            .with_name("DocsGrader")
            .with_fn("req_name", Self::req_name)
            .with_fn("req_name", Self::set_req_name)
            .with_fn("project", Self::project)
            .with_fn("project", Self::set_project)
            .with_fn("files", Self::files)
            .with_fn("files", Self::set_files)
            .with_fn("out_of", Self::out_of)
            .with_fn("out_of", Self::set_out_of)
            .with_fn("penalty", Self::penalty)
            .with_fn("penalty", Self::set_penalty)
            .with_fn("new_docs_grader", Self::default)
            .with_fn("run", Self::grade_docs_script);
    }
}

// Allowed because CustomType is not deprecated, just volatile
#[allow(deprecated)]
/// Allows registering custom types with Rhai
impl CustomType for ByUnitTestGrader {
    /// Builds a custom type to be registered with Rhai
    fn build(mut builder: rhai::TypeBuilder<Self>) {
        builder
            .with_name("ByUnitTestGrader")
            .with_fn("test_files", Self::test_files)
            .with_fn("test_files", Self::set_test_files)
            .with_fn("project", Self::project)
            .with_fn("project", Self::set_project)
            .with_fn("expected_tests", Self::expected_tests)
            .with_fn("expected_tests", Self::set_expected_tests)
            .with_fn("out_of", Self::out_of)
            .with_fn("out_of", Self::set_out_of)
            .with_fn("req_name", Self::req_name)
            .with_fn("req_name", Self::set_req_name)
            .with_fn("new_by_unit_test_grader", Self::default)
            .with_fn("run", Self::grade_by_tests_script);
    }
}

// Allowed because CustomType is not deprecated, just volatile
#[allow(deprecated)]
/// Allows registering custom types with Rhai
impl CustomType for UnitTestGrader {
    /// Builds a custom type to be registered with Rhai
    fn build(mut builder: rhai::TypeBuilder<Self>) {
        builder
            .with_name("UnitTestGrader")
            .with_fn("req_name", Self::get_req_name)
            .with_fn("req_name", Self::set_req_name)
            .with_fn("out_of", Self::get_out_of)
            .with_fn("out_of", Self::set_out_of)
            .with_fn("target_test", Self::get_target_test)
            .with_fn("target_test", Self::set_target_test)
            .with_fn("target_class", Self::get_target_class)
            .with_fn("target_class", Self::set_target_class)
            .with_fn("excluded_methods", Self::get_excluded_methods)
            .with_fn("excluded_methods", Self::set_excluded_methods)
            .with_fn("avoid_calls_to", Self::get_avoid_calls_to)
            .with_fn("avoid_calls_to", Self::set_avoid_calls_to)
            .with_fn("new_unit_test_grader", Self::default)
            .with_fn("run", Self::grade_unit_tests_script);
    }
}

// Allowed because CustomType is not deprecated, just volatile
#[allow(deprecated)]
/// Allows registering custom types with Rhai.
impl CustomType for ByHiddenTestGrader {
    /// Builds a custom type to be registered with Rhai.
    fn build(mut builder: rhai::TypeBuilder<Self>) {
        builder
            .with_name("ByHiddenTestGrader")
            .with_fn("url", Self::url)
            .with_fn("url", Self::set_url)
            .with_fn("test_class_name", Self::test_class_name)
            .with_fn("test_class_name", Self::set_test_class_name)
            .with_fn("out_of", Self::out_of)
            .with_fn("out_of", Self::set_out_of)
            .with_fn("req_name", Self::req_name)
            .with_fn("req_name", Self::set_req_name)
            .with_fn("new_by_hidden_test_grader", Self::default)
            .with_fn("run", Self::grade_by_hidden_tests_script);
    }
}

// Allowed because CustomType is not deprecated, just volatile
#[allow(deprecated)]
/// Allows registering custom types with Rhai.
impl CustomType for DiffGrader {
    /// Builds a custom type to be registered with Rhai.
    fn build(mut builder: rhai::TypeBuilder<Self>) {
        builder
            .with_name("DiffGrader")
            .with_fn("req_name", Self::req_name)
            .with_fn("req_name", Self::set_req_name)
            .with_fn("out_of", Self::out_of)
            .with_fn("out_of", Self::set_out_of)
            .with_fn("expected", Self::expected)
            .with_fn("expected", Self::set_expected)
            .with_fn("actual", Self::actual)
            .with_fn("actual", Self::set_actual)
            .with_fn("new_diff_grader", Self::default)
            .with_fn("run", Self::grade_by_diff_script);
    }
}
