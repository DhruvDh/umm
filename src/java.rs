#![warn(missing_docs)]
#![warn(clippy::missing_docs_in_private_items)]

use std::{
    io::Write,
    path::PathBuf,
    process::{
        Command,
        Stdio,
    },
};

use anyhow::{
    anyhow,
    bail,
    ensure,
    Context,
    Result,
};
use colored::Colorize;
use rhai::EvalAltResult;
use serde::{
    Deserialize,
    Serialize,
};
use tree_sitter::{
    Query,
    QueryCursor,
    Tree,
};
use umm_derive::generate_rhai_variant;

use crate::{
    constants::*,
    util::*,
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
}

#[derive(Debug, Clone, Serialize, Deserialize)]
/// Struct representing a Java project.
/// Any index `i` in any collection in this struct always refers to the same
/// JavaFile.
pub struct Project {
    /// Collection of java files in this project
    files:     Vec<File>,
    /// Names of java files in this project.
    names:     Vec<String>,
    /// Classpath
    classpath: String,
}

/// A struct that wraps a tree-sitter parser object and source code
///
/// TODO: The source code should not be in here, extract it out
pub struct Parser {
    /// the source code being parsed
    code:    String,
    /// the tree-sitter parser object
    _parser: tree_sitter::Parser,
    /// the parse tree
    _tree:   Tree,
    /// the tree-sitter java grammar language
    lang:    tree_sitter::Language,
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

