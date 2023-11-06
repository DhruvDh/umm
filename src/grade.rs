#![warn(missing_docs)]
#![warn(clippy::missing_docs_in_private_items)]

use std::{
    collections::HashSet,
    fmt::Display,
    fs,
    io::{
        BufRead,
        BufReader,
        Write,
    },
    ops::RangeInclusive,
    process::Command,
};

use anyhow::{
    anyhow,
    ensure,
    Context,
    Result,
};
use async_openai::types::{
    ChatCompletionRequestMessage,
    CreateChatCompletionResponse,
    Role,
};
use colored::Colorize;
use futures::{
    future::try_join_all,
    stream::FuturesUnordered,
};
use itertools::Itertools;
use reqwest::{
    Error,
    Response,
};
use rhai::FnPtr;
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
use similar::{
    utils::diff_unicode_words,
    Algorithm,
    ChangeTag,
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
use typed_builder::TypedBuilder;
use umm_derive::generate_rhai_variant;

use crate::{
    constants::{
        JAVA_TS_LANG,
        METHOD_CALL_QUERY,
        POSTGREST_CLIENT,
        PROMPT_TRUNCATE,
        RETRIEVAL_MESSAGE_INTRO,
        ROOT_DIR,
        RUNTIME,
        SCRIPT_AST,
        SOURCE_DIR,
        SYSTEM_MESSAGE,
        USE_ACTIVE_RETRIEVAL,
    },
    create_engine,
    java::{
        File,
        FileType,
        JavaFileError,
        Parser,
        Project,
    },
    parsers::parser,
    util::{
        classpath,
        java_path,
    },
    Dict,
};
#[derive(Debug, Hash, PartialEq, Eq)]
/// A struct representing a line in a stack trace
pub struct LineRef {
    /// The line number
    pub line_number: usize,
    /// The file name
    pub file_name:   String,
}

impl LineRef {
    /// Returns the file name
    pub fn file_name(&self) -> &str {
        self.file_name.as_ref()
    }
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

impl JavacDiagnostic {
    /// Returns the file name
    pub fn file_name(&self) -> &str {
        self.file_name.as_ref()
    }
}

impl From<JavacDiagnostic> for LineRef {
    /// Converts a JavacDiagnostic to a LineRef
    fn from(val: JavacDiagnostic) -> Self {
        LineRef {
            file_name:   val.file_name,
            line_number: val.line_number as usize,
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
            line_number: val.line_number as usize,
        }
    }
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
/// `RetrievalFunctionCallParams` is a struct that holds the parameters for a
/// retrieval function call.
struct RetrievalFunctionCallParams {
    /// A string that holds the name of the class.
    class_name:  String,
    ///  A string that holds the name of the method.
    method_name: String,
}

#[derive(Serialize, Deserialize, Debug)]
/// `RetrievalFunctionCallParamsArray` is a struct that holds an array of
/// `RetrievalFunctionCallParams`.
struct RetrievalFunctionCallParamsArray {
    /// A vector of `RetrievalFunctionCallParams`.
    params: Vec<RetrievalFunctionCallParams>,
}

/// Retrieves the active context for a retrieval operation.
///
/// This function takes a reference to a `Project` and an optional `String` as
/// additional context. It ensures that the additional context is provided when
/// using active retrieval. It then prepares a series of
/// `ChatCompletionRequestMessage` and serializes them into a JSON string.
///
/// # Arguments
///
/// * `proj` - A reference to a `Project`.
/// * `additional_context` - An optional `String` that provides additional
///   context for the retrieval operation.
///
/// # Returns
///
/// * `Result<ChatCompletionRequestMessage>` - A `Result` that contains a
///   `ChatCompletionRequestMessage` if the operation was successful, or an
///   `Err` if it was not.
pub fn get_active_retrieval_context(
    proj: &Project,
    active_retrieval_context: Option<String>,
) -> Result<ChatCompletionRequestMessage> {
    ensure!(
        active_retrieval_context.is_some(),
        "Additional context must be provided when using active retrieval."
    );

    print!("Trying to decide what to share with AI for feedback...");
    let mut messages = Vec::new();

    messages.push(ChatCompletionRequestMessage {
        role:          Role::System,
        content:       Some(RETRIEVAL_MESSAGE_INTRO.to_string()),
        name:          Some(String::from("Instructor")),
        function_call: None,
    });
    messages.push(ChatCompletionRequestMessage {
        role:          Role::User,
        content:       Some(format!(
            "Here is the output (stdout and stderr) from running the auto-grader on my \
             submission:\n```\n{}\n```",
            active_retrieval_context.unwrap()
        )),
        name:          Some(String::from("Student")),
        function_call: None,
    });
    messages.push(ChatCompletionRequestMessage {
        role:          Role::System,
        content:       Some(format!(
            include_str!("prompts/retrieval_system_message_outro.md"),
            JAVA_FILE_NAMES = proj.files().iter().map(File::proper_name).join(", "),
            SYNTHESIZED_OUTLINE = proj.describe(),
        )),
        name:          Some(String::from("Instructor")),
        function_call: None,
    });
    let messages = serde_json::to_string(&messages).expect("Failed to serialize messages array");

    let client = reqwest::blocking::Client::new();
    let response: CreateChatCompletionResponse = client
        .post("https://umm-feedback-openai-func.deno.dev/")
        .body(messages)
        .send()?
        .json()?;
    let response = response.choices[0].message.clone();
    println!(" done!");
    ensure!(
        response.function_call.is_some(),
        "No function call found in response."
    );
    let function_call_args: RetrievalFunctionCallParamsArray =
        serde_json::from_str(response.function_call.unwrap().arguments.as_str())?;

    let mut context = Vec::new();
    for function_call_arg in function_call_args.params {
        let file = proj.identify(&function_call_arg.class_name)?;
        let query = format!(
            include_str!("queries/method_body_with_name.scm"),
            &function_call_arg.method_name
        );

        let res = file
            .query(&query)
            .or_else(|_| Ok::<Vec<Dict>, anyhow::Error>(vec![]))
            .unwrap();

        for r in res {
            let body = r.get("body").unwrap().to_string();
            context.push(format!(
                "Method body for `{}#{}`:",
                file.proper_name(),
                function_call_arg.method_name
            ));
            context.push(format!("\n```\n{}\n```\n", body));
        }
    }

    Ok(ChatCompletionRequestMessage {
        role:          Role::System,
        content:       Some(context.join("\n")),
        name:          Some(String::from("Instructor")),
        function_call: None,
    })
}

/// Returns a ChatCompletionRequestMessage with the given line references that
/// include contextual lines of code from the source
///
/// * `line_refs`: a vector of LineRef objects
/// * `proj`: a Project object
/// * `start_offset`: the number of lines of code to include before the line
/// * `num_lines`: the number of lines of code to include after the line
/// * `max_line_refs`: the maximum number of _processed_ line references to
///   include in the final message
/// * `try_use_active_retrieval`: whether to try to use active retrieval
/// * `additional_context`: additional context to use for
pub fn get_source_context<T: Into<LineRef>>(
    line_refs: Vec<T>,
    proj: Project,
    start_offset: usize,
    num_lines: usize,
    max_line_refs: usize,
    try_use_active_retrieval: bool,
    active_retrieval_context: Option<String>,
) -> Result<ChatCompletionRequestMessage> {
    if try_use_active_retrieval {
        if let Ok(message) = get_active_retrieval_context(&proj, active_retrieval_context) {
            return Ok(message);
        }
    }

    let mut line_refs: Vec<(File, LineRef, RangeInclusive<usize>)> = line_refs
        .into_iter()
        .map(|x| {
            let x = x.into();
            let file = proj.identify(&x.file_name)?;
            let start = match file.kind() {
                FileType::Test => x.line_number.saturating_sub(num_lines),
                _ => x.line_number.saturating_sub(start_offset),
            };
            let end = start + num_lines;
            Ok::<(File, LineRef, RangeInclusive<usize>), anyhow::Error>((file, x, start..=end))
        })
        .filter(Result::is_ok)
        .map(Result::unwrap)
        .collect();

    line_refs.sort_by(|lhs, rhs| {
        rhs.1
            .file_name
            .cmp(&lhs.1.file_name)
            .then(lhs.1.line_number.cmp(&rhs.1.line_number))
    });
    line_refs.dedup();

    let mut context = Vec::new();
    context.push(
        "You cannot see all of the student's submission as you are an AI language model, with \
         limited context length. Here are some snippets of code the stacktrace indicates might be \
         relevant:
:\n"
        .to_string(),
    );
    let end_ticks = "\n```\n".to_string();
    let mut methods: HashSet<String> = HashSet::new();

    line_refs
        .into_iter()
        .coalesce(|lhs, rhs| {
            if lhs.0 == rhs.0 {
                let lhs_start = *lhs.2.start();
                let lhs_end = *lhs.2.end();
                let rhs_start = *rhs.2.start();
                let rhs_end = *rhs.2.end();
                let expanded_range = rhs_start.saturating_sub(num_lines)..=(rhs_end + num_lines);

                if expanded_range.contains(&lhs_start) || expanded_range.contains(&lhs_end) {
                    Ok((lhs.0, lhs.1, lhs_start..=rhs_end))
                } else {
                    Err((lhs, rhs))
                }
            } else {
                Err((lhs, rhs))
            }
        })
        .take(max_line_refs)
        .for_each(|(file, f, r)| {
            let num_lines = r.size_hint().0;
            let count = file.parser().code().lines().count();

            let (f, r) = if num_lines as f32 >= 0.6 * (count as f32) {
                (f, 0..=count)
            } else {
                (f, r)
            };

            context.push(format!(
                "- Lines {} to {} from {} -\n```",
                *r.start(),
                *r.end(),
                f.file_name
            ));

            let relevant_source = file
                .parser()
                .code()
                .lines()
                .skip(*r.start())
                .filter(|line| !line.trim().is_empty())
                .take(num_lines)
                .map(|x| x.to_string().replace("\\\\", "\\").replace("\\\"", "\""))
                .collect::<Vec<String>>();

            context.append(&mut (relevant_source.clone()));
            context.push(end_ticks.clone());

            match Parser::new(relevant_source.join("\n"), *JAVA_TS_LANG) {
                Ok(parser) => {
                    let method_names: Vec<Dict> = parser
                        .query(METHOD_CALL_QUERY)
                        .or_else(|_| Ok::<Vec<Dict>, anyhow::Error>(vec![]))
                        .unwrap();

                    for method in method_names {
                        let method_name = method.get("name").unwrap().to_string();
                        methods.insert(method_name.clone());

                        let query = format!(
                            include_str!("queries/method_body_with_name.scm"),
                            &method_name
                        );

                        for f in proj.files() {
                            if *f.kind() == FileType::Class || *f.kind() == FileType::ClassWithMain
                            {
                                let res = f
                                    .query(&query)
                                    .or_else(|_| Ok::<Vec<Dict>, anyhow::Error>(vec![]))
                                    .unwrap();

                                for r in res {
                                    let body = r.get("body").unwrap().to_string();
                                    context.push(format!(
                                        "Method body for `{}#{}`:",
                                        f.proper_name(),
                                        method_name
                                    ));
                                    context.push(format!("\n```\n{}\n```\n", body));
                                }
                            }
                        }
                    }
                }
                Err(e) => {
                    eprintln!("Error parsing partial source context: {e}");
                }
            };
        });

    let mut context = context.join("\n");
    if context.len() > PROMPT_TRUNCATE {
        context.truncate(PROMPT_TRUNCATE);
        context.push_str("...[TRUNCATED]");
    }

    Ok(ChatCompletionRequestMessage {
        role:          Role::System,
        content:       Some(context),
        name:          Some(String::from("Instructor")),
        function_call: None,
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
            let output = match file.doc_check() {
                Ok(o) => o,
                Err(JavaFileError::DuringCompilation {
                    stacktrace,
                    diags,
                }) => {
                    let messages = vec![
                        ChatCompletionRequestMessage {
                            role:          Role::System,
                            content:       Some(SYSTEM_MESSAGE.to_string()),
                            name:          Some(String::from("Instructor")),
                            function_call: None,
                        },
                        ChatCompletionRequestMessage {
                            role:          Role::User,
                            content:       format!("Compiler error -\n```\n{}\n```", stacktrace)
                                .into(),
                            name:          Some(String::from("Student")),
                            function_call: None,
                        },
                        get_source_context(diags, self.project, 1, 3, 6, false, None)?,
                    ];

                    return Ok(GradeResult {
                        requirement: self.req_name,
                        grade:       Grade::new(0.0, out_of),
                        reason:      String::from("See above."),
                        prompt:      Some(messages),
                    });
                }
                Err(e) => {
                    let messages = vec![
                        ChatCompletionRequestMessage {
                            role:          Role::System,
                            content:       SYSTEM_MESSAGE.to_string().into(),
                            name:          Some(String::from("Instructor")),
                            function_call: None,
                        },
                        ChatCompletionRequestMessage {
                            role:          Role::User,
                            content:       format!("Unknown error -\n```\n{:?}\n```", e).into(),
                            name:          Some(String::from("Student")),
                            function_call: None,
                        },
                    ];

                    return Ok(GradeResult {
                        requirement: self.req_name,
                        grade:       Grade::new(0.0, out_of),
                        reason:      String::from("See above."),
                        prompt:      Some(messages),
                    });
                }
            };
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

        let prompt = if num_diags > 0 {
            let context = get_source_context(all_diags, self.project, 1, 3, 6, false, None)?;

            let mut outputs = outputs
                .iter()
                .map(|output| format!("```\n{output}\n```"))
                .collect::<Vec<String>>()
                .join("\n\n---\n\n");

            if outputs.len() > PROMPT_TRUNCATE {
                outputs.truncate(PROMPT_TRUNCATE);
                outputs.push_str("...[TRUNCATED]");
            }

            Some(vec![
                ChatCompletionRequestMessage {
                    role:          Role::System,
                    content:       SYSTEM_MESSAGE.to_string().into(),
                    name:          Some("Instructor".into()),
                    function_call: None,
                },
                ChatCompletionRequestMessage {
                    role:          Role::User,
                    content:       outputs.into(),
                    name:          Some("Student".into()),
                    function_call: None,
                },
                context,
                ChatCompletionRequestMessage {
                    role:          Role::System,
                    content:       include_str!("prompts/javadoc.md").to_string().into(),
                    name:          Some("Instructor".into()),
                    function_call: None,
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
        let convert_to_string = |f: Vec<Dynamic>| -> Result<Vec<String>> {
            f.iter()
                .map(|f| match f.clone().into_string() {
                    Ok(n) => Ok(n),
                    Err(e) => Err(anyhow!(
                        "test_files array has something that's not a string: {}",
                        e
                    )),
                })
                .try_collect()
        };

        let project = self.project.clone();
        let out_of = self.out_of;
        let req_name = self.req_name;
        let test_files: Vec<String> = convert_to_string(self.test_files)?;
        let expected_tests: Vec<String> = convert_to_string(self.expected_tests)?;

        let mut reasons = {
            let mut reasons = vec![];
            let mut actual_tests = vec![];
            let mut expected_tests = expected_tests;
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

            reasons
        };

        let new_user_message = |content: String| {
            let mut content = content;
            if content.len() > PROMPT_TRUNCATE {
                content.truncate(PROMPT_TRUNCATE);
                content.push_str("...[TRUNCATED]");
            }

            ChatCompletionRequestMessage {
                role:          Role::User,
                content:       Some(content),
                name:          Some("Student".into()),
                function_call: None,
            }
        };
        let new_system_message = |content: String| ChatCompletionRequestMessage {
            role:          Role::System,
            content:       Some(content),
            name:          Some("Instructor".into()),
            function_call: None,
        };
        let process_junit_stacktrace = |stacktrace: String| {
            let mut updated_stacktrace = Vec::new();
            let mut all_diags = Vec::new();

            for line in stacktrace.lines() {
                if line.contains("MethodSource") || line.contains("Native Method") {
                    continue;
                }

                if line.contains("Test run finished after") {
                    break;
                }

                if let Ok(diag) = parser::junit_stacktrace_line_ref(line) {
                    if project.identify(&diag.file_name).is_ok() {
                        updated_stacktrace
                            .push(line.replace("\\\\", "\\").replace("\\\"", "\"").to_string());
                    }
                    all_diags.push(diag);
                } else {
                    updated_stacktrace
                        .push(line.replace("\\\\", "\\").replace("\\\"", "\"").to_string());
                }
            }

            (updated_stacktrace, all_diags)
        };

        let initial_message = new_system_message(SYSTEM_MESSAGE.to_string());

        if !reasons.is_empty() {
            reasons.push("Tests will not be run until above is fixed.".into());
            let reasons = reasons.join("\n");
            let messages = vec![initial_message, new_user_message(reasons.clone())];
            Ok(GradeResult {
                requirement: req_name,
                grade:       Grade::new(0.0, out_of),
                reason:      reasons,
                prompt:      Some(messages),
            })
        } else {
            let mut num_tests_passed = 0.0;
            let mut num_tests_total = 0.0;
            let mut messages = vec![initial_message.clone()];

            for test_file in test_files {
                let res = match project
                    .identify(test_file.as_str())?
                    .test(Vec::new(), Some(&project))
                {
                    Ok(res) => res,
                    Err(JavaFileError::FailedTests {
                        test_results,
                        diags,
                    }) => {
                        let (updated_stacktrace, _) =
                            process_junit_stacktrace(test_results.clone());

                        messages.extend(vec![
                            new_user_message(format!(
                                "Failed tests -\n```\n{}\n```",
                                updated_stacktrace.join("\n")
                            )),
                            get_source_context(
                                diags,
                                project.clone(),
                                3,
                                6,
                                6,
                                *USE_ACTIVE_RETRIEVAL.try_get().unwrap_or(&false),
                                Some(updated_stacktrace.join("\n")),
                            )?,
                        ]);

                        test_results
                    }
                    Err(JavaFileError::Unknown(e)) => {
                        let out = format!("Unknown error -\n```\n{:#?}\n```", e);
                        messages.push(new_user_message(out.clone()));
                        out
                    }
                    Err(JavaFileError::DuringCompilation {
                        stacktrace,
                        diags,
                    }) => {
                        let out = format!("Compiler error -\n```\n{}\n```", stacktrace);
                        messages.extend(vec![
                            new_user_message(out.clone()),
                            get_source_context(diags, project.clone(), 3, 6, 6, false, None)?,
                        ]);
                        out
                    }
                    Err(JavaFileError::AtRuntime {
                        output,
                        diags,
                    }) => {
                        let out = format!("Error at runtime -\n```\n{}\n```", output);
                        messages.extend(vec![
                            new_user_message(out.clone()),
                            get_source_context(diags, project.clone(), 3, 6, 6, false, None)?,
                        ]);
                        out
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
                [
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
            let file = std::fs::File::open(ROOT_DIR.join("test_reports").join("mutations.csv"))
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
                let context = get_source_context(diags.clone(), project, 3, 6, 6, false, None)?;

                let mut feedback = ExpandedDisplay::new(diags).to_string();
                eprintln!("{feedback}");

                if feedback.len() > PROMPT_TRUNCATE {
                    feedback.truncate(PROMPT_TRUNCATE);
                    feedback.push_str("...[TRUNCATED]");
                }

                Some(vec![
                    ChatCompletionRequestMessage {
                        role:          Role::System,
                        content:       SYSTEM_MESSAGE.to_string().into(),
                        name:          Some("Instructor".into()),
                        function_call: None,
                    },
                    // ChatCompletionRequestMessage {
                    //     role:    Role::System,
                    //     content: project.describe(),
                    //     name:    Some("Instructor".into()),
                    // },
                    ChatCompletionRequestMessage {
                        role:          Role::User,
                        content:       feedback.into(),
                        name:          Some("Student".into()),
                        function_call: None,
                    },
                    context,
                    ChatCompletionRequestMessage {
                        role:          Role::System,
                        content:       format!(
                            include_str!("prompts/mutation_testing.md"),
                            test = target_test.join(", "),
                            class = target_class.join(", ")
                        )
                        .into(),
                        name:          Some("Instructor".into()),
                        function_call: None,
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
            if output.len() > PROMPT_TRUNCATE {
                output.truncate(PROMPT_TRUNCATE);
                output.push_str("...[TRUNCATED]");
            }

            let prompt = if !output.is_empty() {
                Some(vec![
                    ChatCompletionRequestMessage {
                        role:          Role::System,
                        content:       SYSTEM_MESSAGE.to_string().into(),
                        name:          Some("Instructor".into()),
                        function_call: None,
                    },
                    // ChatCompletionRequestMessage {
                    //     role:    Role::System,
                    //     content: project.describe(),
                    //     name:    Some("Instructor".into()),
                    // },
                    ChatCompletionRequestMessage {
                        role:          Role::User,
                        content:       Some(output),
                        name:          Some("Student".into()),
                        function_call: None,
                    },
                    ChatCompletionRequestMessage {
                        role:          Role::System,
                        content:       format!(
                            include_str!("prompts/mutation_testing_2.md"),
                            test = target_test.join(", "),
                            class = target_class.join(", ")
                        )
                        .into(),
                        name:          Some("Instructor".into()),
                        function_call: None,
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
        let mut file = std::fs::File::create(&path)?;
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
    pub req_name:    String,
    /// points to give if all tests pass
    pub out_of:      f64,
    /// the project to grade
    pub project:     Project,
    /// Java file to run
    pub file:        String,
    /// the expected output
    pub expected:    Array,
    /// the actual output
    pub input:       Array,
    /// ignore case when comparing
    pub ignore_case: bool,
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
    pub fn expected(&mut self) -> Array {
        self.expected.clone()
    }

    /// sets the `expected` field
    pub fn set_expected(mut self, expected: Array) -> Self {
        self.expected = expected;
        self
    }

    /// gets the `actual` field
    pub fn input(&mut self) -> Array {
        self.input.clone()
    }

    /// sets the `actual` field
    pub fn set_input(mut self, input: Array) -> Self {
        self.input = input;
        self
    }

    /// gets the `project` field
    pub fn project(&mut self) -> Project {
        self.project.clone()
    }

    /// sets the `project` field
    pub fn set_project(mut self, project: Project) -> Self {
        self.project = project;
        self
    }

    /// gets the `file` field
    pub fn file(&mut self) -> String {
        self.file.clone()
    }

    /// sets the `file` field
    pub fn set_file(mut self, file: String) -> Self {
        self.file = file;
        self
    }

    /// gets the `ignore_case` field
    pub fn ignore_case(&mut self) -> bool {
        self.ignore_case
    }

    /// sets the `ignore_case` field
    pub fn set_ignore_case(mut self, ignore_case: bool) -> Self {
        self.ignore_case = ignore_case;
        self
    }

    #[generate_rhai_variant(Fallible)]
    /// Grades by diffing the `expected` and `actual` strings.
    pub fn grade_by_diff(&mut self) -> Result<GradeResult> {
        ensure!(
            !self.expected.is_empty() & !self.input.is_empty(),
            "At least one test case (input-expected pair) must be provided"
        );
        ensure!(
            self.expected.len() == self.input.len(),
            "expected and input case arrays must be of the same length"
        );

        let file = self.project.identify(&self.file)?;
        let mut prompts = vec![];

        for (expected, input) in self.expected.iter().zip(self.input.iter()) {
            let expected = {
                let expected = expected.clone().cast::<String>();
                if self.ignore_case {
                    expected.to_lowercase().trim().to_string()
                } else {
                    expected.trim().to_string()
                }
            };
            let input = input.clone().cast::<String>();

            let actual_out = {
                let out = match file.run(Some(input.clone())) {
                    Ok(out) => out,
                    Err(JavaFileError::AtRuntime {
                        output,
                        diags,
                    }) => {
                        let messages = vec![
                            ChatCompletionRequestMessage {
                                role:          Role::System,
                                content:       SYSTEM_MESSAGE.to_string().into(),
                                name:          Some("Instructor".into()),
                                function_call: None,
                            },
                            ChatCompletionRequestMessage {
                                role:          Role::User,
                                content:       format!(
                                    "Error while running -\n```\n{}\n```",
                                    output
                                )
                                .into(),
                                name:          Some("Student".into()),
                                function_call: None,
                            },
                            get_source_context(diags, self.project.clone(), 3, 6, 6, false, None)?,
                        ];
                        return Ok(GradeResult {
                            requirement: self.req_name.clone(),
                            grade:       Grade::new(0.0, self.out_of),
                            reason:      "Error running file for some cases.".to_string(),
                            prompt:      Some(messages),
                        });
                    }
                    Err(JavaFileError::DuringCompilation {
                        stacktrace,
                        diags,
                    }) => {
                        let messages = vec![
                            ChatCompletionRequestMessage {
                                role:          Role::System,
                                content:       SYSTEM_MESSAGE.to_string().into(),
                                name:          Some("Instructor".into()),
                                function_call: None,
                            },
                            ChatCompletionRequestMessage {
                                role:          Role::User,
                                content:       format!(
                                    "Error while compiling -\n```\n{}\n```",
                                    stacktrace
                                )
                                .into(),
                                name:          Some("Student".into()),
                                function_call: None,
                            },
                            get_source_context(diags, self.project.clone(), 3, 6, 6, false, None)?,
                        ];
                        return Ok(GradeResult {
                            requirement: self.req_name.clone(),
                            grade:       Grade::new(0.0, self.out_of),
                            reason:      "Error compiling file for some cases.".to_string(),
                            prompt:      Some(messages),
                        });
                    }
                    Err(e) => {
                        let messages = vec![
                            ChatCompletionRequestMessage {
                                role:          Role::System,
                                content:       SYSTEM_MESSAGE.to_string().into(),
                                name:          Some("Instructor".into()),
                                function_call: None,
                            },
                            ChatCompletionRequestMessage {
                                role:          Role::User,
                                content:       format!("Unknown error -\n```\n{:?}\n```", e).into(),
                                name:          Some("Student".into()),
                                function_call: None,
                            },
                        ];
                        return Ok(GradeResult {
                            requirement: self.req_name.clone(),
                            grade:       Grade::new(0.0, self.out_of),
                            reason:      "Unknown error while running file for some cases."
                                .to_string(),
                            prompt:      Some(messages),
                        });
                    }
                };

                if self.ignore_case {
                    out.to_lowercase().trim().to_string()
                } else {
                    out.trim().to_string()
                }
            };

            let diff = diff_unicode_words(Algorithm::Patience, &expected, &actual_out);

            let mut is_equal = true;
            let mut expected = String::new();
            let mut actual = String::new();

            for (change, value) in diff {
                match change {
                    ChangeTag::Equal => {
                        expected.push_str(value);
                        actual.push_str(value);
                    }
                    ChangeTag::Insert => {
                        actual.push_str(format!("{}", value.green()).as_str());
                        if !value.trim().is_empty() {
                            is_equal = false;
                        }
                    }
                    ChangeTag::Delete => {
                        expected.push_str(format!("{}", value.red()).as_str());
                        if !value.trim().is_empty() {
                            is_equal = false;
                        }
                    }
                }
            }

            if !is_equal {
                let prompt = format!(
                    "Comparing expected and actual output for \
                     {}:\n```{inp}Expected:\n{}\nActual:\n{}\n```\n",
                    file.file_name(),
                    expected,
                    actual,
                    inp = if self.input.is_empty() {
                        String::new()
                    } else {
                        format!("\nInput:\n`{}`\n", input)
                    },
                );

                eprintln!("{prompt}");
                prompts.push(prompt);
            }
        }

        if prompts.is_empty() {
            Ok(GradeResult {
                requirement: self.req_name.clone(),
                grade:       Grade {
                    grade:  self.out_of,
                    out_of: self.out_of,
                },
                reason:      "Got expected output".to_string(),
                prompt:      None,
            })
        } else {
            let context = format!(
                "{prompt}\n\nSource code:\n```java\n{code}\n```\nMy tests are failing due to the \
                 above.",
                prompt = prompts.join("\n\n"),
                code = file.parser().code()
            );

            Ok(GradeResult {
                requirement: self.req_name.clone(),
                grade:       Grade {
                    grade:  0.0,
                    out_of: self.out_of,
                },
                reason:      "See above.".to_string(),
                prompt:      Some(vec![
                    ChatCompletionRequestMessage {
                        role:          Role::System,
                        content:       SYSTEM_MESSAGE.to_string().into(),
                        name:          Some("Instructor".into()),
                        function_call: None,
                    },
                    ChatCompletionRequestMessage {
                        role:          Role::System,
                        content:       Some(context),
                        name:          Some("Student".into()),
                        function_call: None,
                    },
                ]),
            })
        }
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

#[derive(Default, Debug, Clone)]
/// A struct to represet a treesitter query.
pub struct Query {
    /// The query to run.
    query:   String,
    /// The capture to extract from the query.
    capture: String,
    /// A function pointer to filter the matches using. Must return a boolean.
    filter:  Option<FnPtr>,
}

impl Query {
    /// Creates a new query with default values (empty strings).
    pub fn new() -> Self {
        Self::default()
    }

    /// Gets the query to run.
    pub fn query(&self) -> String {
        unescape(&format!("{:#?}", self.query)).unwrap()
    }

    /// Sets the query to run.
    pub fn set_query(mut self, query: String) -> Self {
        self.query = query;
        self
    }

    /// Gets the captures to extract from the query.
    pub fn capture(&self) -> String {
        self.capture.clone()
    }

    /// Sets the captures to extract from the query.
    pub fn set_capture(mut self, capture: String) -> Self {
        self.capture = capture;
        self
    }

    /// Gets the function to filter the results of the query.
    pub fn filter(&self) -> Option<FnPtr> {
        self.filter.clone()
    }

    /// Set the function to filter the results of the query.
    pub fn set_filter(mut self, filter: FnPtr) -> Self {
        self.filter = Some(filter);
        self
    }
}

/// An enum to represent possible errors when running a query.
#[derive(thiserror::Error, Debug)]
pub enum QueryError {
    /// No file was selected to run the query on.
    #[error("No file was selected to run the query on.")]
    NoFileSelected,
    /// No capture was selected to extract from the query.
    #[error("No capture was selected to extract from the query: {0}")]
    NoCaptureSelected(String),
    /// No previous query to add capture or filter to.
    #[error("No previous query to add capture or filter to.")]
    NoPreviousQuery,
    /// The file selected to run the query on does not exist.
    #[error("The file selected (`{0}`) to run the query on could not be found.")]
    FileNotFound(String),
    /// The query could not be run.
    #[error(
        "This query could not be run, likely due to a syntax \
         error.\nQuery:\n```\n{q}\n```\nError:\n```\n{e}\n```"
    )]
    DuringQueryExecution {
        /// The query that could not be run.
        q: String,
        /// The error that occurred.
        e: String,
    },
    /// No matches found for a previously selected capture, all subsequent
    /// queries will return nothing.
    #[error(
        "No matches found for a previously selected capture: `{0}`, all subsequent queries will \
         return nothing."
    )]
    NoMatchesFound(String),
    /// Unknown error.
    #[error("Unknown error: {0}")]
    Unknown(#[from] anyhow::Error),
}

#[derive(Default, Clone)]
/// An enum to represent the constraint of a query.
pub enum QueryConstraint {
    #[default]
    /// The query must match at least once.
    MustMatchAtLeastOnce,
    /// The query must match exactly once.
    MustMatchExactlyNTimes(usize),
    /// Must not match.
    MustNotMatch,
}

#[derive(Default, Clone)]
/// A struct to represent a query grader.
pub struct QueryGrader {
    /// The name of the requirement.
    req_name:   String,
    /// The grade for the requirement.
    out_of:     f64,
    /// The queries to run.
    queries:    Vec<Query>,
    /// The input to run the queries on.
    project:    Project,
    /// The file to run the query on.
    file:       String,
    /// The constraint of the query.
    constraint: QueryConstraint,
    /// The reason to share with the student.
    reason:     String,
}

impl QueryGrader {
    /// Creates a new query grader with default values.
    pub fn new() -> Self {
        Self::default()
    }

    /// Gets the name of the requirement.
    pub fn req_name(&self) -> &str {
        &self.req_name
    }

    /// Sets the name of the requirement.
    pub fn set_req_name(mut self, req_name: String) -> Self {
        self.req_name = req_name;
        self
    }

    /// Gets the "out of" grade for the requirement.
    pub fn out_of(&self) -> f64 {
        self.out_of
    }

    /// Sets the "out of" grade for the requirement.
    pub fn set_out_of(mut self, out_of: f64) -> Self {
        self.out_of = out_of;
        self
    }

    /// Gets the file to run the query on.
    pub fn file(&self) -> &str {
        &self.file
    }

    /// Sets the file to run the query on.
    pub fn set_file(mut self, file: String) -> Self {
        self.file = file;
        self
    }

    /// Gets the project to run the query on.
    pub fn project(&self) -> &Project {
        &self.project
    }

    /// Sets the project to run the query on.
    pub fn set_project(mut self, project: Project) -> Self {
        self.project = project;
        self
    }

    /// Gets the queries to run.
    pub fn queries(&self) -> Vec<Query> {
        self.queries.clone()
    }

    /// Gets the constraint of the query.
    pub fn constraint(&self) -> QueryConstraint {
        self.constraint.clone()
    }

    /// Sets the constraint of the query to "must match at least once".
    pub fn must_match_at_least_once(mut self) -> Self {
        self.constraint = QueryConstraint::MustMatchAtLeastOnce;
        self
    }

    /// Sets the constraint of the query to "must match exactly n times".
    pub fn must_match_exactly_n_times(mut self, n: usize) -> Self {
        self.constraint = QueryConstraint::MustMatchExactlyNTimes(n);
        self
    }

    /// Sets the constraint of the query to "must not match".
    pub fn must_not_match(mut self) -> Self {
        self.constraint = QueryConstraint::MustNotMatch;
        self
    }

    /// Gets the reason to share with the student.
    pub fn reason(&self) -> &str {
        &self.reason
    }

    /// Sets the reason to share with the student.
    pub fn set_reason(mut self, reason: String) -> Self {
        self.reason = reason;
        self
    }

    #[generate_rhai_variant(Fallible)]
    /// Adds a query to run.
    /// If no file has been selected, this will throw an error.
    pub fn query(#[allow(unused_mut)] mut self, q: String) -> Result<Self, QueryError> {
        if self.file.is_empty() {
            return Err(QueryError::NoFileSelected);
        }

        self.queries.push(Query {
            query:   q,
            capture: String::new(),
            filter:  None,
        });

        Ok(self)
    }

    #[generate_rhai_variant(Fallible)]
    /// Adds a capture to the last query.
    /// If no queries have been added, this will throw an error.
    pub fn capture(#[allow(unused_mut)] mut self, c: String) -> Result<Self, QueryError> {
        if let Some(last) = self.queries.last_mut() {
            *last = last.clone().set_capture(c);
            Ok(self)
        } else {
            Err(QueryError::NoPreviousQuery)
        }
    }

    #[generate_rhai_variant(Fallible)]
    /// Adds a capture to the last query.
    /// If no queries have been added, this will throw an error.
    pub fn filter(#[allow(unused_mut)] mut self, f: FnPtr) -> Result<Self, QueryError> {
        if let Some(last) = self.queries.last_mut() {
            *last = last.clone().set_filter(f);
            Ok(self)
        } else {
            Err(QueryError::NoPreviousQuery)
        }
    }

    /// Selects entire method body and returns
    pub fn method_body_with_name(mut self, method_name: String) -> Self {
        self.queries.push(Query {
            query:   format!(
                include_str!("queries/method_body_with_name.scm"),
                method_name
            ),
            capture: "body".to_string(),
            filter:  None,
        });
        self
    }

    /// Selects entire method body and returns
    pub fn method_body_with_return_type(mut self, return_type: String) -> Self {
        self.queries.push(Query {
            query:   format!(
                include_str!("queries/method_body_with_return_type.scm"),
                return_type
            ),
            capture: "body".to_string(),
            filter:  None,
        });
        self
    }

    /// Selects and returns the entire main method
    pub fn main_method(mut self) -> Self {
        self.queries.push(Query {
            query:   include_str!("queries/main_method.scm").to_string(),
            capture: "body".to_string(),
            filter:  None,
        });
        self
    }

    /// Selects entire class body with name
    pub fn class_body_with_name(mut self, class_name: String) -> Self {
        self.queries.push(Query {
            query:   format!(include_str!("queries/class_with_name.scm"), class_name),
            capture: "body".to_string(),
            filter:  None,
        });
        self
    }

    /// Selects local variable declaration statements
    pub fn local_variables(mut self) -> Self {
        self.queries.push(Query {
            query:   String::from("((local_variable_declaration) @var)"),
            capture: "var".to_string(),
            filter:  None,
        });
        self
    }

    /// Selects local variable declaration statements with supplied name
    pub fn local_variables_with_name(mut self, name: String) -> Self {
        self.queries.push(Query {
            query:   format!(include_str!("queries/local_variable_with_name.scm"), name),
            capture: "body".to_string(),
            filter:  None,
        });
        self
    }

    /// Selects local variable declaration statements with supplied type
    pub fn local_variables_with_type(mut self, type_name: String) -> Self {
        self.queries.push(Query {
            query:   format!(
                include_str!("queries/local_variable_with_type.scm"),
                type_name
            ),
            capture: "body".to_string(),
            filter:  None,
        });
        self
    }

    /// Selects if statements (entire, including else if and else)
    pub fn if_statements(mut self) -> Self {
        self.queries.push(Query {
            query:   String::from("((if_statement) @if)"),
            capture: "if".to_string(),
            filter:  None,
        });
        self
    }

    /// Selects for loops
    pub fn for_loops(mut self) -> Self {
        self.queries.push(Query {
            query:   String::from("((for_statement) @for)"),
            capture: "for".to_string(),
            filter:  None,
        });
        self
    }

    /// Selects while loops
    pub fn while_loops(mut self) -> Self {
        self.queries.push(Query {
            query:   String::from("((while_statement) @while)"),
            capture: "while".to_string(),
            filter:  None,
        });
        self
    }

    /// Selects method invocations
    pub fn method_invocations(mut self) -> Self {
        self.queries.push(Query {
            query:   include_str!("queries/method_invocation.scm").to_string(),
            capture: "body".to_string(),
            filter:  None,
        });
        self
    }

    /// Selects method invocations with supplied name
    pub fn method_invocations_with_name(mut self, name: String) -> Self {
        self.queries.push(Query {
            query:   format!(
                include_str!("queries/method_invocations_with_name.scm"),
                name
            ),
            capture: "body".to_string(),
            filter:  None,
        });
        self
    }

    /// Selects method invocations with supplied arguments
    pub fn method_invocations_with_arguments(mut self, name: String) -> Self {
        self.queries.push(Query {
            query:   format!(
                include_str!("queries/method_invocations_with_arguments.scm"),
                name
            ),
            capture: "body".to_string(),
            filter:  None,
        });
        self
    }

    /// Selects method invocations with supplied object
    pub fn method_invocations_with_object(mut self, name: String) -> Self {
        self.queries.push(Query {
            query:   format!(
                include_str!("queries/method_invocations_with_object.scm"),
                name
            ),
            capture: "body".to_string(),
            filter:  None,
        });
        self
    }

    #[generate_rhai_variant(Fallible)]
    /// Runs the queries, and returns the result.
    /// TODO: Make it so that it doesn't parse a new peice of code, just filters
    /// out the irrelevant line ranges. This performs better but more
    /// importantly is more accurate.
    pub fn run_query(&self) -> Result<Dynamic, QueryError> {
        let engine = create_engine();
        let ast = std::sync::Arc::clone(&SCRIPT_AST);
        let ast = ast.lock().unwrap();

        let first = self
            .queries
            .first()
            .ok_or_else(|| QueryError::NoMatchesFound("No queries to run".to_string()))?;

        let file = self
            .project
            .identify(self.file())
            .map_err(|_| QueryError::FileNotFound(self.file().to_string()))?;

        let mut matches: Vec<String> = match file.query(&first.query()) {
            Ok(m) => {
                if first.capture().is_empty() {
                    return Err(QueryError::NoCaptureSelected(format!("{:#?}", first)));
                }
                let result = m
                    .iter()
                    .filter_map(|map| map.get(&first.capture()))
                    .cloned();

                let result: Vec<String> = if let Some(f) = first.filter() {
                    result
                        .filter(|x| f.call(&engine, &ast, (x.clone(),)).unwrap_or(false))
                        .collect()
                } else {
                    result.collect()
                };

                if m.is_empty() {
                    return Err(QueryError::NoMatchesFound(
                        unescape(&format!("{:#?}", first)).context("Unescape error")?,
                    ));
                }
                result
            }
            Err(e) => {
                return Err(QueryError::DuringQueryExecution {
                    q: first.query(),
                    e: format!("{:#?}", e),
                })
            }
        };

        if self.queries.len() == 1 {
            return Ok(matches.into());
        }

        for (prev_q, q) in self.queries().into_iter().tuple_windows() {
            if matches.is_empty() {
                return Err(QueryError::NoMatchesFound(
                    unescape(&format!("{:#?}", prev_q)).context("Unescape error")?,
                ));
            }

            if q.capture().is_empty() {
                return Err(QueryError::NoCaptureSelected(format!("{:#?}", q)));
            }

            let mut new_matches = vec![];

            for code in matches {
                let parser = Parser::new(code, *JAVA_TS_LANG).context(format!(
                    "Failed to create parser for query: `{}`",
                    q.query()
                ))?;

                match parser.query(&q.query()) {
                    Ok(m) => {
                        let result = m.iter().filter_map(|map| map.get(&q.capture())).cloned();

                        let mut result: Vec<String> = if let Some(f) = q.filter() {
                            result
                                .filter(|x| f.call(&engine, &ast, (x.clone(),)).unwrap_or(false))
                                .collect()
                        } else {
                            result.collect()
                        };

                        new_matches.append(&mut result)
                    }
                    Err(e) => {
                        return Err(QueryError::DuringQueryExecution {
                            q: q.query(),
                            e: format!("{:#?}", e),
                        })
                    }
                };
            }

            matches = new_matches;
        }

        Ok(matches.into())
    }

    #[generate_rhai_variant(Fallible)]
    /// Grades the file according to the supplied queries, captures, and
    /// constraints.
    pub fn grade_by_query(self) -> Result<GradeResult> {
        let reason = if self.reason.trim().is_empty() {
            eprintln!(
                "Warning: No reason provided for query grading. Feedback to student will not be \
                 very helpful."
            );
            match self.constraint {
                QueryConstraint::MustMatchAtLeastOnce => {
                    "Query Constraint: Must match at least once.".to_string()
                }
                QueryConstraint::MustMatchExactlyNTimes(n) => {
                    format!("Query Constraint: Must match exactly {n} times.")
                }
                QueryConstraint::MustNotMatch => "Query Constraint: Must not match.".to_string(),
            }
        } else {
            self.reason.to_string()
        };

        let result: Vec<String> = match self.run_query() {
            Ok(r) => {
                let r: Array = r.cast();
                r.into_iter().map(|s| s.cast()).collect()
            }
            Err(e) => {
                return Ok(GradeResult {
                    requirement: self.req_name.clone(),
                    grade: Grade {
                        grade:  0.0,
                        out_of: self.out_of,
                    },
                    reason,
                    prompt: Some(vec![
                        ChatCompletionRequestMessage {
                            role:          Role::System,
                            content:       SYSTEM_MESSAGE.to_string().into(),
                            name:          Some("Instructor".into()),
                            function_call: None,
                        },
                        ChatCompletionRequestMessage {
                            role:          Role::System,
                            content:       format!(
                                "Something went wrong when using treesitter queries to grade \
                                 `{}`. Error message:\n\n```\n{}\n```\n",
                                self.file, e
                            )
                            .into(),
                            name:          Some("Instructor".into()),
                            function_call: None,
                        },
                    ]),
                })
            }
        };

        match self.constraint {
            QueryConstraint::MustMatchAtLeastOnce => {
                if result.is_empty() {
                    Ok(GradeResult {
                        requirement: self.req_name.clone(),
                        grade: Grade {
                            grade:  0.0,
                            out_of: self.out_of,
                        },
                        reason,
                        prompt: Some(vec![
                            ChatCompletionRequestMessage {
                                role:          Role::System,
                                content:       SYSTEM_MESSAGE.to_string().into(),
                                name:          Some("Instructor".into()),
                                function_call: None,
                            },
                            ChatCompletionRequestMessage {
                                role:          Role::System,
                                content:       format!(
                                    "For file `{}`: {}.",
                                    self.file, self.reason
                                )
                                .into(),
                                name:          Some("Instructor".into()),
                                function_call: None,
                            },
                        ]),
                    })
                } else {
                    Ok(GradeResult {
                        requirement: self.req_name.clone(),
                        grade: Grade {
                            grade:  self.out_of,
                            out_of: self.out_of,
                        },
                        reason,
                        prompt: None,
                    })
                }
            }
            QueryConstraint::MustMatchExactlyNTimes(n) => {
                if result.len() == n {
                    Ok(GradeResult {
                        requirement: self.req_name.clone(),
                        grade: Grade {
                            grade:  self.out_of,
                            out_of: self.out_of,
                        },
                        reason,
                        prompt: None,
                    })
                } else {
                    Ok(GradeResult {
                        requirement: self.req_name.clone(),
                        grade: Grade {
                            grade:  0.0,
                            out_of: self.out_of,
                        },
                        reason,
                        prompt: Some(vec![
                            ChatCompletionRequestMessage {
                                role:          Role::System,
                                content:       SYSTEM_MESSAGE.to_string().into(),
                                name:          Some("Instructor".into()),
                                function_call: None,
                            },
                            ChatCompletionRequestMessage {
                                role:          Role::System,
                                content:       format!("For file `{}`: {}", self.file, self.reason)
                                    .into(),
                                name:          Some("Instructor".into()),
                                function_call: None,
                            },
                        ]),
                    })
                }
            }
            QueryConstraint::MustNotMatch => {
                if result.is_empty() {
                    Ok(GradeResult {
                        requirement: self.req_name.clone(),
                        grade: Grade {
                            grade:  self.out_of,
                            out_of: self.out_of,
                        },
                        reason,
                        prompt: None,
                    })
                } else {
                    Ok(GradeResult {
                        requirement: self.req_name.clone(),
                        grade: Grade {
                            grade:  0.0,
                            out_of: self.out_of,
                        },
                        reason,
                        prompt: Some(vec![
                            ChatCompletionRequestMessage {
                                role:          Role::System,
                                content:       SYSTEM_MESSAGE.to_string().into(),
                                name:          Some("Instructor".into()),
                                function_call: None,
                            },
                            ChatCompletionRequestMessage {
                                role:          Role::System,
                                content:       format!("For file `{}`: {}", self.file, self.reason)
                                    .into(),
                                name:          Some("Instructor".into()),
                                function_call: None,
                            },
                        ]),
                    })
                }
            }
        }
    }
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
            .with_fn("input", Self::input)
            .with_fn("input", Self::set_input)
            .with_fn("project", Self::project)
            .with_fn("project", Self::set_project)
            .with_fn("file", Self::file)
            .with_fn("file", Self::set_file)
            .with_fn("ignore_case", Self::ignore_case)
            .with_fn("ignore_case", Self::set_ignore_case)
            .with_fn("new_diff_grader", Self::default)
            .with_fn("run", Self::grade_by_diff_script);
    }
}

// Allowed because CustomType is not deprecated, just volatile
#[allow(deprecated)]
/// Allows registering custom types with Rhai.
impl CustomType for Query {
    /// Builds a custom type to be registered with Rhai.
    fn build(mut builder: rhai::TypeBuilder<Self>) {
        builder
            .with_name("Query")
            .with_fn("new_query", Self::new)
            .with_fn("query", Self::query)
            .with_fn("query", Self::set_query)
            .with_fn("capture", Self::capture)
            .with_fn("capture", Self::set_capture);
    }
}

// Allowed because CustomType is not deprecated, just volatile
#[allow(deprecated)]
/// Allows registering custom types with Rhai.
impl CustomType for QueryGrader {
    /// Builds a custom type to be registered with Rhai.
    fn build(mut builder: rhai::TypeBuilder<Self>) {
        builder
            .with_name("QueryGrader")
            .with_fn("req_name", Self::req_name)
            .with_fn("req_name", Self::set_req_name)
            .with_fn("out_of", Self::out_of)
            .with_fn("out_of", Self::set_out_of)
            .with_fn("file", Self::file)
            .with_fn("file", Self::set_file)
            .with_fn("project", Self::project)
            .with_fn("project", Self::set_project)
            .with_fn("queries", Self::queries)
            .with_fn("query", Self::query_script)
            .with_fn("capture", Self::capture_script)
            .with_fn("reason", Self::reason)
            .with_fn("reason", Self::set_reason)
            .with_fn("must_match_at_least_once", Self::must_match_at_least_once)
            .with_fn(
                "must_match_exactly_n_times",
                Self::must_match_exactly_n_times,
            )
            .with_fn("must_not_match", Self::must_not_match)
            .with_fn("method_body_with_name", Self::method_body_with_name)
            .with_fn(
                "method_body_with_return_type",
                Self::method_body_with_return_type,
            )
            .with_fn("main_method", Self::main_method)
            .with_fn("class_body_with_name", Self::class_body_with_name)
            .with_fn("local_variables", Self::local_variables)
            .with_fn("local_variables_with_name", Self::local_variables_with_name)
            .with_fn("local_variables_with_type", Self::local_variables_with_type)
            .with_fn("if_statements", Self::if_statements)
            .with_fn("for_loops", Self::for_loops)
            .with_fn("while_loops", Self::while_loops)
            .with_fn("method_invocations", Self::method_invocations)
            .with_fn(
                "method_invocations_with_name",
                Self::method_invocations_with_name,
            )
            .with_fn(
                "method_invocations_with_arguments",
                Self::method_invocations_with_arguments,
            )
            .with_fn(
                "method_invocations_with_object",
                Self::method_invocations_with_object,
            )
            .with_fn("filter", Self::filter_script)
            .with_fn("run_query", Self::run_query_script)
            .with_fn("run", Self::grade_by_query_script)
            .with_fn("new_query_grader", Self::default);
    }
}
