#![feature(slice_pattern)]
#![feature(array_methods)]

use anyhow::{bail, ensure, Context, Result};
use colored::*;
use glob::glob;
use inquire::{error::InquireError, MultiSelect, Select};
use java_dependency_analyzer::*;
use lazy_static::lazy_static;
use std::{
    collections::HashMap,
    ffi::OsString,
    path::{Path, PathBuf},
    process::{Command, Stdio},
};
use which::which;

lazy_static! {
    static ref ROOT_DIR: PathBuf = PathBuf::from(".");
    static ref SOURCE_DIR: PathBuf = PathBuf::from(".").join("src");
    static ref BUILD_DIR: PathBuf = PathBuf::from(".").join("target");
    static ref TEST_DIR: PathBuf = PathBuf::from(".").join("test");
    static ref LIB_DIR: PathBuf = PathBuf::from(".").join("lib");
    static ref UMM_DIR: PathBuf = PathBuf::from(".").join(".umm");
    static ref SEPARATOR: &'static str = if cfg!(windows) { ";" } else { ":" };
    static ref JAVA_TS_LANG: tree_sitter::Language = tree_sitter_java::language();
}
type Dict = HashMap<String, String>;

#[derive(Debug, Clone)]
enum JavaFileType {
    Interface,
    Class,
    ClassWithMain,
    Test,
}

#[derive(Debug, Clone)]
struct JavaFile {
    path: PathBuf,
    file_name: String,
    package_name: Option<String>,
    imports: Option<Vec<Dict>>,
    name: Option<String>,
    pretty_name: Option<String>,
    proper_name: Option<String>,
    test_methods: Vec<String>,
    pretty_test_methods: Vec<String>,
    kind: JavaFileType,
}

fn javac_path() -> Result<OsString> {
    Ok(which("javac").map(PathBuf::into_os_string)?)
}

fn java_path() -> Result<OsString> {
    Ok(which("java").map(PathBuf::into_os_string)?)
}

fn classpath() -> Result<String> {
    let mut path: Vec<String> = vec![
        BUILD_DIR.display().to_string(),
        LIB_DIR.display().to_string(),
    ];

    path.append(
        &mut find_files("jar", 4, &LIB_DIR)?
            .iter()
            .map(|p| p.as_path().display().to_string())
            .collect(),
    );

    Ok(path.join(&SEPARATOR))
}

impl JavaFile {
    fn new(path: PathBuf) -> Result<Self> {
        let source_code = std::fs::read_to_string(&path)
            .with_context(|| format!("Could not read file: {:?}", &path))?;

        let parser = Parser::new(source_code, *JAVA_TS_LANG)?;

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
            "Expected exactly one class/interface name, found {}.",
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
        })
    }
}

struct JavaProject {
    files: Vec<JavaFile>,
    pretty_names: Vec<String>,
    names: Vec<String>,
}

