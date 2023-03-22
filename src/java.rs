#![warn(missing_docs)]
#![warn(clippy::missing_docs_in_private_items)]

use std::{
    fmt::Formatter,
    path::PathBuf,
    process::{
        Command,
        Output,
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
use futures::{
    future::{
        join_all,
        try_join_all,
    },
    stream::FuturesUnordered,
};
use rhai::Array;
// Allowed because CustomType is not deprecated, just volatile
#[allow(deprecated)]
use rhai::{
    CustomType,
    EvalAltResult,
};
use serde::{
    Deserialize,
    Serialize,
};
use tokio::io::AsyncWriteExt;
use tree_sitter::{
    Query,
    QueryCursor,
    Tree,
};
use umm_derive::generate_rhai_variant;

use crate::{
    constants::*,
    util::*,
    vscode::{self,},
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
    /// Conscise description of the file
    description:  String,
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
///
/// TODO: The source code should not be in here, extract it out
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
        parser
            .set_language(*JAVA_TS_LANG)
            .expect("Error loading Java grammar");
        let tree = parser.parse(code, None);

        Self {
            code:  String::new(),
            _tree: tree,
            lang:  *JAVA_TS_LANG,
        }
    }
}

impl std::fmt::Debug for Parser {
    fn fmt(&self, _: &mut Formatter<'_>) -> std::fmt::Result {
        Ok(())
    }
}

impl Parser {
    #[generate_rhai_variant(Impl, Fallible)]
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
            _tree: Some(tree),
            lang,
        })
    }

    /// A getter for parser's source code
    pub fn code(&mut self) -> String {
        self.code.clone()
    }

    /// A setter for parser's source code
    pub fn set_code(&mut self, code: String) {
        self.code = code;
    }

    #[generate_rhai_variant(Fallible, Mut)]
    /// Applies a tree sitter query and returns the result as a collection of
    /// HashMaps
    ///
    /// * `q`: the tree-sitter query to be applied
    pub fn query(&self, q: &str) -> Result<Vec<Dict>> {
        let mut results = vec![];
        let tree = self
            ._tree
            .as_ref()
            .context("Treesitter could not parse code")?;

        let query = Query::new(self.lang, q).unwrap();

        let mut cursor = QueryCursor::new();
        let matches = cursor.matches(&query, tree.root_node(), self.code.as_bytes());
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
                             {name}."
                        )
                    })?;

                result.insert(name.clone(), value.to_string());
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
        builder
            .with_name("JavaParser")
            .with_fn("new_java_parser", Parser::new_script)
            .with_fn("code", Parser::code)
            .with_fn("set_code", Parser::set_code)
            .with_fn("query", Parser::query_mut_script);
    }
}

impl File {
    #[generate_rhai_variant(Impl, Fallible)]
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

