use std::{
    collections::HashMap,
    path::{Path, PathBuf},
};

use anyhow::{bail, ensure, Context, Result};
use glob::glob;
use inquire::{error::InquireError, Select};
use java_dependency_analyzer::*;
use lazy_static::lazy_static;
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
}

type Dict = HashMap<String, String>;

struct JavaFile<'a> {
    path: &'a Path,
    file_name: &'a str,
    package_name: Option<&'a String>,
    imports: Option<Vec<Dict>>,
    class_name: Option<&'a String>,
    test_methods: Option<Vec<Dict>>,
}

impl<'a> JavaFile<'a> {
    fn new(path: &'a Path) -> Result<Self> {
        let source_code = std::fs::read_to_string(path)
            .with_context(|| format!("Could not read file: {:?}", path))?;

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

        let package_name = package_name[0].get("name");

        let class_name = parser.query(CLASSNAME_QUERY)?;

        ensure!(
            class_name.len() == 1,
            "Expected exactly one class name, found {}.",
            class_name.len()
        );

        let class_name = class_name[0].get("name");

        let test_methods = parser.query(TEST_ANNOTATION_QUERY)?;
        let test_methods = if test_methods.is_empty() {
            None
        } else {
            Some(test_methods)
        };

        Ok(Self {
            path,
            file_name: path.file_name().unwrap().to_str().unwrap(),
            package_name,
            imports,
            class_name,
            test_methods,
        })
    }
}

pub fn run_prompt() -> Result<()> {
    Ok(())
}

pub fn check_prompt() -> Result<()> {
    // for r in find_files("java", 5, &ROOT_DIR)? {
    //     for result in analyze_file(&r)? {
    //         println!(
    //             "For {:?} -> {:?}; with asterik?: {}",
    //             r,
    //             result.get("path").unwrap(),
    //             result.get("asterik").is_some()
    //         );
    //     }
    // }

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