impl JavaProject {
    fn new() -> Result<Self> {
        let mut files = vec![];
        let mut pretty_names = vec![];
        let mut names = vec![];

        for path in find_files("java", 15, &ROOT_DIR)? {
            let file = JavaFile::new(path)?;
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

    fn check(&self, name: String) -> Result<()> {
        let index = self.pretty_names.iter().position(|x| x == &name);
        ensure!(
            index.is_some(),
            "Could not find class/interface with name {}.",
            name
        );
        let path = self.files[index.unwrap()].path.display().to_string();
        let name = self.names[index.unwrap()].clone();

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
                // "-Xdoclint:missing",
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
            Err(e) => bail!("Failed to wait for child process for {}: {}", name, e),
        };
        Ok(())
    }

    fn run(&self, name: String) -> Result<()> {
        self.check(name.clone())?;

        let index = self.pretty_names.iter().position(|x| x == &name);
        ensure!(
            index.is_some(),
            "Could not find class/interface with name {}.",
            name
        );
        let path = self.files[index.unwrap()].path.display().to_string();
        let name = self.names[index.unwrap()].clone();

        let child = Command::new(java_path()?)
            .stdin(Stdio::inherit())
            .stdout(Stdio::inherit())
            .stderr(Stdio::inherit())
            .args(["--class-path", classpath()?.as_str(), path.as_str()])
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

    fn test(&self, name: String) -> Result<()> {
        self.check(name.clone())?;

        let index = self.pretty_names.iter().position(|x| x == &name);
        ensure!(
            index.is_some(),
            "Could not find class/interface with name {}.",
            name
        );
        let file = &self.files[index.unwrap()];
        let name = file.proper_name.clone().unwrap();

        let tests = file.test_methods.clone();
        let pretty_tests = file.pretty_test_methods.clone();

        let ans: Result<Vec<String>, InquireError> =
            MultiSelect::new("Which tests to run?", pretty_tests.clone()).prompt();
        let ans = ans.context("Failed to get answer for some reason.")?;

        ensure!(!ans.is_empty(), "Must select at least one test to run");

        let mut indices = vec![];

        for (i, f) in pretty_tests.iter().enumerate() {
            if ans.contains(f) {
                indices.push(i);
            }
        }

        let names: Vec<String> = tests
            .iter()
            .enumerate()
            .filter(|x| indices.contains(&x.0))
            .map(|x| x.1.clone())
            .collect();

        let mut methods = vec![];
        for a in names {
            methods.push("-m".to_string());
            methods.push(a);
        }

        let methods: Vec<&str> = methods.iter().map(String::as_str).collect();

        let child = Command::new(java_path()?)
            .stdin(Stdio::inherit())
            .stdout(Stdio::inherit())
            .stderr(Stdio::inherit())
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
            .spawn()
            .context("Could not issue java command to run the tests for some reason.")?;

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
}

pub fn run_prompt() -> Result<()> {
    let project =
        JavaProject::new().context("Something went wrong while discovering the project.")?;

    let mut indices = vec![];

    for (i, f) in project.files.iter().enumerate() {
        if let JavaFileType::ClassWithMain = f.kind {
            indices.push(i)
        };
    }

    let names: Vec<String> = project
        .pretty_names
        .iter()
        .enumerate()
        .filter(|x| indices.contains(&x.0))
        .map(|x| x.1.clone())
        .collect();

    if names.is_empty() {
        println!(
            "{}",
            "No classes with tests methods found.".bright_red().bold()
        );
    } else {
        let ans: Result<String, InquireError> = Select::new("Which file?", names).prompt();
        let ans = ans.context("Failed to get answer for some reason.")?;

        project.run(ans)?;
    }

    Ok(())
}

pub fn check_prompt() -> Result<()> {
    let project =
        JavaProject::new().context("Something went wrong while discovering the project.")?;

    let names = project.pretty_names.clone();
    let ans: Result<String, InquireError> = Select::new("Which file?", names).prompt();
    let ans = ans.context("Failed to get answer for some reason.")?;

    project.check(ans)?;

    Ok(())
}

pub fn test_prompt() -> Result<()> {
    let project =
        JavaProject::new().context("Something went wrong while discovering the project.")?;

    let mut indices = vec![];

    for (i, f) in project.files.iter().enumerate() {
        if let JavaFileType::Test = f.kind {
            indices.push(i)
        };
    }

    let names: Vec<String> = project
        .pretty_names
        .iter()
        .enumerate()
        .filter(|x| indices.contains(&x.0))
        .map(|x| x.1.clone())
        .collect();

    if names.is_empty() {
        println!(
            "{}",
            "No classes with JUnit annotated test methods found."
                .bright_red()
                .bold()
        );
    } else {
        let ans: Result<String, InquireError> = Select::new("Which file?", names).prompt();
        let ans = ans.context("Failed to get answer for some reason.")?;

        project.test(ans)?;
    }

    Ok(())
}

pub fn clean() {
    std::fs::remove_dir_all(BUILD_DIR.as_path()).unwrap_or(());
}

fn find_files(extension: &str, search_depth: i8, root_dir: &Path) -> Result<Vec<PathBuf>> {
    let mut root_dir = PathBuf::from(root_dir);

    for _ in 0..search_depth {
        root_dir.push("**");
    }

    root_dir.push(format!("*.{}", extension));
    let root_dir = root_dir
        .to_str()
        .context("Could not convert root_dir to string")?;

    Ok(glob(root_dir)
        .context("Could not create glob")?
        .filter_map(Result::ok)
        .collect())
}
