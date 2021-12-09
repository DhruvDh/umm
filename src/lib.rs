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
use tabled::{Table, Tabled};
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
    source_code: String,
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
            source_code: source_code.clone(),
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

pub fn grade() -> Result<()> {
    let mut result = vec![];
    clean();

    let ans = vec![
        String::from("test\u{1b}[1;92m1A\u{1b}[0m"),
        String::from("test\u{1b}[1;92m1B\u{1b}[0m"),
        String::from("test\u{1b}[1;92m1C\u{1b}[0m"),
        String::from("test\u{1b}[1;92m2A\u{1b}[0m"),
        String::from("test\u{1b}[1;92m2B\u{1b}[0m"),
        String::from("test\u{1b}[1;92m2C\u{1b}[0m"),
        String::from("test\u{1b}[1;92m3A\u{1b}[0m"),
        String::from("test\u{1b}[1;92m3B\u{1b}[0m"),
        String::from("test\u{1b}[1;92m3C\u{1b}[0m"),
        String::from("test\u{1b}[1;92m4A\u{1b}[0m"),
        String::from("test\u{1b}[1;92m4B\u{1b}[0m"),
        String::from("test\u{1b}[1;92m4C\u{1b}[0m"),
        String::from("test\u{1b}[1;92m5A\u{1b}[0m"),
        String::from("test\u{1b}[1;92m5B\u{1b}[0m"),
        String::from("test\u{1b}[1;92m5C\u{1b}[0m"),
        String::from("test\u{1b}[1;92m6A\u{1b}[0m"),
        String::from("test\u{1b}[1;92m6B\u{1b}[0m"),
        String::from("test\u{1b}[1;92m6C\u{1b}[0m"),
        String::from("test\u{1b}[1;92m7A\u{1b}[0m"),
        String::from("test\u{1b}[1;92m7B\u{1b}[0m"),
        String::from("test\u{1b}[1;92m7C\u{1b}[0m"),
        String::from("test\u{1b}[1;92m8A\u{1b}[0m"),
        String::from("test\u{1b}[1;92m8B\u{1b}[0m"),
        String::from("test\u{1b}[1;92m8C\u{1b}[0m"),
        String::from("test\u{1b}[1;92m9A\u{1b}[0m"),
        String::from("test\u{1b}[1;92m9B\u{1b}[0m"),
        String::from("test\u{1b}[1;92m9C\u{1b}[0m"),
        String::from("test\u{1b}[1;92m10A\u{1b}[0m"),
        String::from("test\u{1b}[1;92m10B\u{1b}[0m"),
        String::from("test\u{1b}[1;92m10C\u{1b}[0m"),
    ];

    let project =
        JavaProject::new().context("Something went wrong while discovering the project.")?;

    let index = project.names.iter().position(|x| x == "Project4.ArrayUtil");
    ensure!(
        index.is_some(),
        "There is no Project4.ArrayUtil class file I can find."
    );
    let solution = project.files[index.unwrap()].source_code.clone();
    let parser = Parser::new(solution, *JAVA_TS_LANG)?;

    let mut reasons = vec![];

    if parser.query(CLASS_ARRAYUTIL)?.is_empty() {
        reasons.push("- No class with name 'ArrayUtil'");
    }

    if parser.query(INTARRAY)?.is_empty() {
        reasons.push("- No class member with name 'intArray' of type int[]");
    }

    if parser.query(DEFAULT_CONSTRUCTOR)?.is_empty() {
        reasons.push("- No default constructor");
    }

    if parser.query(CONSTRUCTOR_INT)?.is_empty() {
        reasons.push("- No constructor that takes an integer arguement");
    }

    if parser.query(GETTER)?.is_empty() {
        reasons.push("- No appropriate getter.");
    }

    if parser.query(SETTER)?.is_empty() {
        reasons.push("- No appropriate setter.");
    }

    if parser.query(MINVALUE)?.is_empty() {
        reasons.push("- No appropriate minValue method.");
    }
    if parser.query(MAXVALUE)?.is_empty() {
        reasons.push("- No appropriate maxValue method.");
    }
    if parser.query(COUNTUNIQUE)?.is_empty() {
        reasons.push("- No appropriate countUniqueIntegers method.");
    }

    let grade = 9 - reasons.len();

    result.push(GradeResult {
        Part: "B - Class and Members",
        Grade: format!("{}.00", grade),
        Reason: if reasons.is_empty() {
            format!("Everything checks out")
        } else {
            reasons.join("\n")
        },
    });

    project.check(project.pretty_names[index.unwrap()].clone());

    let name = String::from("\u{1b}[1;93mProject4\u{1b}[0m.\u{1b}[1;94mArrayUtilTest\u{1b}[0m");
    let index = project.pretty_names.iter().position(|x| x == &name);
    ensure!(
        index.is_some(),
        "Could not find class/interface with name {}.",
        name
    );
    project.check(name);

    let file = &project.files[index.unwrap()];

    let tests = file.test_methods.clone();
    let pretty_tests = file.pretty_test_methods.clone();

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
    } else {
        println!("{}", "Ran but exited unsuccessfully.".bright_red().bold(),);
    }

    let mut num_tests_passed = 0;
    for line in std::str::from_utf8(&child.stdout)?.lines() {
        let parse_result = junit_summary_parser::num_tests_passed(line)
            .context("While parsing Junit summary table");
        if let Ok(n) = parse_result {
            num_tests_passed = n;
        }
    }
    result.push(GradeResult {
        Part: "B - Array Operations",
        Grade: format!("{:.2}", (num_tests_passed as f32 / 30.0) * 12.0),
        Reason: format!("{}/30 unit tests passed", num_tests_passed),
    });
    println!("{}", Table::new(result).with(tabled::Style::pseudo()));

    Ok(())
}