    /// Applies a tree sitter query and returns the result as a collection of
    /// HashMaps
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
                    None => bail!(
                        "Error while querying source code. Capture name: {} has no index \
                         associated.",
                        name
                    ),
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
                            "Cannot match query result indices with source code for capture name: \
                             {}.",
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
    #[generate_rhai_variant(Impl)]
    /// Creates a new `File` from `path`
    ///
    /// * `path`: the path to read and try to create a File instance for.
    fn new(path: PathBuf) -> Result<Self> {
        let parser = {
            let source_code = std::fs::read_to_string(&path)
                .with_context(|| format!("Could not read file: {:?}", &path))?;
            Parser::new(source_code, *JAVA_TS_LANG)?
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
            let work = vec![
                (FileType::Interface, INTERFACENAME_QUERY),
                (FileType::ClassWithMain, MAIN_METHOD_QUERY),
                (FileType::Class, CLASSNAME_QUERY),
            ];
            for (kind, query) in work {
                let result = parser.query(query)?;

                if !result.is_empty() {
                    break 'outer (
                        kind,
                        #[allow(clippy::or_fun_call)]
                        result
                            .get(0)
                            .ok_or(anyhow!(
                                "Could not find a valid class/interface declaration for {} (vec \
                                 size is 0)",
                                path.display()
                            ))?
                            .get("name")
                            .ok_or(anyhow!(
                                "Could not find a valid class/interface declaration for {} \
                                 (hashmap has no name key) ",
                                path.display()
                            ))?
                            .to_string(),
                    );
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

        Ok(Self {
            path: path.to_owned(),
            file_name: path.file_name().unwrap().to_str().unwrap().to_string(),
            package_name,
            imports,
            name,
            test_methods,
            kind,
            proper_name,
        })
    }

    #[generate_rhai_variant]
    /// Utility method to ask javac for documentation lints using the -Xdoclint
    /// flag.
    ///
    /// The method simply returns the output produced by javac as a String.
    /// There is a ['parse_diag method'][fn@crate::grade::parser::parse_diag]
    /// that can parse these to yield useful information.
    pub fn doc_check(&self) -> Result<String> {
        let child = Command::new(javac_path()?)
            .args([
                "--source-path",
                sourcepath()?.as_str(),
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

        let output = [
            String::from_utf8(child.stderr)?,
            String::from_utf8(child.stdout)?,
        ]
        .concat();

        Ok(output)
    }

    #[generate_rhai_variant]
    /// Utility method to check for syntax errors using javac flag.
    /// TODO: instead of printing javac output, return it.
    /// TODO: have all such methods have generated versions that display output
    /// instead of returning.
    pub fn check(&self) -> Result<()> {
        let path = self.path.display().to_string();

        let child = Command::new(javac_path()?)
            .stdin(Stdio::inherit())
            .stdout(Stdio::inherit())
            .stderr(Stdio::inherit())
            .args([
                "--source-path",
                sourcepath()?.as_str(),
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
                if !status.status.success() {
                    bail!(
                        "There were compiler errors in checked file or other source files it \
                         imports."
                            .bright_red()
                            .bold()
                    );
                }
            }
            Err(e) => bail!(
                "Failed to wait for child process for {}: {}",
                self.proper_name.clone(),
                e
            ),
        };
        Ok(())
    }

    #[generate_rhai_variant]
    /// Utility method to run a java file that has a main method.
    /// TODO: instead of printing javac output, return it.
    /// TODO: have all such methods have generated versions that display output
    /// instead of returning.
    pub fn run(&self) -> Result<()> {
        self.check()?;

        ensure!(
            self.kind == FileType::ClassWithMain,
            "File you wish to run doesn't have a main method."
        );

        let name = self.proper_name.clone();
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
                    eprintln!("{}", "Ran and exited successfully.".bright_green().bold(),);
                } else {
                    eprintln!("{}", "Ran but exited unsuccessfully.".bright_red().bold(),);
                }
            }
            Err(e) => bail!("Failed to wait for child process for {}: {}", name, e),
        };

        Ok(())
    }

    #[generate_rhai_variant]
    /// A utility method that takes a list of strings (or types that implement
    /// Into<String>) meant to represent test method names, and runs those
    /// tests.
    ///
    /// Returns the output from JUnit as a string. There are parsers in
    /// ['grade module'][crate::grade::parser] that helps parse this output.
    ///
    /// * `tests`: list of strings (or types that implement
    /// Into<String>) meant to represent test method names,
    pub fn test(&self, tests: Vec<&str>) -> Result<String> {
        self.check()?;
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

        let tests = tests
            .iter()
            .map(|s| format!("-m {}", s))
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

        let output = [
            String::from_utf8(child.stderr)?,
            String::from_utf8(child.stdout)?,
        ]
        .concat();

        Ok(output)
    }

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

    #[generate_rhai_variant]
    /// treesitter query for this file
    pub fn query(&self, q: &str) -> Result<Vec<Dict>> {
        let parser = {
            let source_code = std::fs::read_to_string(&self.path)
                .with_context(|| format!("Could not read file: {:?}", &self.path))?;
            Parser::new(source_code, *JAVA_TS_LANG)?
        };

        let mut results = vec![];

        let query = Query::new(parser.lang, q).unwrap();

        let mut cursor = QueryCursor::new();
        let matches = cursor.matches(&query, parser._tree.root_node(), parser.code.as_bytes());
        let capture_names = query.capture_names();

        for m in matches {
            let mut result = Dict::new();

            for name in capture_names {
                let index = query.capture_index_for_name(name);
                let index = match index {
                    Some(i) => i,
                    None => bail!(
                        "Error while querying source code. Capture name: {} has no index \
                         associated.",
                        name
                    ),
                };

                let value = m.captures.iter().find(|c| c.index == index);
                let value = match value {
                    Some(v) => v,
                    None => continue,
                };

                let value = value
                    .node
                    .utf8_text(parser.code.as_bytes())
                    .with_context(|| {
                        format!(
                            "Cannot match query result indices with source code for capture name: \
                             {}.",
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

impl Project {
    #[generate_rhai_variant(Impl)]
    /// Initializes a Project, by discovering java files in the
    /// [struct@UMM_DIR] directory. Also downloads some `jar`
    /// files required for unit testing and mutation testing.
    ///
    /// TODO: Only download these jars if required.
    /// TODO: get rid of DataStructures.jar from all labs and assignments.
    pub fn new() -> Result<Self> {
        let mut files = vec![];
        let mut names = vec![];

        for path in find_files("java", 15, &ROOT_DIR)? {
            let file = File::new(path)?;
            names.push(file.proper_name.clone());
            files.push(file);
        }

        if !LIB_DIR.as_path().is_dir() {
            std::fs::create_dir(LIB_DIR.as_path()).unwrap();
        }

        // TODO: Move this to an init function that takes CONSTANTS into account
        if !ROOT_DIR.join(".vscode").as_path().is_dir() {
            std::fs::create_dir(ROOT_DIR.join(".vscode").as_path()).unwrap();
            let mut file = std::fs::OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(ROOT_DIR.join(".vscode").join("settings.json").as_path())?;

            write!(
                &mut file,
                r#"
{{
    "java.project.sourcePaths": [
        ".",
        "./src/",
        "./test/"
    ],
    "java.project.outputPath": "./target/",
    "java.project.referencedLibraries": [
        "lib/**/*.jar"
    ],
}}
            "#
            )?;
        }

        //     download(
        //     "https://github.com/DhruvDh/umm/blob/next-assign1-spring-22/jar_files/DataStructures.jar?raw=true",
        //     &LIB_DIR.join("DataStructures.jar"),
        // false)?;
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

        Ok(Self {
            files,
            names,
            classpath: classpath()?,
        })
    }

    #[generate_rhai_variant]
    /// Attempts to identiy the correct file from the project from a partial or
    /// fully formed name as expected by a java compiler.
    ///
    /// Returns a reference to the identified file, if any.
    ///
    /// * `name`: partial/fully formed name of the Java file to look for.
    pub fn identify(&self, name: &str) -> Result<File> {
        let name: String = name.into();

        if let Some(i) = self.names.iter().position(|n| *n == name) {
            Ok(self.files[i].clone())
        } else if let Some(i) = self.files.iter().position(|n| n.file_name == name) {
            Ok(self.files[i].clone())
        } else if let Some(i) = self.files.iter().position(|n| n.name.clone() == name) {
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

    /// Get a reference to the project's files.
    #[must_use]
    pub fn files(&self) -> &[File] {
        self.files.as_ref()
    }

    /// Prints project struct as a json
    pub fn info(&self) -> Result<()> {
        println!("{}", serde_json::to_string(&self)?);
        Ok(())
    }
}
