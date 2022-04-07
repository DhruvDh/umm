use std::{
    path::PathBuf,
    process::{Command, Stdio},
};

use anyhow::{bail, ensure, Context, Result};
use colored::Colorize;
use serde::{Deserialize, Serialize};
use tree_sitter::{Query, QueryCursor, Tree};

use crate::{constants::*, util::*, Dict};

/// Types of Java files -
/// - Interface
/// - Class
/// - Class with a main method
/// - JUnit test class
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum JavaFileType {
    Interface,
    Class,
    ClassWithMain,
    Test,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
/// Struct representing a java file
///
/// * `path`: path to java file
/// * `file_name`: name of file
/// * `package_name`: package the java file belongs to
/// * `imports`: imports made by the java file
/// * `name`: name of the file TODO: How does this differ from `file_name`?
/// * `pretty_name`: colored terminal string representing java file name
/// * `proper_name`: proper name of the file as understood by the java compiler
/// * `test_methods`: Name of tests methods in this file, as understood by JUnit
/// * `pretty_test_methods`: Name of tests methods in this file, colored using terminal color codes
/// * `kind`: `JavaFileType` variant for this java file
/// * `source_code`: Source code as a string for this java file
pub struct File {
    path: PathBuf,
    pub file_name: String,
    package_name: Option<String>,
    imports: Option<Vec<Dict>>,
    /// TODO: How does this differ from `file_name`?
    pub name: Option<String>,
    pretty_name: Option<String>,
    pub proper_name: Option<String>,
    pub test_methods: Vec<String>,
    pretty_test_methods: Vec<String>,
    kind: JavaFileType,
    source_code: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
/// Struct representing a Java project.
/// Any index `i` in any collection in this struct always refers to the same JavaFile.
///
/// * `files`: Collection of java files in this project
/// * `pretty_names`: Names of java files in this projects, colored using terminal color codes
/// * `names`: Names of java files in this project.
pub struct Project {
    pub files: Vec<File>,
    pretty_names: Vec<String>,
    pub names: Vec<String>,
}

/// A struct that wraps a tree-sitter parser object and source code
///
/// TODO: The source code should not be in here, extract it out
///
/// * `code`: the source code being parsed
/// * `_parser`: the tree-sitter parser object
/// * `_tree`: the parse tree
/// * `lang`: the tree-sitter java grammar language
pub struct Parser {
    code: String,
    _parser: tree_sitter::Parser,
    _tree: Tree,
    lang: tree_sitter::Language,
}

impl Parser {
    /// Returns a new parser object
    ///
    /// * `source_code`: the source code to be parsed
    /// * `lang`: the tree-sitter grammar to use
    pub fn new(source_code: String, lang: tree_sitter::Language) -> Result<Self> {
        let mut parser = tree_sitter::Parser::new();

        parser
            .set_language(lang)
            .expect("Error loading Java grammar");
        let tree = parser
            .parse(source_code.clone(), None)
            .context("Error parsing Java code")?;

        Ok(Self {
            code: source_code,
            _parser: parser,
            _tree: tree,
            lang,
        })
    }

    /// Applies a tree sitter query and returns the result as a collection of HashMaps
    ///
    /// * `q`: the tree-sitter query to be applied
    pub fn query(&self, q: &str) -> Result<Vec<Dict>> {
        let mut results = vec![];

        let query = Query::new(self.lang, q).unwrap();

        let mut cursor = QueryCursor::new();
        let matches = cursor.matches(&query, self._tree.root_node(), self.code.as_bytes());
        let capture_names = query.capture_names();

        for m in matches {
            let mut result = Dict::new();

            for name in capture_names {
                let index = query.capture_index_for_name(name);
                let index = match index {
                        Some(i) => i,
                        None => bail!("Error while querying source code. Capture name: {} has no index associated.",
                        name),
                    };

                let value = m.captures.iter().find(|c| c.index == index);
                let value = match value {
                    Some(v) => v,
                    None => continue,
                };

                let value = value
                        .node
                        .utf8_text(self.code.as_bytes())
                        .with_context(|| {
                            format!(
                            "Cannot match query result indices with source code for capture name: {}.",
                            name
                        )
                        })?;

                result.insert(name.clone(), value.to_string());
            }
            results.push(result);
        }

        Ok(results)
    }
}
impl File {
    fn new(path: PathBuf) -> Result<Self> {
        let source_code = std::fs::read_to_string(&path)
            .with_context(|| format!("Could not read file: {:?}", &path))?;

        let parser = Parser::new(source_code.clone(), *JAVA_TS_LANG)?;

        let imports = parser.query(IMPORT_QUERY)?;
        let imports = if imports.is_empty() {
            None
        } else {
            Some(imports)
        };
        let _package_name = parser.query(PACKAGE_QUERY)?;

        ensure!(
            _package_name.len() == 1 || _package_name.is_empty(),
            "Expected 0 or 1 package declaration statements, found {}.",
            _package_name.len()
        );

        let package_name = if _package_name.is_empty() {
            None
        } else {
            _package_name[0].get("name").map(String::to_owned)
        };

        let mut kind = JavaFileType::Class;
        let name = {
            let class = parser.query(CLASSNAME_QUERY)?;
            if class.is_empty() {
                kind = JavaFileType::Interface;
                parser.query(INTERFACENAME_QUERY)?
            } else {
                class
            }
        };

        let main_method_result = parser.query(MAIN_METHOD_QUERY)?;

        ensure!(
            main_method_result.len() <= 1,
            "Number of main methods should be 0 or 1."
        );
        if !main_method_result.is_empty() {
            kind = JavaFileType::ClassWithMain;
        }

        ensure!(
            name.len() == 1,
            "For file: {} Expected exactly one class/interface name, found {}.",
            path.as_path().display(),
            name.len()
        );

        let name = name[0].get("name").map(String::to_owned);
        let pretty_name = if package_name.is_some() {
            format!(
                "{}.{}",
                package_name.as_ref().unwrap().bright_yellow().bold(),
                name.as_ref().unwrap().bright_blue().bold()
            )
        } else {
            format!("{}", name.as_ref().unwrap().blue())
        };

        let proper_name = if package_name.is_some() {
            format!(
                "{}.{}",
                package_name.as_ref().unwrap(),
                name.as_ref().unwrap()
            )
        } else {
            name.as_ref().unwrap().to_string()
        };

        let test_methods = parser.query(TEST_ANNOTATION_QUERY)?;

        let mut pretty_test_methods = vec![];
        for test_method in test_methods.clone() {
            let method_name = test_method
                .get("name")
                .map(String::to_owned)
                .unwrap_or_default();

            let method_name = if method_name.starts_with("test") {
                let method_name = method_name.replace("test", "");
                let method_name = method_name.bright_green().bold();
                format!("test{}", method_name)
            } else {
                method_name.bright_green().bold().to_string()
            };

            pretty_test_methods.push(method_name);
        }

        let test_methods = {
            let mut tests = vec![];
            for t in test_methods {
                if let Some(t) = t.get("name") {
                    tests.push(format!("{}#{}", &proper_name, t));
                }
            }

            if !tests.is_empty() {
                kind = JavaFileType::Test;
            }
            tests
        };

        Ok(Self {
            path: path.to_owned(),
            file_name: path.file_name().unwrap().to_str().unwrap().to_string(),
            package_name,
            imports,
            pretty_name: Some(pretty_name),
            name,
            test_methods,
            pretty_test_methods,
            kind,
            proper_name: Some(proper_name),
            source_code,
        })
    }

    pub fn doc_check(&self) -> Result<String> {
        let child = Command::new(javac_path()?)
            .args([
                "--source-path",
                SOURCE_DIR.to_str().unwrap(),
                "-g",
                "--class-path",
                classpath()?.as_str(),
                "-d",
                BUILD_DIR.to_str().unwrap(),
                self.path.as_path().to_str().unwrap(),
                "-Xdiags:verbose",
                "-Xdoclint",
                // "-Xlint",
                "-Xprefer:source",
            ])
            .output()
            .context("Failed to spawn javac process.")?;

        let output = String::from_utf8(child.stderr)? + &String::from_utf8(child.stdout)?;

        Ok(output)
    }

    pub fn check(&self) -> Result<()> {
        let path = self.path.display().to_string();

        let child = Command::new(javac_path()?)
            .stdin(Stdio::inherit())
            .stdout(Stdio::inherit())
            .stderr(Stdio::inherit())
            .args([
                "--source-path",
                SOURCE_DIR.to_str().unwrap(),
                "-g",
                "--class-path",
                classpath()?.as_str(),
                "-d",
                BUILD_DIR.to_str().unwrap(),
                path.as_str(),
                "-Xdiags:verbose",
                // "-Xlint",
                "-Xprefer:source",
            ])
            .spawn()
            .context("Failed to spawn javac process.")?;

        match child.wait_with_output() {
            Ok(status) => {
                if status.status.success() {
                    println!(
                        "{}",
                        "No compiler errors in checked file or other source files it imports."
                            .bright_green()
                            .bold(),
                    );
                } else {
                    bail!("There were compiler errors in checked file or other source files it imports.".bright_red().bold());
                }
            }
            Err(e) => bail!(
                "Failed to wait for child process for {}: {}",
                self.proper_name.clone().unwrap(),
                e
            ),
        };
        Ok(())
    }

    pub fn run(&self) -> Result<()> {
        self.check()?;
        let name = self.proper_name.clone().unwrap();
        let child = Command::new(java_path()?)
            .stdin(Stdio::inherit())
            .stdout(Stdio::inherit())
            .stderr(Stdio::inherit())
            .args(["--class-path", classpath()?.as_str(), name.as_str()])
            .spawn()
            .context("Failed to spawn javac process.")?;

        match child.wait_with_output() {
            Ok(status) => {
                if status.status.success() {
                    println!("{}", "Ran and exited successfully.".bright_green().bold(),);
                } else {
                    println!("{}", "Ran but exited unsuccessfully.".bright_red().bold(),);
                }
            }
            Err(e) => bail!("Failed to wait for child process for {}: {}", name, e),
        };

        Ok(())
    }

    pub fn test<T: Into<String>>(&self, tests: Vec<T>) -> Result<String> {
        self.check()?;
        let tests = {
            let mut new_tests = Vec::<String>::new();
            for t in tests {
                new_tests.push(format!(
                    "{}#{}",
                    self.proper_name
                        .clone()
                        .unwrap_or(self.name.clone().unwrap_or("".into())),
                    t.into()
                ));
            }

            if new_tests.is_empty() {
                self.test_methods.clone()
            } else {
                new_tests
            }
        };

        let tests = tests
            .iter()
            .map(|s| "-m ".to_owned() + s)
            .collect::<Vec<String>>();
        let methods: Vec<&str> = tests.iter().map(String::as_str).collect();

        let child = Command::new(java_path()?)
            // .stdin(Stdio::inherit())
            // .stdout(Stdio::inherit())
            // .stderr(Stdio::inherit())
            .args(
                [
                    [
                        "-jar",
                        ROOT_DIR
                            .join("lib/junit-platform-console-standalone-1.8.0-RC1.jar")
                            .as_path()
                            .to_str()
                            .unwrap(),
                        "--disable-banner",
                        "--reports-dir",
                        "test_reports",
                        "--details",
                        "tree",
                        "-cp",
                        &classpath()?,
                    ]
                    .as_slice(),
                    methods.as_slice(),
                ]
                .concat(),
            )
            .output()
            .context("Could not issue java command to run the tests for some reason.")?;

        if child.status.success() {
            println!("{}", "Ran and exited successfully.".bright_green().bold(),);
        } else {
            println!("{}", "Ran but exited unsuccessfully.".bright_red().bold(),);
        }
        let output = String::from_utf8(child.stderr)? + &String::from_utf8(child.stdout)?;

        Ok(output)
    }

    /// Get a reference to the file's kind.
    pub fn kind(&self) -> &JavaFileType {
        &self.kind
    }

    /// Get a reference to the file's file name.
    pub fn file_name(&self) -> &str {
        self.file_name.as_ref()
    }

    /// Get a reference to the file's test methods.
    pub fn test_methods(&self) -> &[String] {
        self.test_methods.as_ref()
    }
}

impl Project {
    pub fn new() -> Result<Self> {
        let mut files = vec![];
        let mut pretty_names = vec![];
        let mut names = vec![];

        download(
        "https://github.com/DhruvDh/umm/blob/next-assign1-spring-22/jar_files/DataStructures.jar?raw=true",
        &LIB_DIR.join("DataStructures.jar"),
    false)?;
        download(
        "https://github.com/DhruvDh/umm/blob/next-assign1-spring-22/jar_files/junit-platform-console-standalone-1.8.0-RC1.jar?raw=true",
        &LIB_DIR.join("junit-platform-console-standalone-1.8.0-RC1.jar"),
false    )?;
        download(
        "https://github.com/DhruvDh/umm/blob/next-assign1-spring-22/jar_files/pitest-1.7.4.jar?raw=true",
        &LIB_DIR.join("pitest.jar"),
    false)?;
        download(
        "https://github.com/DhruvDh/umm/blob/next-assign1-spring-22/jar_files/pitest-command-line-1.7.4.jar?raw=true",
        &LIB_DIR.join("pitest-command-line.jar"),
    false)?;
        download(
        "https://github.com/DhruvDh/umm/blob/next-assign1-spring-22/jar_files/pitest-entry-1.7.4.jar?raw=true",
        &LIB_DIR.join("pitest-entry.jar"),
    false)?;
        download(
        "https://github.com/DhruvDh/umm/blob/next-assign1-spring-22/jar_files/pitest-junit5-plugin-0.14.jar?raw=true",
        &LIB_DIR.join("pitest-junit5-plugin.jar"),
   false )?;

        println!(
            "Discovering project at {}",
            std::fs::canonicalize(ROOT_DIR.as_path())?.display()
        );

        for path in find_files("java", 15, &ROOT_DIR)? {
            let file = File::new(path)?;
            pretty_names.push(file.pretty_name.clone().unwrap());
            names.push(file.proper_name.clone().unwrap());
            files.push(file);
        }

        Ok(Self {
            files,
            pretty_names,
            names,
        })
    }

    pub fn identify<T: Into<String>>(&self, name: T) -> Result<File> {
        let name: String = name.into();

        if let Some(i) = self.names.iter().position(|n| *n == name) {
            Ok(self.files[i].clone())
        } else if let Some(i) = self.files.iter().position(|n| n.file_name == name) {
            Ok(self.files[i].clone())
        } else if let Some(i) = self
            .files
            .iter()
            .position(|n| n.name.clone().unwrap() == name)
        {
            Ok(self.files[i].clone())
        } else if let Some(i) = self
            .files
            .iter()
            .position(|n| n.path.display().to_string() == name)
        {
            Ok(self.files[i].clone())
        } else {
            bail!("Could not find {} in the project", name)
        }
    }
}
