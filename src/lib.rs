use anyhow::{bail, ensure, Context, Result};
use colored::*;
use glob::glob;
use inquire::{error::InquireError, Select};
use java_dependency_analyzer::*;
use lazy_static::lazy_static;
use std::{
    collections::HashMap,
    path::{Path, PathBuf},
};
use which::which;

lazy_static! {
    static ref ROOT_DIR: PathBuf = PathBuf::from(".");
    static ref SOURCE_DIR: PathBuf = PathBuf::from(".").join("src");
    static ref BUILD_DIR: PathBuf = PathBuf::from(".").join("target");
    static ref TEST_DIR: PathBuf = PathBuf::from(".").join("test");
    static ref LIB_DIR: PathBuf = PathBuf::from(".").join("lib");
    static ref UMM_DIR: PathBuf = PathBuf::from(".").join(".umm");
    static ref JAVA_PATH: Result<PathBuf> =
        which("java").context("Failed to find `java` executable on path.");
    static ref JAVAC_PATH: Result<PathBuf> =
        which("javac").context("Failed to find `javac` executable on path.");
    static ref SEPARATOR: &'static str = if cfg!(windows) { ";" } else { ":" };
    static ref JAVA_TS_LANG: tree_sitter::Language = tree_sitter_java::language();
    static ref PROJECT: Result<JavaProject> = JavaProject::new();
}
type Dict = HashMap<String, String>;

#[derive(Debug, Clone)]
enum JavaFileType {
    Interface,
    Class,
    Test,
}

#[derive(Debug, Clone)]
struct JavaFile {
    path: PathBuf,
    file_name: String,
    package_name: Option<String>,
    imports: Option<Vec<Dict>>,
    name: Option<String>,
    proper_name: Option<String>,
    test_methods: Option<Vec<Dict>>,
    kind: JavaFileType,
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
        let package_name = parser.query(PACKAGE_QUERY)?;

        ensure!(
            package_name.len() == 1,
            "Expected exactly one package name, found {}.",
            package_name.len()
        );

        let package_name = package_name[0].get("name").map(String::to_owned);
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

        ensure!(
            name.len() == 1,
            "Expected exactly one class/interface name, found {}.",
            name.len()
        );
        let name = name[0].get("name").map(String::to_owned);

        let test_methods = parser.query(TEST_ANNOTATION_QUERY)?;
        let test_methods = if test_methods.is_empty() {
            None
        } else {
            kind = JavaFileType::Test;
            Some(test_methods)
        };

        let proper_name = if package_name.is_some() {
            format!(
                "{}.{}",
                package_name.as_ref().unwrap().yellow(),
                name.as_ref().unwrap().blue()
            )
        } else {
            format!("{}", name.as_ref().unwrap().blue())
        };

        Ok(Self {
            path: path.to_owned(),
            file_name: path.file_name().unwrap().to_str().unwrap().to_string(),
            package_name,
            imports,
            proper_name: Some(proper_name),
            name,
            test_methods,
            kind,
        })
    }
}

struct JavaProject {
    files: Vec<JavaFile>,
    names: Vec<String>,
}

impl JavaProject {
    fn new() -> Result<Self> {
        let mut files = vec![];
        let mut names = vec![];

        for path in find_files("java", 5, &ROOT_DIR)? {
            let file = JavaFile::new(path)?;
            names.push(file.proper_name.clone().unwrap());
            files.push(file);
        }

        Ok(Self { files, names })
    }
}

pub fn run_prompt() -> Result<()> {
    Ok(())
}

pub fn check_prompt() -> Result<()> {
    Ok(())
}

pub fn test_prompt() -> Result<()> {
    Ok(())
}

pub fn clean() -> Result<()> {
    Ok(())
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
