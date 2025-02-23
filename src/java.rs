#![warn(missing_docs)]
#![warn(clippy::missing_docs_in_private_items)]

use std::{
    fmt::Formatter,
    hash::{Hash, Hasher},
    io::Write,
    path::PathBuf,
    process::{Command, Output, Stdio},
};

use anyhow::{anyhow, bail, Context, Result};
use futures::{
    future::{join_all, try_join_all},
    stream::FuturesUnordered,
};
use rhai::Array;
// Allowed because CustomType is not deprecated, just volatile
#[allow(deprecated)]
use rhai::{CustomType, EvalAltResult};
use serde::{Deserialize, Serialize};
use snailquote::unescape;
use tokio::io::AsyncWriteExt;
use tree_sitter::{Query, QueryCursor, Tree};
use umm_derive::generate_rhai_variant;

use crate::{
    constants::*,
    grade::{JavacDiagnostic, LineRef},
    parsers::parser,
    util::*,
    vscode::{self},
    Dict,
};

/// Types of Java files -
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum FileType {
    /// - Interface
    Interface,
    /// - Class
    Class,
    /// - Class with a main method
    ClassWithMain,
    /// - JUnit test class
    Test,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
/// Struct representing a java file
pub struct File {
    /// path to java file.
    path:         PathBuf,
    /// name of file.
    file_name:    String,
    /// package the java file belongs to.
    package_name: Option<String>,
    /// imports made by the java file.
    imports:      Option<Vec<Dict>>,
    /// name of the file TODO: How does this differ from `file_name`?
    name:         String,
    /// colored terminal string representing java file name.
    proper_name:  String,
    /// Name of tests methods in this file, as understood by JUnit.
    test_methods: Vec<String>,
    /// Name of tests methods in this file, colored using terminal color codes.
    kind:         FileType,
    #[serde(skip)]
    /// The parser for this file
    parser:       Parser,
    /// Concise description of the file
    description:  String,
}

/// Two `File`s are equal if their paths are equal
impl PartialEq for File {
    fn eq(&self,
          other: &Self)
          -> bool {
        self.path == other.path
    }
}

/// Based on PartialEq
impl Eq for File {}

/// Hash based on path
impl Hash for File {
    fn hash<H: Hasher>(&self,
                       state: &mut H) {
        self.path.hash(state);
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
/// Struct representing a Java project.
/// Any index `i` in any collection in this struct always refers to the same
/// JavaFile.
pub struct Project {
    /// Collection of java files in this project
    files:      Vec<File>,
    /// Names of java files in this project.
    names:      Vec<String>,
    /// Classpath
    classpath:  Vec<String>,
    /// Source path
    sourcepath: Vec<String>,
    /// Root directory
    root_dir:   String,
}

#[derive(Clone)]
/// A struct that wraps a tree-sitter parser object and source code
pub struct Parser {
    /// the source code being parsed
    code:  String,
    /// the parse tree
    _tree: Option<Tree>,
    /// the tree-sitter java grammar language
    lang:  tree_sitter::Language,
}

impl Default for Parser {
    fn default() -> Self {
        let mut parser = tree_sitter::Parser::new();
        let code = String::new();
        parser.set_language(&tree_sitter_java::language())
              .expect("Error loading Java grammar");
        let tree = parser.parse(code, None);

        Self { code:  String::new(),
               _tree: tree,
               lang:  tree_sitter_java::language(), }
    }
}

impl std::fmt::Debug for Parser {
    fn fmt(&self,
           _: &mut Formatter<'_>)
           -> std::fmt::Result {
        Ok(())
    }
}

impl Parser {
    #[generate_rhai_variant(Impl, Fallible)]
    /// Returns a new parser object
    ///
    /// * `source_code`: the source code to be parsed
    /// * `lang`: the tree-sitter grammar to use
    pub fn new(source_code: String) -> Result<Self> {
        let mut parser = tree_sitter::Parser::new();

        parser.set_language(&tree_sitter_java::language())
              .expect("Error loading Java grammar");
        let tree = parser.parse(source_code.clone(), None)
                         .context("Error parsing Java code")?;

        Ok(Self { code:  source_code,
                  _tree: Some(tree),
                  lang:  tree_sitter_java::language(), })
    }

    /// A getter for parser's source code
    pub fn code(&mut self) -> String {
        self.code.clone()
    }

    /// A setter for parser's source code
    pub fn set_code(&mut self,
                    code: String) {
        self.code = code;
    }

    #[generate_rhai_variant(Fallible, Mut)]
    /// Applies a tree sitter query and returns the result as a collection of
    /// HashMaps
    ///
    /// * `q`: the tree-sitter query to be applied
    pub fn query(&self,
                 q: &str)
                 -> Result<Vec<Dict>> {
        let mut results = vec![];
        let tree = self._tree
                       .as_ref()
                       .context("Treesitter could not parse code")?;

        let query = Query::new(&self.lang, q).unwrap();
        let mut cursor = QueryCursor::new();
        let matches = cursor.matches(&query, tree.root_node(), self.code.as_bytes());
        let capture_names = query.capture_names();

        for m in matches {
            let mut result = Dict::new();

            for name in capture_names {
                let index = query.capture_index_for_name(name);
                let index = match index {
                    Some(i) => i,
                    None => bail!("Error while querying source code. Capture name: {} has no \
                                   index associated.",
                                  name),
                };

                let value = m.captures.iter().find(|c| c.index == index);
                let value = match value {
                    Some(v) => v,
                    None => continue,
                };

                let value = value.node
                                 .utf8_text(self.code.as_bytes())
                                 .with_context(|| {
                                     format!("Cannot match query result indices with source code \
                                              for capture name: {name}.")
                                 })?;

                result.insert(name.to_string(), value.to_string());
            }
            results.push(result);
        }

        Ok(results)
    }
}

// Allowed because CustomType is not deprecated, just volatile
#[allow(deprecated)]
impl CustomType for Parser {
    fn build(mut builder: rhai::TypeBuilder<Self>) {
        builder.with_name("JavaParser")
               .with_fn("new_java_parser", Parser::new_script)
               .with_fn("code", Parser::code)
               .with_fn("set_code", Parser::set_code)
               .with_fn("query", Parser::query_mut_script);
    }
}

/// An enum to represent possible errors with a Java file
#[derive(thiserror::Error, Debug)]
pub enum JavaFileError {
    /// An error while compiling a Java file (running
    /// [fn@crate::java::File::check])
    #[error("Something went wrong while compiling the Java file")]
    DuringCompilation {
        /// javac stacktrace
        stacktrace: String,
        /// javac stacktrace, parsed with
        /// [fn@crate::parsers::parser::parse_diag]
        diags:      Vec<JavacDiagnostic>,
    },
    /// An error while running a Java file (running
    /// [fn@crate::java::File::run])
    #[error("Something went wrong while running the Java file")]
    AtRuntime {
        /// java output
        output: String,
        /// java stacktrace, parsed with [parser::junit_stacktrace_line_ref]
        diags:  Vec<LineRef>,
    },
    /// An error while testing a Java file (running
    /// [fn@crate::java::File::test])
    #[error("Something went wrong while testing the Java file")]
    FailedTests {
        /// junit test results
        test_results: String,
        /// junit stacktrace, parsed with [parser::junit_stacktrace_line_ref]
        diags:        Vec<LineRef>,
    },
    /// Unknown error
    #[error(transparent)]
    Unknown(#[from] anyhow::Error),
}

impl File {
    #[generate_rhai_variant(Impl, Fallible)]
    /// Creates a new `File` from `path`
    ///
    /// * `path`: the path to read and try to create a File instance for.
    fn new(path: PathBuf) -> Result<Self> {
        let parser = {
            let source_code = std::fs::read_to_string(&path).with_context(|| {
                                                                format!("Could not read file: {:?}",
                                                                        &path)
                                                            })?;
            Parser::new(source_code)?
        };

        let imports = {
            let imports = parser.query(IMPORT_QUERY)?;
            if imports.is_empty() {
                None
            } else {
                Some(imports)
            }
        };

        let package_name = {
            let package_name = parser.query(PACKAGE_QUERY)?;

            if package_name.is_empty() {
                None
            } else {
                package_name[0].get("name").map(String::to_owned)
            }
        };

        let (kind, name) = 'outer: {
            let work = vec![(FileType::Interface, INTERFACENAME_QUERY),
                            (FileType::ClassWithMain, CLASSNAME_QUERY),
                            (FileType::Class, CLASSNAME_QUERY),];
            for (kind, query) in work {
                let result = parser.query(query)?;

                if !result.is_empty() {
                    break 'outer (kind,
                                  #[allow(clippy::or_fun_call)]
                                  result.first()
                                        .ok_or(anyhow!("Could not find a valid class/interface \
                                                        declaration for {} (vec size is 0)",
                                                       path.display()))?
                                        .get("name")
                                        .ok_or(anyhow!("Could not find a valid class/interface \
                                                        declaration for {} (hashmap has no name \
                                                        key) ",
                                                       path.display()))?
                                        .to_string());
                }
            }

            (FileType::Class, String::new())
        };

        let proper_name = if package_name.is_some() {
            format!("{}.{}", package_name.as_ref().unwrap(), name)
        } else {
            name.clone()
        };

        let test_methods = {
            let test_methods = parser.query(TEST_ANNOTATION_QUERY)?;
            let mut tests = vec![];
            for t in test_methods {
                if let Some(t) = t.get("name") {
                    tests.push(format!("{}#{}", &proper_name, t));
                }
            }

            tests
        };

        let kind = if !test_methods.is_empty() {
            FileType::Test
        } else {
            kind
        };

        let description = match kind {
            FileType::Interface => {
                let empty_dict = Dict::new();
                let empty = String::new();
                let not_found = String::from("[NOT FOUND]");

                let query_result = parser.query(INTERFACE_DECLARATION_QUERY)
                                         .unwrap_or_default();
                let declaration = query_result.first().unwrap_or(&empty_dict);

                let parameters = declaration.get("parameters").unwrap_or(&empty).trim();
                let extends = declaration.get("extends").unwrap_or(&empty).trim();

                let consts = parser.query(INTERFACE_CONSTANTS_QUERY)
                                   .unwrap_or_default()
                                   .iter()
                                   .map(|c| c.get("constant").unwrap_or(&not_found).to_string())
                                   .collect::<Vec<String>>()
                                   .join("\n");

                let methods = parser.query(INTERFACE_METHODS_QUERY)
                                    .unwrap_or_default()
                                    .iter()
                                    .map(|m| m.get("signature").unwrap_or(&not_found).to_string())
                                    .collect::<Vec<String>>()
                                    .join("\n");

                let methods = if methods.trim().is_empty() {
                    String::from("[NOT FOUND]")
                } else {
                    methods.trim().to_string()
                };

                let consts = if consts.trim().is_empty() {
                    String::from("[NOT FOUND]")
                } else {
                    consts.trim().to_string()
                };

                let mut result = vec![];
                result.push(format!("Interface: {proper_name} {parameters} {extends}:\n"));

                if !consts.contains("NOT FOUND") {
                    result.push(String::from("Constants:"));
                    result.push(consts);
                }
                if !methods.contains("NOT FOUND") {
                    result.push(String::from("Methods:"));
                    result.push(methods);
                }

                format!("```\n{r}\n```", r = result.join("\n"))
            }
            _ => {
                let empty_dict = Dict::new();
                let empty = String::new();
                let not_found = String::from("[NOT FOUND]");

                let query_result = parser.query(CLASS_DECLARATION_QUERY).unwrap_or_default();
                let declaration = query_result.first().unwrap_or(&empty_dict);

                let parameters = declaration.get("typeParameters").unwrap_or(&empty).trim();
                let implements = declaration.get("interfaces").unwrap_or(&empty).trim();

                let fields = parser.query(CLASS_FIELDS_QUERY)
                                   .unwrap_or_default()
                                   .iter()
                                   .map(|f| f.get("field").unwrap_or(&not_found).trim().to_string())
                                   .collect::<Vec<String>>()
                                   .join(", ");

                let methods = parser.query(CLASS_METHOD_QUERY)
                                    .unwrap_or_default()
                                    .iter()
                                    .map(|m| {
                                        let identifier =
                                            m.get("identifier").unwrap_or(&not_found).trim();
                                        let parameters = m.get("parameters").unwrap_or(&empty);

                                        if identifier == not_found.as_str() {
                                            "[NOT FOUND]".to_string()
                                        } else {
                                            format!("{}{}", identifier.trim(), parameters.trim())
                                        }
                                    })
                                    .collect::<Vec<String>>()
                                    .join(", ");

                let constructors =
                    parser.query(CLASS_CONSTRUCTOR_QUERY)
                          .unwrap_or_default()
                          .iter()
                          .map(|m| {
                              let identifier = m.get("identifier").unwrap_or(&not_found).trim();
                              let parameters = m.get("parameters").unwrap_or(&empty);

                              if identifier == not_found.as_str() {
                                  "[NOT FOUND]".to_string()
                              } else {
                                  format!("{}{}", identifier.trim(), parameters.trim())
                              }
                          })
                          .collect::<Vec<String>>()
                          .join(", ");

                let fields = if fields.trim().is_empty() {
                    String::from("[NOT FOUND]")
                } else {
                    format!("\tFields: {}", fields)
                };

                let methods = if methods.trim().is_empty() {
                    String::from("[NOT FOUND]")
                } else {
                    format!("\tMethods: {}", methods)
                };

                let constructors = if constructors.trim().is_empty() {
                    String::from("[NOT FOUND]")
                } else {
                    format!("\tConstructors: {}", constructors)
                };

                let mut result = vec![];
                result.push(format!("Class: {proper_name} {parameters} {implements}:\n"));

                if !fields.contains("NOT FOUND") {
                    result.push(fields);
                }
                if !constructors.contains("NOT FOUND") {
                    result.push(constructors);
                }
                if !methods.contains("NOT FOUND") {
                    result.push(methods);
                }

                result.join("\n")
            }
        };

        Ok(Self { path: path.to_owned(),
                  file_name: path.file_name().unwrap().to_str().unwrap().to_string(),
                  package_name,
                  imports,
                  name,
                  test_methods,
                  kind,
                  proper_name,
                  parser,
                  description })
    }

    /// Returns the inner doc check of this [`File`].
    fn inner_doc_check(&self,
                       err: Stdio,
                       out: Stdio,
                       in_: Stdio)
                       -> Result<Output> {
        Command::new(javac_path()?).stderr(err)
                                   .stdout(out)
                                   .stdin(in_)
                                   .args(["--source-path",
                                          sourcepath()?.as_str(),
                                          "-g",
                                          "--class-path",
                                          classpath()?.as_str(),
                                          "-d",
                                          BUILD_DIR.to_str().unwrap(),
                                          self.path.as_path().to_str().unwrap(),
                                          "-Xdiags:verbose",
                                          "-Xdoclint"
                                          /* "-Xlint", */])
                                   .output()
                                   .context("Failed to spawn javac process.")
    }

    /// Utility method to ask javac for documentation lints using the -Xdoclint
    /// flag.
    ///
    /// The method simply returns the output produced by javac as a String.
    /// There is a ['parse_diag method'][fn@crate::parsers::parser::parse_diag]
    /// that can parse these to yield useful information.
    pub fn doc_check(&self) -> Result<String, JavaFileError> {
        let child = self.inner_doc_check(Stdio::piped(), Stdio::piped(), Stdio::piped())?;

        let output = unescape(
            &[
                String::from_utf8(child.stderr).context("Error when parsing stderr as utf8")?,
                String::from_utf8(child.stdout).context("Error when parsing stdout as utf8")?,
            ]
            .concat(),
        )
        .context("Error when un-escaping javac output.")?;

        Ok(output)
    }

    /// Utility method to ask javac for documentation lints using the -Xdoclint
    /// flag.
    ///
    /// The method simply returns the output produced by javac as a String.
    /// There is a ['parse_diag method'][fn@crate::parsers::parser::parse_diag]
    /// that can parse these to yield useful information.
    pub fn doc_check_mut_script(&self) -> Result<String, Box<EvalAltResult>> {
        match self.inner_doc_check(Stdio::inherit(), Stdio::inherit(), Stdio::inherit()) {
            Ok(child) => match unescape(&[match String::from_utf8(child.stderr) {
                                              Ok(s) => s,
                                              Err(e) => {
                                                  return Err(format!("{}", e).into());
                                              }
                                          },
                                          match String::from_utf8(child.stdout) {
                                              Ok(s) => s,
                                              Err(e) => {
                                                  return Err(format!("{}", e).into());
                                              }
                                          }].concat())
            {
                Ok(s) => Ok(s),
                Err(e) => Err(format!("{}", e).into()),
            },
            Err(e) => {
                Err(Box::new(unescape(e.to_string().as_str()).unwrap_or_else(|_| {
                                                                 format!("Could not unescape: {:?}",
                                                                         e)
                                                             })
                                                             .into()))
            }
        }
    }

    /// Returns the inner check of this [`File`].
    fn inner_check(&self,
                   err: Stdio,
                   out: Stdio,
                   in_: Stdio)
                   -> Result<Output> {
        let path = self.path.display().to_string();

        Command::new(javac_path()?).stderr(err)
                                   .stdout(out)
                                   .stdin(in_)
                                   .args(["--source-path",
                                          sourcepath()?.as_str(),
                                          "-g",
                                          "--class-path",
                                          classpath()?.as_str(),
                                          "-d",
                                          BUILD_DIR.to_str().unwrap(),
                                          path.as_str(),
                                          "-Xdiags:verbose",
                                          // "-Xlint",
                                          "-Xprefer:source"])
                                   .output()
                                   .context("Failed to spawn javac process.")
    }

    /// Utility method to check for syntax errors using javac flag.
    pub fn check(&self) -> Result<String, JavaFileError> {
        match self.inner_check(Stdio::piped(), Stdio::piped(), Stdio::piped()) {
            Ok(out) => {
                let output = unescape(
                    &[
                        String::from_utf8(out.stderr).context("Error parsing stderr as utf8")?,
                        String::from_utf8(out.stdout).context("Error parsing stdout as utf8")?,
                    ]
                    .concat(),
                )
                .context("Error when un-escaping javac output.")?;

                if out.status.success() {
                    Ok(output)
                } else {
                    let mut diags = Vec::new();
                    for line in output.lines() {
                        if let Ok(diag) = parser::parse_diag(line) {
                            diags.push(diag);
                        }
                    }

                    Err(JavaFileError::DuringCompilation { stacktrace: output,
                                                           diags })
                }
            }
            Err(e) => Err(JavaFileError::Unknown(e)),
        }
    }

    /// Utility method to check for syntax errors using javac flag.
    pub fn check_mut_script(&self) -> Result<String, Box<EvalAltResult>> {
        match self.inner_check(Stdio::inherit(), Stdio::inherit(), Stdio::inherit()) {
            Ok(child) => match unescape(&[match String::from_utf8(child.stderr) {
                                              Ok(s) => s,
                                              Err(e) => {
                                                  return Err(format!("{}", e).into());
                                              }
                                          },
                                          match String::from_utf8(child.stdout) {
                                              Ok(s) => s,
                                              Err(e) => {
                                                  return Err(format!("{}", e).into());
                                              }
                                          }].concat())
            {
                Ok(s) => Ok(s),
                Err(e) => Err(format!("{}", e).into()),
            },
            Err(e) => {
                Err(Box::new(unescape(e.to_string().as_str()).unwrap_or_else(|_| {
                                                                 format!("Could not unescape: {:?}",
                                                                         e)
                                                             })
                                                             .into()))
            }
        }
    }

    /// Returns the inner run of this [`File`].
    fn inner_run(&self,
                 input: Option<String>,
                 err: Stdio,
                 out: Stdio)
                 -> Result<Output> {
        if self.kind != FileType::ClassWithMain {
            Err(JavaFileError::DuringCompilation { stacktrace: "The file you wish to run does not \
                                                                have a main method."
                                                                                    .into(),
                                                   diags:      vec![], })?;
        }

        if let Some(input_str) = input {
            let mut child = Command::new(java_path()?).args(["--class-path",
                                                             classpath()?.as_str(),
                                                             self.proper_name.clone().as_str()])
                                                      .stdin(Stdio::piped())
                                                      .stdout(out)
                                                      .stderr(err)
                                                      .spawn()
                                                      .context("Failed to spawn javac process.")?;

            let input = format!("{}\r\n", input_str);

            let mut stdin = child.stdin.take().unwrap();

            stdin.write_all(input.as_bytes())
                 .context("Error when trying to write input to stdin")?;
            stdin.flush().context("Error when trying to flush stdin")?;

            child.wait_with_output()
                 .context("Error when waiting for child process to finish")
        } else {
            Command::new(java_path()?).args(["--class-path",
                                             classpath()?.as_str(),
                                             self.proper_name.clone().as_str()])
                                      .stdin(Stdio::inherit())
                                      .stdout(out)
                                      .stderr(err)
                                      .spawn()?
                                      .wait_with_output()
                                      .context("Failed to spawn javac process.")
        }
    }

    /// Utility method to run a java file that has a main method.
    pub fn run(&self,
               input: Option<String>)
               -> Result<String, JavaFileError> {
        self.check()?;

        match self.inner_run(input, Stdio::piped(), Stdio::piped()) {
            Ok(out) => {
                let output = unescape(
                    &[
                        String::from_utf8(out.stderr)
                            .context("Error when parsing stderr as utf8")?,
                        String::from_utf8(out.stdout)
                            .context("Error when parsing stdout as utf8")?,
                    ]
                    .concat(),
                )
                .context("Error when escaping java output.")?;

                if out.status.success() {
                    Ok(output)
                } else {
                    let mut diags = Vec::new();

                    for line in output.lines() {
                        if let Ok(diag) = parser::junit_stacktrace_line_ref(line) {
                            diags.push(diag);
                        }
                    }

                    Err(JavaFileError::AtRuntime { output, diags })
                }
            }
            Err(e) => Err(anyhow!(e).into()),
        }
    }

    /// Utility method to run a java file that has a main method.
    pub fn run_mut_script(&self,
                          input: Option<String>)
                          -> Result<String, Box<EvalAltResult>> {
        match self.inner_run(input, Stdio::inherit(), Stdio::inherit()) {
            Ok(child) => match unescape(&[match String::from_utf8(child.stderr) {
                                              Ok(s) => s,
                                              Err(e) => {
                                                  return Err(format!("{}", e).into());
                                              }
                                          },
                                          match String::from_utf8(child.stdout) {
                                              Ok(s) => s,
                                              Err(e) => {
                                                  return Err(format!("{}", e).into());
                                              }
                                          }].concat())
            {
                Ok(s) => Ok(s),
                Err(e) => Err(format!("{}", e).into()),
            },
            Err(e) => {
                Err(Box::new(unescape(e.to_string().as_str()).unwrap_or_else(|_| {
                                                                 format!("Could not unescape: {:?}",
                                                                         e)
                                                             })
                                                             .into()))
            }
        }
    }

    /// Inner method to run tests.
    fn inner_test(&self,
                  tests: Vec<&str>,
                  err: Stdio,
                  out: Stdio,
                  in_: Stdio)
                  -> Result<Output> {
        let tests = {
            let mut new_tests = Vec::<String>::new();
            for t in tests {
                new_tests.push(format!("{}#{}", self.proper_name.clone(), t));
            }

            if new_tests.is_empty() {
                self.test_methods.clone()
            } else {
                new_tests
            }
        };

        let tests = tests.iter()
                         .map(|s| format!("-m{s}"))
                         .collect::<Vec<String>>();
        let methods: Vec<&str> = tests.iter().map(String::as_str).collect();

        Command::new(java_path().context("Could not find `java` command on path.")?)
            .stderr(err)
            .stdout(out)
            .stdin(in_)
            .args(
                [
                    [
                        "-jar",
                        LIB_DIR.join(JUNIT_PLATFORM).as_path().to_str().unwrap(),
                        "--disable-banner",
                        "--disable-ansi-colors",
                        "--details-theme=unicode",
                        "--single-color",
                        "-cp",
                        &classpath()?,
                    ]
                    .as_slice(),
                    methods.as_slice(),
                ]
                .concat(),
            )
            .output()
            .context("Failed to spawn javac process.")
    }

    /// A utility method that takes a list of strings (or types that implement
    /// `Into<String>`) meant to represent test method names, and runs those
    /// tests.
    ///
    /// Returns the output from JUnit as a string. There are parsers in
    /// ['parsers module'][crate::parsers::parser] that helps parse this output.
    ///
    /// * `tests`: list of strings (or types that implement `Into<String>`)
    ///   meant to represent test method names,
    pub fn test(&self,
                tests: Vec<&str>,
                project: Option<&Project>)
                -> Result<String, JavaFileError> {
        self.check()?;

        match self.inner_test(tests, Stdio::piped(), Stdio::piped(), Stdio::inherit()) {
            Ok(out) => {
                let output = unescape(
                    &[
                        String::from_utf8(out.stderr)
                            .context("Error when parsing stderr as utf8")?,
                        String::from_utf8(out.stdout)
                            .context("Error when parsing stdout as utf8")?,
                    ]
                    .concat(),
                )
                .context("Error when un-escaping JUnit output.")?;

                if out.status.success() {
                    Ok(output)
                } else {
                    let mut diags = Vec::new();
                    let mut new_output = Vec::new();

                    for line in output.lines() {
                        if line.contains("MethodSource") || line.contains("Native Method") {
                            continue;
                        }

                        // if line.contains("Test run finished after") {
                        //     break;
                        // }

                        if let Ok(diag) = parser::junit_stacktrace_line_ref(line) {
                            if let Some(proj) = project
                               && proj.identify(diag.file_name()).is_ok()
                            {
                                new_output.push(line.replace("\\\\", "\\")
                                                    .replace("\\\"", "\"")
                                                    .to_string());
                            }
                            diags.push(diag);
                        } else if let Ok(diag) = parser::parse_diag(line) {
                            if let Some(proj) = project
                               && proj.identify(diag.file_name()).is_ok()
                            {
                                new_output.push(line.replace("\\\\", "\\")
                                                    .replace("\\\"", "\"")
                                                    .to_string());
                            }
                            diags.push(diag.into());
                        } else {
                            new_output.push(line.replace("\\\\", "\\")
                                                .replace("\\\"", "\"")
                                                .to_string());
                        }
                    }

                    Err(JavaFileError::FailedTests { test_results: new_output.join("\n"),
                                                     diags })
                }
            }
            Err(e) => Err(anyhow!(e).into()),
        }
    }

    /// A utility method that takes a list of strings (or types that implement
    /// `Into<String>`) meant to represent test method names, and runs those
    /// tests.
    pub fn test_mut_script(&mut self,
                           tests: Vec<&str>)
                           -> Result<String, Box<EvalAltResult>> {
        match self.inner_test(tests, Stdio::inherit(), Stdio::inherit(), Stdio::inherit()) {
            Ok(child) => match unescape(&[match String::from_utf8(child.stderr) {
                                              Ok(s) => s,
                                              Err(e) => {
                                                  return Err(format!("{}", e).into());
                                              }
                                          },
                                          match String::from_utf8(child.stdout) {
                                              Ok(s) => s,
                                              Err(e) => {
                                                  return Err(format!("{}", e).into());
                                              }
                                          }].concat())
            {
                Ok(s) => Ok(s),
                Err(e) => Err(format!("{}", e).into()),
            },
            Err(e) => {
                Err(Box::new(unescape(e.to_string().as_str()).unwrap_or_else(|_| {
                                                                 format!("Could not unescape: {:?}",
                                                                         e)
                                                             })
                                                             .into()))
            }
        }
    }

    /// A utility method that takes a list of strings (or types that implement
    /// `Into<String>`) meant to represent test method names, and runs those
    /// tests.
    ///
    /// Returns the output from JUnit as a string. There are parsers in
    /// ['parsers module'][crate::parsers::parser] that helps parse this output.

    /// Get a reference to the file's kind.
    pub fn kind(&self) -> &FileType {
        &self.kind
    }

    /// Get a reference to the file's file name.
    pub fn file_name(&self) -> &str {
        self.file_name.as_ref()
    }

    /// Get a reference to the file's test methods.
    pub fn test_methods(&self) -> Vec<String> {
        self.test_methods.clone()
    }

    /// Get a reference to the file's test methods.
    pub fn test_methods_mut_script(&mut self) -> Array {
        self.test_methods().iter().map(|s| s.into()).collect()
    }

    /// treesitter query for this file
    pub fn query(&self,
                 q: &str)
                 -> Result<Vec<Dict>> {
        self.parser.query(q)
    }

    /// treesitter query for this file
    pub fn query_mut_script(&mut self,
                            q: &str)
                            -> Result<Array, Box<EvalAltResult>> {
        match self.parser.query(q) {
            Ok(v) => {
                let mut arr = Array::new();
                for d in v {
                    arr.push(d.into());
                }
                Ok(arr)
            }
            Err(e) => Err(format!("Failed to query file: {e}").into()),
        }
    }

    /// Get a reference to the file's path.
    pub fn path(&self) -> &PathBuf {
        &self.path
    }

    /// Get a reference to the file's path.
    pub fn path_mut_script(&mut self) -> String {
        self.path.display().to_string()
    }

    /// Get a reference to the file's proper name.
    pub fn package_name(&self) -> Option<&String> {
        self.package_name.as_ref()
    }

    /// Get a reference to the file's parser.
    pub fn parser(&self) -> Parser {
        self.parser.clone()
    }

    /// Get a reference to the file's description.
    pub fn description(&self) -> String {
        self.description.clone()
    }

    /// Get the file's proper name.
    pub fn proper_name(&self) -> String {
        self.proper_name.clone()
    }
}

// Allowed because CustomType is not deprecated, just volatile
#[allow(deprecated)]
impl CustomType for File {
    fn build(mut builder: rhai::TypeBuilder<Self>) {
        builder.with_name("JavaFile")
               .with_fn("new_java_file", File::new_script)
               .with_fn("check", File::check_mut_script)
               .with_fn("doc_check", File::doc_check_mut_script)
               .with_fn("run", File::run_mut_script)
               .with_fn("test", File::test_mut_script)
               .with_fn("kind", File::kind)
               .with_fn("file_name", File::file_name)
               .with_fn("test_methods", File::test_methods_mut_script)
               .with_fn("query", File::query_mut_script)
               .with_fn("package_name", File::package_name)
               .with_fn("path", File::path_mut_script)
               .with_fn("parser", File::parser);
    }
}

impl Project {
    #[generate_rhai_variant(Impl, Fallible)]
    /// Initializes a Project, by discovering java files in the
    /// [struct@UMM_DIR] directory. Also downloads some `jar`
    /// files required for unit testing and mutation testing.
    pub fn new() -> Result<Self> {
        let mut files = vec![];
        let mut names = vec![];

        let rt = RUNTIME.handle().clone();
        let handles = FuturesUnordered::new();

        let results = rt.block_on(async {
                            let found_files = match find_files("java", 15, &ROOT_DIR) {
                                Ok(f) => f,
                                Err(e) => panic!("Could not find java files: {e}"),
                            };

                            for path in found_files {
                                handles.push(rt.spawn_blocking(|| File::new(path)))
                            }

                            join_all(handles).await
                        });

        for result in results {
            let file = result??;
            names.push(file.proper_name.clone());
            files.push(file);
        }

        let classpath = vec![LIB_DIR.join("*.jar").display().to_string()];

        let mut sourcepath = vec![SOURCE_DIR.join("").display().to_string(),
                                  TEST_DIR.join("").display().to_string(),];

        if !find_files("java", 0, &ROOT_DIR)?.is_empty() {
            sourcepath.push(ROOT_DIR.join("").display().to_string());
        }

        let proj = Self { files,
                          names,
                          classpath,
                          sourcepath,
                          root_dir: ROOT_DIR.display().to_string() };

        let _guard = rt.enter();
        rt.block_on(async {
              let handles = FuturesUnordered::new();
              let (proj1, proj2, proj3) = (proj.clone(), proj.clone(), proj.clone());

              handles.push(tokio::spawn(async move { proj1.download_libraries_if_needed().await }));
              handles.push(tokio::spawn(async move { proj2.update_vscode_settings().await }));
              handles.push(tokio::spawn(async move { proj3.update_vscode_tasks().await }));

              try_join_all(handles).await
          })?
          .into_iter()
          .collect::<Result<Vec<()>>>()?;

        Ok(proj)
    }

    #[generate_rhai_variant(Impl, Mut, Fallible)]
    /// Attempts to identify the correct file from the project from a partial or
    /// fully formed name as expected by a java compiler.
    ///
    /// Returns a reference to the identified file, if any.
    ///
    /// * `name`: partial/fully formed name of the Java file to look for.
    pub fn identify(&self,
                    name: &str)
                    -> Result<File> {
        let name: String = name.into();

        if let Some(i) = self.names.iter().position(|n| *n == name) {
            Ok(self.files[i].clone())
        } else if let Some(i) = self.files.iter().position(|n| n.file_name == name) {
            Ok(self.files[i].clone())
        } else if let Some(i) = self.files
                                    .iter()
                                    .position(|n| n.file_name.replace(".java", "") == name)
        {
            Ok(self.files[i].clone())
        } else if let Some(i) = self.files.iter().position(|n| n.name.clone() == name) {
            Ok(self.files[i].clone())
        } else if let Some(i) = self.files
                                    .iter()
                                    .position(|n| n.path.display().to_string() == name)
        {
            Ok(self.files[i].clone())
        } else if let Some(i) = self.files.iter().position(|n| n.proper_name == name) {
            Ok(self.files[i].clone())
        } else {
            bail!("Could not find {} in the project", name)
        }
    }

    /// Returns true if project contains a file with the given name.
    pub fn contains(&self,
                    name: &str)
                    -> bool {
        self.identify(name).is_ok()
    }

    /// Downloads certain libraries like JUnit if found in imports.
    /// times out after 20 seconds.
    pub async fn download_libraries_if_needed(&self) -> Result<()> {
        let need_junit = 'outer: {
            for file in self.files.iter() {
                if let Some(imports) = &file.imports {
                    for import in imports {
                        if let Some(path) = import.get(&String::from("path")) {
                            if path.starts_with("org.junit") {
                                break 'outer true;
                            }
                        }
                    }
                }
            }
            false
        };

        if need_junit {
            if !LIB_DIR.as_path().is_dir() {
                std::fs::create_dir(LIB_DIR.as_path()).unwrap();
            }

            let handle1 = tokio::spawn(async {
                download(
                    "https://ummfiles.fra1.digitaloceanspaces.com/jar_files/junit-platform-console-standalone-1.10.2.jar",
                    &LIB_DIR.join(JUNIT_PLATFORM),
                false
                        )
                        .await
            });

            let handle2 = tokio::spawn(async {
                download("https://ummfiles.fra1.digitaloceanspaces.com/jar_files/junit-4.13.2.jar",
                         &LIB_DIR.join("junit-4.13.2.jar"),
                         false).await
            });

            let handle3 = tokio::spawn(async {
                download("https://ummfiles.fra1.digitaloceanspaces.com/jar_files/pitest-1.16.1.jar",
                         &LIB_DIR.join("pitest.jar"),
                         false).await
            });

            let handle4 = tokio::spawn(async {
                download(
                        "https://ummfiles.fra1.digitaloceanspaces.com/jar_files/pitest-command-line-1.16.1.jar",
                        &LIB_DIR.join("pitest-command-line.jar"),
                        false,
                    )
                    .await
            });

            let handle5 = tokio::spawn(async {
                download(
                    "https://ummfiles.fra1.digitaloceanspaces.com/jar_files/pitest-entry-1.16.1.jar",
                    &LIB_DIR.join("pitest-entry.jar"),
                    false,
                )
                .await
            });

            let handle6 = tokio::spawn(async {
                download(
                        "https://ummfiles.fra1.digitaloceanspaces.com/jar_files/pitest-junit5-plugin-1.2.1.jar",
                        &LIB_DIR.join("pitest-junit5-plugin.jar"),
                        false,
                    )
                    .await
            });

            let handle7 = tokio::spawn(async {
                download(
                        "https://ummfiles.fra1.digitaloceanspaces.com/jar_files/commons-text-1.12.0.jar",
                        &LIB_DIR.join("commons-text-1.12.0.jar"),
                        false,
                    )
                    .await
            });

            let handle8 = tokio::spawn(async {
                download(
                        "https://ummfiles.fra1.digitaloceanspaces.com/jar_files/commons-lang3-3.14.0.jar",
                        &LIB_DIR.join("commons-lang3-3.14.0.jar"),
                        false,
                    )
                    .await
            });

            let handles = FuturesUnordered::from_iter([handle1, handle2, handle3, handle4,
                                                       handle5, handle6, handle7, handle8]);

            futures::future::try_join_all(handles).await?;
        }
        Ok(())
    }

    /// Creates a vscode settings.json file for the project.
    pub async fn update_vscode_settings(&self) -> Result<()> {
        // TODO: Move this to an init function that takes CONSTANTS into account
        if !ROOT_DIR.join(".vscode").as_path().is_dir() {
            tokio::fs::create_dir(ROOT_DIR.join(".vscode").as_path()).await
                                                                     .unwrap();
        }

        if !ROOT_DIR.join(".vscode/settings.json").as_path().exists() {
            let mut file = tokio::fs::OpenOptions::new().write(true)
                                                        .truncate(true)
                                                        .create(true)
                                                        .open(ROOT_DIR.join(".vscode")
                                                                      .join("settings.json")
                                                                      .as_path())
                                                        .await?;

            let settings =
                vscode::SettingsFile::builder().java_source_path(self.sourcepath.clone())
                                               .java_output_path(BUILD_DIR.join("")
                                                                          .display()
                                                                          .to_string())
                                               .java_referenced_libs(self.classpath.clone())
                                               .umm_binary_path(umm_path())
                                               .build();

            file.write_all(serde_json::to_string_pretty(&settings)?.as_bytes())
                .await?;
        }

        // Do the same for extensions.json
        if !ROOT_DIR.join(".vscode/extensions.json").as_path().exists() {
            let mut file = tokio::fs::OpenOptions::new().write(true)
                                                        .truncate(true)
                                                        .create(true)
                                                        .open(ROOT_DIR.join(".vscode")
                                                                      .join("extensions.json")
                                                                      .as_path())
                                                        .await?;

            let extensions = r#"
{
	// See https://go.microsoft.com/fwlink/?LinkId=827846 to learn about workspace recommendations.
	// Extension identifier format: ${publisher}.${name}. Example: vscode.csharp

	// List of extensions which should be recommended for users of this workspace.
	"recommendations": [
        "vscjava.vscode-java-pack",
        "ms-vsliveshare.vsliveshare"
	],
	// List of extensions recommended by VS Code that should not be recommended for users of this workspace.
	"unwantedRecommendations": [
		
	]
}
            "#;

            file.write_all(extensions.as_bytes()).await?;
        }

        Ok(())
    }

    /// Get a reference to the project's files.
    pub fn files(&self) -> &[File] {
        self.files.as_ref()
    }

    #[generate_rhai_variant(Fallible)]
    /// Prints project struct as a json
    pub fn info(&self) -> Result<()> {
        println!("{}", serde_json::to_string(&self)?);
        Ok(())
    }

    /// Returns a short summary of the project, it's files, their fields and
    /// methods.
    pub fn describe(&self) -> String {
        let mut result = String::new();
        result.push_str("> What follows is a summary of the student's submission's files, their \
                         fields and methods generated via treesitter queries.\n\n");

        for f in self.files.iter() {
            if f.proper_name.contains("Hidden") {
                continue;
            }
            result.push_str(f.description().as_str());
            result.push_str("\n\n");
        }

        result
    }

    /// Writes a .vscode/tasks.json file for the project.
    pub async fn update_vscode_tasks(&self) -> Result<()> {
        let mut tasks = Vec::new();
        let mut inputs = Vec::new();

        let (default_depends_on, default_depends_order) = if umm_path() == "./umm" {
            (Some(vec!["Set umm to be executable".to_string()]),
             Some(vscode::DependsOrder::Sequence))
        } else {
            (None, None)
        };

        tasks.push(
                   vscode::Task::builder().label("Set umm to be executable".to_string())
                                          .r#type(vscode::Type::Shell)
                                          .command("chmod")
                                          .args(vec![
            vscode::Args::builder().value("+x")
                                   .quoting(vscode::ArgQuoting::Escape)
                                   .build(),
            vscode::Args::builder().value("${config:ummBinaryPath}")
                                   .quoting(vscode::ArgQuoting::Weak)
                                   .build(),
        ])
                                          .depends_on(None)
                                          .depends_order(None)
                                          .build(),
        );
        tasks.push(
                   vscode::Task::builder().label("Clean library and target folders".to_string())
                                          .r#type(vscode::Type::Shell)
                                          .command("${config:ummBinaryPath}")
                                          .args(vec![vscode::Args::builder().value("clean")
                                                                            .quoting(
            vscode::ArgQuoting::Escape,
        )
                                                                            .build()])
                                          .depends_on(default_depends_on.clone())
                                          .depends_order(default_depends_order)
                                          .build(),
        );

        tasks.push(
                   vscode::Task::builder().label("Reset project metadata".into())
                                          .r#type(vscode::Type::Shell)
                                          .command("${config:ummBinaryPath}")
                                          .args(vec![vscode::Args::builder().value("reset")
                                                                            .quoting(
            vscode::ArgQuoting::Escape,
        )
                                                                            .build()])
                                          .depends_on(default_depends_on.clone())
                                          .depends_order(default_depends_order)
                                          .build(),
        );

        tasks.push(
                   vscode::Task::builder().label("Check health of the project".into())
                                          .r#type(vscode::Type::Shell)
                                          .command("${config:ummBinaryPath}")
                                          .args(vec![vscode::Args::builder().value("check-health")
                                                                            .quoting(
            vscode::ArgQuoting::Escape,
        )
                                                                            .build()])
                                          .depends_on(default_depends_on.clone())
                                          .depends_order(default_depends_order)
                                          .build(),
        );

        tasks.push(
                   vscode::Task::builder().label("Update umm executable".into())
                                          .r#type(vscode::Type::Shell)
                                          .command("${config:ummBinaryPath}")
                                          .args(vec![vscode::Args::builder().value("update")
                                                                            .quoting(
            vscode::ArgQuoting::Escape,
        )
                                                                            .build()])
                                          .depends_on(default_depends_on.clone())
                                          .depends_order(default_depends_order)
                                          .build(),
        );

        for file in self.files().iter() {
            match file.kind() {
                FileType::ClassWithMain => {
                    tasks.push(
                               vscode::Task::builder().label(format!("Run {}", file.name))
                                                      .r#type(vscode::Type::Shell)
                                                      .command("${config:ummBinaryPath}")
                                                      .args(vec![
                        vscode::Args::builder().value("run")
                                               .quoting(vscode::ArgQuoting::Escape)
                                               .build(),
                        vscode::Args::builder().value(&file.proper_name)
                                               .quoting(vscode::ArgQuoting::Escape)
                                               .build(),
                    ])
                                                      .depends_on(default_depends_on.clone())
                                                      .depends_order(default_depends_order)
                                                      .build(),
                    );
                }
                FileType::Test => {
                    tasks.push(
                               vscode::Task::builder().label(format!(
                        "Run tests for {}",
                        file.name
                    ))
                                                      .r#type(vscode::Type::Shell)
                                                      .command("${config:ummBinaryPath}")
                                                      .args(vec![
                        vscode::Args::builder().value("test")
                                               .quoting(vscode::ArgQuoting::Escape)
                                               .build(),
                        vscode::Args::builder().value(&file.proper_name)
                                               .quoting(vscode::ArgQuoting::Escape)
                                               .build(),
                    ])
                                                      .group("test".to_string())
                                                      .depends_on(default_depends_on.clone())
                                                      .depends_order(default_depends_order)
                                                      .build(),
                    );

                    let mut test_methods = Vec::new();

                    for method in file.test_methods() {
                        let method = method.clone();
                        #[allow(clippy::or_fun_call)]
                        let method =
                            method.split_once('#')
                                  .ok_or(anyhow!("Could not parse test method - {}", method))?
                                  .1;
                        // commands.push(method.into());
                        test_methods.push(String::from(method));
                    }

                    if !test_methods.is_empty() {
                        let input = vscode::Input::PickString { id:          file.proper_name
                                                                                 .to_string(),
                                                                description:
                                                                    "Which test to run?".to_string(),
                                                                options:     test_methods.clone(),
                                                                default:     test_methods.first()
                                                                                         .unwrap()
                                                                                         .clone(), };
                        inputs.push(input);
                    }

                    tasks.push(
                               vscode::Task::builder().label(format!(
                        "Run specific test from {}",
                        file.name
                    ))
                                                      .r#type(vscode::Type::Shell)
                                                      .command("${config:ummBinaryPath}")
                                                      .args(vec![
                        vscode::Args::builder().value("test")
                                               .quoting(vscode::ArgQuoting::Escape)
                                               .build(),
                        vscode::Args::builder().value(&file.proper_name)
                                               .quoting(vscode::ArgQuoting::Escape)
                                               .build(),
                        vscode::Args::builder().value(format!("${{input:{}}}", file.proper_name))
                                               .quoting(vscode::ArgQuoting::Escape)
                                               .build(),
                    ])
                                                      .group("test".to_string())
                                                      .depends_on(default_depends_on.clone())
                                                      .depends_order(default_depends_order)
                                                      .build(),
                    );
                }
                _ => {}
            };

            tasks.push(
                       vscode::Task::builder().label(format!("Check {}", file.name))
                                              .r#type(vscode::Type::Shell)
                                              .command("${config:ummBinaryPath}")
                                              .args(vec![
                vscode::Args::builder().value("check")
                                       .quoting(vscode::ArgQuoting::Escape)
                                       .build(),
                vscode::Args::builder().value(&file.proper_name)
                                       .quoting(vscode::ArgQuoting::Escape)
                                       .build(),
            ])
                                              .depends_on(default_depends_on.clone())
                                              .depends_order(default_depends_order)
                                              .build(),
            );
            tasks.push(
                       vscode::Task::builder().label(format!("Check JavaDoc for {}", file.name))
                                              .r#type(vscode::Type::Shell)
                                              .command("${config:ummBinaryPath}")
                                              .args(vec![
                vscode::Args::builder().value("doc-check")
                                       .quoting(vscode::ArgQuoting::Escape)
                                       .build(),
                vscode::Args::builder().value(&file.proper_name)
                                       .quoting(vscode::ArgQuoting::Escape)
                                       .build(),
            ])
                                              .depends_on(default_depends_on.clone())
                                              .depends_order(default_depends_order)
                                              .build(),
            );
        }

        let rhai_scripts = {
            let scripts = find_files(".rhai", 3, &ROOT_DIR)?.iter()
                                                            .map(|f| f.display().to_string())
                                                            .collect::<Vec<String>>();

            if scripts.is_empty() {
                vec!["script.rhai".to_string()]
            } else {
                scripts
            }
        };

        inputs.push(vscode::Input::PickString { id:          "gradable_assignments".to_string(),
                                          description: "What script to use?".to_string(),
                                          options:     rhai_scripts.clone(),
                                          default:     rhai_scripts.first().unwrap().clone(), });

        tasks.push(
            vscode::Task::builder()
                .label("Grade Assignment".to_string())
                .r#type(vscode::Type::Shell)
                .command("${config:ummBinaryPath}")
                .args(vec![
                    vscode::Args::builder()
                        .value("grade")
                        .quoting(vscode::ArgQuoting::Escape)
                        .build(),
                    vscode::Args::builder()
                        .value("${input:gradable_assignments}".to_string())
                        .quoting(vscode::ArgQuoting::Escape)
                        .build(),
                ])
                .problem_matcher(Some(vec![vscode::ProblemMatcher::builder()
                    .apply_to("allDocuments".to_string())
                    .file_location(vec![
                        "relative".to_string(),
                        "${workspaceFolder}".to_string(),
                    ])
                    .owner("umm".to_string())
                    .pattern(
                        vscode::Pattern::builder()
                            .regexp(r#"\s*[│]\s*([\w./]+)\s*[│]\s*([0-9]+)\s*[│]\s*([\w ]+)"#)
                            .file(1)
                            .line(2)
                            .end_line(2)
                            .message(3)
                            .build(),
                    )
                    .build()]))
                .depends_on(default_depends_on)
                .depends_order(default_depends_order)
                .build(),
        );

        if !ROOT_DIR.join(".vscode").as_path().exists() {
            tokio::fs::create_dir(ROOT_DIR.join(".vscode").as_path()).await
                                                                     .unwrap();
        }

        let mut file =
            tokio::fs::OpenOptions::new().write(true)
                                         .truncate(true)
                                         .create(true)
                                         .open(ROOT_DIR.join(".vscode")
                                                       .join("tasks.json")
                                                       .as_path())
                                         .await?;

        let task_file = vscode::TasksFile::builder().tasks(tasks)
                                                    .inputs(inputs)
                                                    .build();

        file.write_all(serde_json::to_string_pretty(&task_file)?.as_bytes())
            .await?;

        Ok(())
    }

    /// Serves the project code as a static website.
    pub fn serve_project_code(&self) -> anyhow::Result<()> {
        let mut markdown = format!("# Student Submission Source Code\n\n## Overview\n\n{}\n\n## \
                                    Source Code\n\n",
                                   self.describe());

        for file in &self.files {
            markdown.push_str(&format!("### {}\n\n```java\n{}\n```\n\n",
                                       file.proper_name(),
                                       file.parser().code()));
        }
        let id = uuid::Uuid::new_v4().to_string();
        let submission = serde_json::to_string(&SubmissionRow { id:      id.clone(),
                                                                course:  COURSE.to_string(),
                                                                term:    TERM.to_string(),
                                                                content: markdown, })?;

        let rt = RUNTIME.handle().clone();
        rt.block_on(async {
              POSTGREST_CLIENT.from("submissions")
                              .insert(submission)
                              .execute()
                              .await
          })?;

        println!("Please visit https://feedback.dhruvdh.com/submissions/{} to see your \
                  submission code.",
                 id);

        Ok(())
    }
}

/// Schema for `submissions` table
#[derive(Serialize, Debug)]
pub struct SubmissionRow {
    /// UUID of data entry
    id:      String,
    /// Course the submission belongs to
    course:  String,
    /// Term of the course
    term:    String,
    /// Content of the submission
    content: String,
}

// Allowed because CustomType is not deprecated, just volatile
#[allow(deprecated)]
impl CustomType for Project {
    fn build(mut builder: rhai::TypeBuilder<Self>) {
        builder.with_name("JavaProject")
               .with_fn("new_java_project", Project::new_script)
               .with_fn("identify", Project::identify_mut_script)
               .with_fn("files", Project::files)
               .with_fn("info", Project::info_script);
    }
}