        let description = match kind {
            FileType::Interface => {
                let empty_dict = Dict::new();
                let empty = String::new();
                let not_found = String::from("[NOT FOUND]");

                let query_result = parser
                    .query(INTERFACE_DECLARATION_QUERY)
                    .unwrap_or_default();
                let declaration = query_result.first().unwrap_or(&empty_dict);

                let parameters = declaration.get("parameters").unwrap_or(&empty);
                let extends = declaration.get("extends").unwrap_or(&empty);

                let consts = parser
                    .query(INTERFACE_CONSTANTS_QUERY)
                    .unwrap_or_default()
                    .iter()
                    .map(|c| {
                        let name = c.get("constant").unwrap_or(&not_found);
                        format!("\t\t- `{}`", name)
                    })
                    .collect::<Vec<String>>()
                    .join("\n");

                let methods = parser
                    .query(INTERFACE_METHODS_QUERY)
                    .unwrap_or_default()
                    .iter()
                    .map(|m| {
                        let sig = m.get("signature").unwrap_or(&not_found);
                        format!("\t\t- `{}`", sig)
                    })
                    .collect::<Vec<String>>()
                    .join("\n");

                let methods = if methods.trim().is_empty() {
                    String::from("\t\t[NOT FOUND]")
                } else {
                    methods
                };

                #[rustfmt::skip]
                format!(
                    "- Interface: \
                     `{proper_name} {parameters} {extends}`:\n\n\tConstants:\n{consts}\n\n\tMethods:\n{methods}"
                )
            }
            _ => {
                let empty_dict = Dict::new();
                let empty = String::new();
                let not_found = String::from("[NOT FOUND]");

                let query_result = parser.query(CLASS_DECLARATION_QUERY).unwrap_or_default();
                let declaration = query_result.first().unwrap_or(&empty_dict);

                let parameters = declaration.get("typeParameters").unwrap_or(&empty);
                let implements = declaration.get("interfaces").unwrap_or(&empty);

                let fields = parser
                    .query(CLASS_FIELDS_QUERY)
                    .unwrap_or_default()
                    .iter()
                    .map(|f| {
                        let field = f.get("field").unwrap_or(&not_found);
                        format!("\t\t- `{}`", field)
                    })
                    .collect::<Vec<String>>()
                    .join("\n");

                let methods = parser
                    .query(CLASS_METHOD_QUERY)
                    .unwrap_or_default()
                    .iter()
                    .map(|m| {
                        let modifier = m.get("modifier").unwrap_or(&empty);
                        let annotation = m.get("annotation").unwrap_or(&empty);
                        let return_type = m.get("returnType").unwrap_or(&not_found);
                        let identifier = m.get("identifier").unwrap_or(&not_found);
                        let parameters = m.get("parameters").unwrap_or(&empty);
                        let throws = m.get("throws").unwrap_or(&empty);

                        if identifier.as_str() == not_found.as_str() {
                            "\t\t- [NOT FOUND]".to_string()
                        } else {
                            format!(
                                "\t\t- `{annotation} {modifier} {return_type} {identifier} \
                                 {parameters} {throws}`",
                            )
                        }
                    })
                    .collect::<Vec<String>>()
                    .join("\n");

                format!(
                    "- Class: `{proper_name} {parameters} \
                     {implements}`:\n\n\tFields:\n{fields}\n\n\tMethods:\n{methods}",
                )
            }
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
            parser,
            description,
        })
    }

    #[generate_rhai_variant(Fallible, Mut)]
    /// Utility method to ask javac for documentation lints using the -Xdoclint
    /// flag.
    ///
    /// The method simply returns the output produced by javac as a String.
    /// There is a ['parse_diag method'][fn@crate::parsers::parser::parse_diag]
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

    #[generate_rhai_variant(Fallible, Mut)]
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

    #[generate_rhai_variant(Fallible, Mut)]
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

    #[generate_rhai_variant(Fallible, Mut)]
    /// A utility method that takes a list of strings (or types that implement
    /// `Into<String>`) meant to represent test method names, and runs those
    /// tests.
    ///
    /// Returns the output from JUnit as a string. There are parsers in
    /// ['parsers module'][crate::parsers::parser] that helps parse this output.
    ///
    /// * `tests`: list of strings (or types that implement
    /// `Into<String>`) meant to represent test method names,
    pub fn test(&self, tests: Vec<&str>) -> Result<Output> {
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
            .map(|s| format!("-m{s}"))
            .collect::<Vec<String>>();
        let methods: Vec<&str> = tests.iter().map(String::as_str).collect();

        Command::new(java_path()?)
            // .stdin(Stdio::inherit())
            // .stdout(Stdio::inherit())
            // .stderr(Stdio::inherit())
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
            .context("Could not issue java command to run the tests for some reason.")
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

    /// Get a reference to the file's test methods.
    pub fn test_methods_mut_script(&mut self) -> Array {
        self.test_methods().iter().map(|s| s.into()).collect()
    }

    /// treesitter query for this file
    pub fn query(&self, q: &str) -> Result<Vec<Dict>> {
        self.parser.query(q)
    }

    /// treesitter query for this file
    pub fn query_mut_script(&mut self, q: &str) -> Result<Array, Box<EvalAltResult>> {
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
}

// Allowed because CustomType is not deprecated, just volatile
#[allow(deprecated)]
impl CustomType for File {
    fn build(mut builder: rhai::TypeBuilder<Self>) {
        builder
            .with_name("JavaFile")
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
    ///
    /// TODO: Only download these jars if required.
    /// TODO: get rid of DataStructures.jar from all labs and assignments.
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

        let mut sourcepath = vec![
            SOURCE_DIR.join("").display().to_string(),
            TEST_DIR.join("").display().to_string(),
        ];

        if !find_files("java", 0, &ROOT_DIR)?.is_empty() {
            sourcepath.push(ROOT_DIR.join("").display().to_string());
        }

        let proj = Self {
            files,
            names,
            classpath,
            sourcepath,
            root_dir: ROOT_DIR.display().to_string(),
        };

        let _guard = rt.enter();
        rt.block_on(async {
            let handles = FuturesUnordered::new();
            let (proj1, proj2, proj3) = (proj.clone(), proj.clone(), proj.clone());

            handles.push(tokio::spawn(async move {
                proj1.download_libraries_if_needed().await
            }));
            handles.push(tokio::spawn(
                async move { proj2.update_vscode_settings().await },
            ));
            handles.push(tokio::spawn(
                async move { proj3.update_vscode_tasks().await },
            ));

            try_join_all(handles).await
        })?
        .into_iter()
        .collect::<Result<Vec<()>>>()?;

        Ok(proj)
    }

    #[generate_rhai_variant(Impl, Mut, Fallible)]
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
                    "https://ummfiles.fra1.digitaloceanspaces.com/jar_files/junit-platform-console-standalone-1.9.0-RC1.jar",
                    &LIB_DIR.join(JUNIT_PLATFORM),
                false
                        )
                        .await
            });

            let handle2 = tokio::spawn(async {
                download(
                    "https://ummfiles.fra1.digitaloceanspaces.com/jar_files/junit-4.13.2.jar",
                    &LIB_DIR.join("junit-4.13.2.jar"),
                    false,
                )
                .await
            });

            let handle3 = tokio::spawn(async {
                download(
                    "https://ummfiles.fra1.digitaloceanspaces.com/jar_files/pitest-1.9.5.jar",
                    &LIB_DIR.join("pitest.jar"),
                    false,
                )
                .await
            });

            let handle4 = tokio::spawn(async {
                download(
                        "https://ummfiles.fra1.digitaloceanspaces.com/jar_files/pitest-command-line-1.9.5.jar",
                        &LIB_DIR.join("pitest-command-line.jar"),
                        false,
                    )
                    .await
            });

            let handle5 = tokio::spawn(async {
                download(
                    "https://ummfiles.fra1.digitaloceanspaces.com/jar_files/pitest-entry-1.9.5.jar",
                    &LIB_DIR.join("pitest-entry.jar"),
                    false,
                )
                .await
            });

            let handle6 = tokio::spawn(async {
                download(
                        "https://ummfiles.fra1.digitaloceanspaces.com/jar_files/pitest-junit5-plugin-1.0.0.jar",
                        &LIB_DIR.join("pitest-junit5-plugin.jar"),
                        false,
                    )
                    .await
            });

            let handles =
                FuturesUnordered::from_iter([handle1, handle2, handle3, handle4, handle5, handle6]);

            futures::future::try_join_all(handles).await?;
        }
        Ok(())
    }

    /// Creates a vscode settings.json file for the project.
    pub async fn update_vscode_settings(&self) -> Result<()> {
        // TODO: Move this to an init function that takes CONSTANTS into account
        if !ROOT_DIR.join(".vscode").as_path().is_dir() {
            tokio::fs::create_dir(ROOT_DIR.join(".vscode").as_path())
                .await
                .unwrap();
        }

        if !ROOT_DIR.join(".vscode/settings.json").as_path().exists() {
            let mut file = tokio::fs::OpenOptions::new()
                .write(true)
                .truncate(true)
                .create(true)
                .open(ROOT_DIR.join(".vscode").join("settings.json").as_path())
                .await?;

            let settings = vscode::SettingsFile::builder()
                .java_source_path(self.sourcepath.clone())
                .java_output_path(BUILD_DIR.join("").display().to_string())
                .java_referenced_libs(self.classpath.clone())
                .umm_binary_path(umm_path())
                .build();

            file.write_all(serde_json::to_string_pretty(&settings)?.as_bytes())
                .await?;
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
        result.push_str(
            "> What follows is a summary of the student's submission's files, their fields and \
             methods generated via treesitter queries.\n\n",
        );

        for f in self.files.iter() {
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
            (
                Some(vec!["Set umm to be executable".to_string()]),
                Some(vscode::DependsOrder::Sequence),
            )
        } else {
            (None, None)
        };

        tasks.push(
            vscode::Task::builder()
                .label("Set umm to be executable".to_string())
                .r#type(vscode::Type::Shell)
                .command("chmod")
                .args(vec![
                    vscode::Args::builder()
                        .value("+x")
                        .quoting(vscode::ArgQuoting::Escape)
                        .build(),
                    vscode::Args::builder()
                        .value("${config:ummBinaryPath}")
                        .quoting(vscode::ArgQuoting::Weak)
                        .build(),
                ])
                .depends_on(None)
                .depends_order(None)
                .build(),
        );
        tasks.push(
            vscode::Task::builder()
                .label("Clean library and target folders".to_string())
                .r#type(vscode::Type::Shell)
                .command("${config:ummBinaryPath}")
                .args(vec![vscode::Args::builder()
                    .value("clean")
                    .quoting(vscode::ArgQuoting::Escape)
                    .build()])
                .depends_on(default_depends_on.clone())
                .depends_order(default_depends_order)
                .build(),
        );

        tasks.push(
            vscode::Task::builder()
                .label("Reset project metadata".into())
                .r#type(vscode::Type::Shell)
                .command("${config:ummBinaryPath}")
                .args(vec![vscode::Args::builder()
                    .value("reset")
                    .quoting(vscode::ArgQuoting::Escape)
                    .build()])
                .depends_on(default_depends_on.clone())
                .depends_order(default_depends_order)
                .build(),
        );

        tasks.push(
            vscode::Task::builder()
                .label("Check health of the project".into())
                .r#type(vscode::Type::Shell)
                .command("${config:ummBinaryPath}")
                .args(vec![vscode::Args::builder()
                    .value("check-health")
                    .quoting(vscode::ArgQuoting::Escape)
                    .build()])
                .depends_on(default_depends_on.clone())
                .depends_order(default_depends_order)
                .build(),
        );

        tasks.push(
            vscode::Task::builder()
                .label("Update umm executable".into())
                .r#type(vscode::Type::Shell)
                .command("${config:ummBinaryPath}")
                .args(vec![vscode::Args::builder()
                    .value("update")
                    .quoting(vscode::ArgQuoting::Escape)
                    .build()])
                .depends_on(default_depends_on.clone())
                .depends_order(default_depends_order)
                .build(),
        );

        for file in self.files().iter() {
            match file.kind() {
                FileType::ClassWithMain => {
                    tasks.push(
                        vscode::Task::builder()
                            .label(format!("Run {}", file.name))
                            .r#type(vscode::Type::Shell)
                            .command("${config:ummBinaryPath}")
                            .args(vec![
                                vscode::Args::builder()
                                    .value("run")
                                    .quoting(vscode::ArgQuoting::Escape)
                                    .build(),
                                vscode::Args::builder()
                                    .value(&file.proper_name)
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
                        vscode::Task::builder()
                            .label(format!("Run tests for {}", file.name))
                            .r#type(vscode::Type::Shell)
                            .command("${config:ummBinaryPath}")
                            .args(vec![
                                vscode::Args::builder()
                                    .value("test")
                                    .quoting(vscode::ArgQuoting::Escape)
                                    .build(),
                                vscode::Args::builder()
                                    .value(&file.proper_name)
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
                        let method = method
                            .split_once('#')
                            .ok_or(anyhow!("Could not parse test method - {}", method))?
                            .1;
                        // commands.push(method.into());
                        test_methods.push(String::from(method));
                    }

                    if !test_methods.is_empty() {
                        let input = vscode::Input::PickString {
                            id:          file.proper_name.to_string(),
                            description: "Which test to run?".to_string(),
                            options:     test_methods.clone(),
                            default:     test_methods.first().unwrap().clone(),
                        };
                        inputs.push(input);
                    }

                    tasks.push(
                        vscode::Task::builder()
                            .label(format!("Run specific test from {}", file.name))
                            .r#type(vscode::Type::Shell)
                            .command("${config:ummBinaryPath}")
                            .args(vec![
                                vscode::Args::builder()
                                    .value("test")
                                    .quoting(vscode::ArgQuoting::Escape)
                                    .build(),
                                vscode::Args::builder()
                                    .value(&file.proper_name)
                                    .quoting(vscode::ArgQuoting::Escape)
                                    .build(),
                                vscode::Args::builder()
                                    .value(format!("${{input:{}}}", file.proper_name))
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
                vscode::Task::builder()
                    .label(format!("Check {}", file.name))
                    .r#type(vscode::Type::Shell)
                    .command("${config:ummBinaryPath}")
                    .args(vec![
                        vscode::Args::builder()
                            .value("check")
                            .quoting(vscode::ArgQuoting::Escape)
                            .build(),
                        vscode::Args::builder()
                            .value(&file.proper_name)
                            .quoting(vscode::ArgQuoting::Escape)
                            .build(),
                    ])
                    .depends_on(default_depends_on.clone())
                    .depends_order(default_depends_order)
                    .build(),
            );
            tasks.push(
                vscode::Task::builder()
                    .label(format!("Check JavaDoc for {}", file.name))
                    .r#type(vscode::Type::Shell)
                    .command("${config:ummBinaryPath}")
                    .args(vec![
                        vscode::Args::builder()
                            .value("doc-check")
                            .quoting(vscode::ArgQuoting::Escape)
                            .build(),
                        vscode::Args::builder()
                            .value(&file.proper_name)
                            .quoting(vscode::ArgQuoting::Escape)
                            .build(),
                    ])
                    .depends_on(default_depends_on.clone())
                    .depends_order(default_depends_order)
                    .build(),
            );
        }

        let resp = tokio::spawn(async {
            POSTGREST_CLIENT
                .from("grading_scripts")
                .eq("course", COURSE)
                .eq("term", TERM)
                .select("assignment")
                .execute()
                .await?
                .text()
                .await
                .context("Could not get grading scripts")
        })
        .await?;

        let resp: serde_json::Value = serde_json::from_str(resp?.as_str())?;

        let mut keys = vec![];
        for val in resp.as_array().unwrap() {
            keys.push(val.as_object().unwrap()["assignment"].to_string());
        }

        inputs.push(vscode::Input::PickString {
            id:          "gradable_assignments".to_string(),
            description: "Which assignment are you working on?".to_string(),
            options:     keys.clone(),
            default:     keys.first().unwrap().clone(),
        });

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

        if !ROOT_DIR.join(".vscode").as_path().is_dir() {
            tokio::fs::create_dir(ROOT_DIR.join(".vscode").as_path())
                .await
                .unwrap();
        }

        let mut file = tokio::fs::OpenOptions::new()
            .write(true)
            .truncate(true)
            .create(true)
            .open(ROOT_DIR.join(".vscode").join("tasks.json").as_path())
            .await?;

        let task_file = vscode::TasksFile::builder()
            .tasks(tasks)
            .inputs(inputs)
            .build();

        file.write_all(serde_json::to_string_pretty(&task_file)?.as_bytes())
            .await?;

        Ok(())
    }
}

// Allowed because CustomType is not deprecated, just volatile
#[allow(deprecated)]
impl CustomType for Project {
    fn build(mut builder: rhai::TypeBuilder<Self>) {
        builder
            .with_name("JavaProject")
            .with_fn("new_java_project", Project::new_script)
            .with_fn("identify", Project::identify_mut_script)
            .with_fn("files", Project::files)
            .with_fn("info", Project::info_script);
    }
}