#[derive(Tabled)]
struct GradeResult<'a> {
    Part: &'a str,
    Grade: String,
    Reason: String,
}

peg::parser! {
    grammar junit_summary_parser() for str {
        rule number() -> u32
            = n:$(['0'..='9']+) {? n.parse().or(Err("u32")) }
        rule whitespace() = quiet!{[' ' | '\n' | '\t']+}
        rule type_name()
            = "tests successful"
        pub rule num_tests_passed() -> u32
            = "[" whitespace()? l:number() whitespace()? type_name() whitespace()? "]" { l }
    }
}

const CLASS_ARRAYUTIL: &str = r#"
    (class_declaration
        name: (identifier) @name
        (#eq? @name "ArrayUtil")
    )
"#;

const INTARRAY: &str = r#"
(field_declaration
	type: (array_type
    element: (integral_type)
    )
    declarator: (variable_declarator
    	name: (identifier) @name
        )
    (#eq? @name "intArray")
)
"#;

const CONSTRUCTOR_INT: &str = r#"
(constructor_declaration 
	(formal_parameters
    (formal_parameter
    	type: (integral_type) @type
    ))
	(#eq? @type "int")
)
"#;
const DEFAULT_CONSTRUCTOR: &str = r#"
(constructor_declaration 
	parameters: (formal_parameters) @para
    (#eq? @para "()")
)
"#;

const GETTER: &str = r#"
(method_declaration
	type: (array_type element: (integral_type))
 	name: (identifier) @ident
	(#eq? @ident "getIntArray")
)
"#;

const SETTER: &str = r#"
(method_declaration
	type: (void_type)
 	name: (identifier) @ident
	(#eq? @ident "setIntArray")
)
"#;

const MINVALUE: &str = r#"
(method_declaration
    type: (integral_type)
    name: (identifier) @ident
	parameters: (formal_parameters) @para
    (#eq? @ident "minValue")
    (#eq? @para "()")
)
"#;
const MAXVALUE: &str = r#"
(method_declaration
    type: (integral_type)
    name: (identifier) @ident
	parameters: (formal_parameters) @para
    (#eq? @ident "maxValue")
    (#eq? @para "()")
)
"#;
const COUNTUNIQUE: &str = r#"
(method_declaration
    type: (integral_type)
    name: (identifier) @ident
	parameters: (formal_parameters) @para
    (#eq? @ident "countUniqueIntegers")
    (#eq? @para "()")
)
"#;
