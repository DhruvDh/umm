use clap::{App, SubCommand};
use colored::*;
use glob::glob;
use std::collections::BTreeSet;
use std::fs::File;
use std::io::{Read, Write};
use std::process::Stdio;
use std::{path::PathBuf, process::Command};

use java_import_parser::*;
use miette::{bail, Result};

fn root_dir() -> PathBuf {
    PathBuf::from("./")
}

fn build_dir() -> PathBuf {
    root_dir().join("target/")
}

fn source_dir() -> PathBuf {
    root_dir().join("src/")
}

fn test_dir() -> PathBuf {
    root_dir().join("test/")
}

fn umm_files() -> PathBuf {
    root_dir().join(".umm_files/")
}

fn find(name: &str) -> Result<String> {
    let output = match Command::new("which").arg(name).output() {
        Ok(output) => output.stdout,
        Err(e) => bail!("Failed to find {} executable: {}", name, e),
    };

    if output.is_empty() {
        bail!("Failed to find {} executable", name);
    }

    match String::from_utf8(output) {
        Ok(s) => Ok(s.trim().to_string()),
        Err(_) => bail!("Failed to parse output."),
    }
}

fn find_sourcepath() -> Result<String> {
    let mut files = Vec::new();

    files.push(root_dir().display().to_string());
    files.push(build_dir().display().to_string());

    let pattern_1 = format!("{}**/**/**/**/*.java", root_dir().display());

    let paths = match glob(&pattern_1) {
        Ok(paths) => paths,
        Err(_e) => bail!("Failed to glob for pattern {build_dir}**/**/*.class."),
    };

    for entry in paths {
        match entry {
            Ok(path) => {
                let path = format!("{}", path.parent().unwrap_or(&root_dir()).display());
                files.push(path)
            }
            Err(e) => bail!("while globbing: {}", e),
        }
    }

    let files: BTreeSet<String> = files.iter().map(|s| s.to_string()).collect();
    let files: Vec<String> = files.into_iter().collect();

    Ok(files.join(":"))
}

fn find_classpath() -> Result<String> {
    let mut files = Vec::new();

    files.push(root_dir().display().to_string());
    files.push(build_dir().display().to_string());

    let pattern_1 = format!("{}**/**/**/**/*.class", build_dir().display());
    let pattern_2 = format!("{}**/**/**/**/*.jar", umm_files().display());

    let paths = match glob(&pattern_1) {
        Ok(paths) => paths,
        Err(_e) => bail!("Failed to glob for pattern {build_dir}**/**/*.class."),
    };

    let paths = paths.chain(match glob(&pattern_2) {
        Ok(paths) => paths,
        Err(_e) => bail!("Failed to glob for pattern {umm_files}**/**/*.jar."),
    });

    for entry in paths {
        match entry {
            Ok(path) => {
                if path.extension().unwrap() == "jar" {
                    let path = format!("{}", path.display());
                    files.push(path)
                } else {
                    let path = format!("{}", path.parent().unwrap_or(&root_dir()).display());
                    files.push(path)
                }
            }
            Err(e) => bail!("while globbing: {}", e),
        }
    }

    files.push(build_dir().as_path().to_str().unwrap().to_string());

    let files: BTreeSet<String> = files.iter().map(|s| s.to_string()).collect();
    let files: Vec<String> = files.into_iter().collect();

    Ok(files.join(":"))
}

fn get_parse_result(path: &PathBuf) -> Result<ParseResult> {
    let name = path.file_name().unwrap().to_str().unwrap();

    let source = match std::fs::read_to_string(path) {
        Ok(source) => source,
        Err(e) => bail!("Failed to read {}: {}", name, e),
    };

    parse(&source, name).into()
}

fn starts_with_one_of(s: &String, prefixes: &[&str]) -> bool {
    for prefix in prefixes {
        if s.starts_with(prefix) {
            return true;
        }
    }

    false
}

fn compile(path: &PathBuf, look_at_package: bool) -> Result<()> {
    if path.extension() != Some(std::ffi::OsStr::new("java")) {
        bail!("{} is not a java file.", path.display());
    }

    let name = path.file_name().unwrap().to_str().unwrap();

    if starts_with_one_of(
        &String::from(name),
        &[
            "ArrayListStack.java",
            "ArrayStack.java",
            "CArrayList.java",
            "DoublyLinkedList.java",
            "DoublyLinkedNode.java",
            "DoublyLinkedListTest.java",
            "SinglyLinkedList.java",
            "SinglyLinkedNode.java",
            "SinglyLinkedListTest.java",
        ],
    ) {
        return Ok(());
    }

    if name == "*.java" {
        return Ok(());
    }

    if !path.exists() {
        bail!(
            "{} does not exist.\n{}: All source files must be inside the {} directory",
            path.display(),
            "Note".bright_green().bold(),
            source_dir().display()
        );
    }

    let result = get_parse_result(&path)?;
    let package = result.package_name.unwrap_or("".to_string());
    let imports = result.imports.unwrap_or(Vec::new());
    let mut class_name = result.class_name;

    if name.strip_suffix(".java").unwrap_or(name) != class_name {
        bail!(
            "{} should have a class name which is same as this file name for this tool to work. Current name is {}",
            path.display(),
            name
        );
    }

    if !package.is_empty() {
        if path
            .parent()
            .unwrap()
            .file_name()
            .unwrap()
            .to_str()
            .unwrap()
            != package
        {
            bail!(
                "{} belongs to package {} but is not in a folder called {}\n{}: All source files for package {} be inside a {} directory",
                path.display(),
                package,
                package,
                "Note".bright_green().bold(),
                package,
                package
            );
        }

        if look_at_package {
            let pattern_1 = format!("{}/**/**/*.java", source_dir().join(&package).display());
            let pattern_2 = format!("{}/**/**/*.java", test_dir().join(&package).display());

            let paths = match glob(&pattern_1) {
                Ok(paths) => paths,
                Err(_) => bail!("Failed to glob for pattern {source_dir}**/**/*.java."),
            };

            let paths = paths.chain(match glob(&pattern_2) {
                Ok(paths) => paths,
                Err(_e) => bail!("Failed to glob for pattern {test_dir}**/**/*.java."),
            });

            for entry in paths {
                match entry {
                    Ok(path) => compile(&path, false)?,
                    Err(e) => bail!("while globbing: {}", e),
                }
            }
        }
    }

    for import in imports {
        if starts_with_one_of(
            &import[0],
            &[
                "java",
                "org",
                "com",
                "edu",
                &package,
                "DataStructures",
                "Exceptions",
                "ADTs",
            ],
        ) {
            continue;
        } else {
            let mut new_path = source_dir();
            for part in import {
                new_path = new_path.join(part);
            }

            compile(&new_path.with_extension("java"), true)?;
        }
    }

    let javac_path = find("javac")?;

    let output = match Command::new(javac_path)
        .arg("-cp")
        .arg(find_classpath()?)
        .arg("-d")
        .arg(build_dir().as_path().to_str().unwrap())
        .arg("-sourcepath")
        .arg(find_sourcepath()?)
        .arg(&path)
        .output()
    {
        Ok(output) => output,
        Err(e) => bail!("Failed to execute javac for {}: {}", name, e),
    };

    match String::from_utf8(output.stderr) {
        Ok(err) => {
            if err.len() > 0 {
                print!("\n{}", err);
            }
            // } else {
            //     println!(" {}", "✔".bright_green());
            // }
        }
        Err(e) => bail!("Failed to parse stderr for {}: {}", name, e),
    };

    match String::from_utf8(output.stdout) {
        Ok(out) => {
            if out.len() > 0 {
                println!("{}", out);
            }
        }
        Err(e) => bail!("Failed to parse stderr for {}: {}", name, e),
    };

    if output.status.success() {
        println!(
            "{} {}\tExit status: {}",
            "Compiled".bright_green().bold(),
            path.display(),
            "✔".bright_green()
        );
    } else {
        println!(
            "{} {}\tExit status: {}",
            "Compiled".bright_red().bold(),
            path.display(),
            "✘".bright_red()
        );
    };

    Ok(())
}

fn test(path: &PathBuf) -> Result<()> {
    let name = path.file_name().unwrap().to_str().unwrap();
    let java_path = find("java")?;

    let result = get_parse_result(path)?;
    let mut class_name = result.class_name;
    let package = result.package_name.unwrap_or("".to_string());

    if package.trim().len() > 0 {
        class_name = format!("{}.{}", package, class_name);
    }

    println!(
        "{} tests for class {}",
        "Running".bright_yellow().bold(),
        class_name,
    );

    let output = match Command::new(&java_path)
        .arg("-jar")
        .arg(
            &umm_files()
                .join("lib/junit-platform-console-standalone-1.8.0-RC1.jar")
                .as_path()
                .to_str()
                .unwrap(),
        )
        .arg("--disable-banner")
        .arg("-cp")
        .arg(find_classpath()?)
        // .arg("--scan-classpath")
        .arg("-c")
        .arg(class_name)
        .output()
    {
        Ok(output) => output,
        Err(e) => bail!("Failed to execute java test command for {}: {}", name, e),
    };

    match String::from_utf8(output.stderr) {
        Ok(err) => {
            if err.len() > 0 {
                println!("{}", err);
            }
        }
        Err(e) => bail!("Failed to parse stderr for {}: {}", name, e),
    };

    match String::from_utf8(output.stdout) {
        Ok(out) => {
            if out.len() > 0 {
                let mut out = out;
                if let Some(location) = out.find("Test run finished after") {
                    out.truncate(location);
                    println!("{}", out.trim());
                }
            }
        }
        Err(e) => bail!("Failed to parse stderr for {}: {}", name, e),
    };

    Ok(())
}

fn run(path: &PathBuf) -> Result<()> {
    let name = path.file_name().unwrap().to_str().unwrap();
    let java_path = find("java")?;
    let classpath = find_classpath()?;
    let classpath = format!("{}:{}", classpath, build_dir().as_path().to_str().unwrap());

    let result = get_parse_result(&path)?;
    let package = result.package_name.unwrap_or("".to_string());
    let mut class_name = result.class_name;

    if !package.trim().is_empty() {
        class_name = format!("{}.{}", package, class_name);
    }

    let mut child = match Command::new(java_path)
        .stdin(Stdio::inherit())
        .stdout(Stdio::inherit())
        .arg("-cp")
        .arg(classpath)
        .arg(class_name)
        .spawn()
    {
        Ok(c) => c,
        Err(e) => bail!("Failed to execute java run command for {}: {}", name, e),
    };

    // match String::from_utf8(output.stderr) {
    //     Ok(err) => {
    //         if err.len() > 0 {
    //             println!("{}", err);
    //         } else {
    //             println!(
    //                 "{}: There were no errors or warnings while running {}!",
    //                 "Note".bright_green().bold(),
    //                 path.display()
    //             );
    //         }
    //     }
    //     Err(e) => bail!("Failed to parse stderr for {}: {}", name, e),
    // };

    // match String::from_utf8(output.stdout) {
    //     Ok(out) => {
    //         if out.len() > 0 {
    //             println!("\n--------------- OUTPUT ---------------");
    //             println!("{}", out);
    //         }
    //     }
    //     Err(e) => bail!("Failed to parse stderr for {}: {}", name, e),
    // };

    match child.wait_with_output() {
        Ok(status) => {
            if status.status.success() {
                println!(
                    "{} {}\tExit status: {}",
                    "Ran".bright_green().bold(),
                    path.display(),
                    "✔".bright_green()
                );
            } else {
                println!(
                    "{} {}\tExit status: {}",
                    "Ran".bright_red().bold(),
                    path.display(),
                    "✘".bright_red()
                );
            }
        }
        Err(e) => bail!("Failed to wait for child process for {}: {}", name, e),
    };

    Ok(())
}

fn create_app() -> App<'static, 'static> {
    App::new("Umm")
        .version("0.0.1")
        .about("A Java build for novices")
        .subcommand(
            SubCommand::with_name("check")
                .about("Checks the java file for syntax errors. (For this lab there is no need to specify a file path)")
                .version("0.0.1"),
        )
        .subcommand(
            SubCommand::with_name("run")
                .about("Runs the java file and shows the output. (For this lab there is no need to specify a file path)")
                .version("0.0.1")
        )
        .subcommand(
            SubCommand::with_name("test")
                .about("Runs the given junit test file. (For this lab there is nothing to test)")
                .version("0.0.1"),
        )
        .subcommand(
            SubCommand::with_name("clean")
                .about("Cleans all compiled classes")
                .version("0.0.1"),
        )
}

fn clean(path: &PathBuf) {
    std::fs::remove_dir_all(path).unwrap_or(());
    std::fs::create_dir_all(path).unwrap_or(());
}

fn download(url: &str, path: &PathBuf) -> Result<()> {
    let resp = match ureq::get(url).call() {
        Ok(resp) => resp,
        Err(e) => bail!("Failed to download {}: {}", url, e),
    };

    let len = resp
        .header("Content-Length")
        .and_then(|s| s.parse::<usize>().ok())
        .unwrap();

    let mut bytes: Vec<u8> = Vec::with_capacity(len);

    match resp.into_reader().take(10_000_000).read_to_end(&mut bytes) {
        Ok(bytes) => bytes,
        Err(e) => bail!(
            "Failed to read response till the end while downloading file at {}: {}",
            url,
            e
        ),
    };

    let name = path.file_name().unwrap().to_str().unwrap();

    let mut file = match File::create(path) {
        Ok(file) => file,
        Err(e) => bail!("Failed to create file at {}: {}", name, e),
    };

    match file.write_all(&bytes) {
        Ok(_) => Ok(()),
        Err(e) => bail!("Failed to write to file at {}: {}", name, e),
    }
}

fn init() -> Result<()> {
    std::fs::create_dir_all(&umm_files().join("lib")).unwrap_or(());
    std::fs::create_dir_all(&umm_files().join("target")).unwrap_or(());
    std::fs::create_dir_all(&umm_files().join("target").join("ADTs")).unwrap_or(());
    std::fs::create_dir_all(&umm_files().join("target").join("DataStructures")).unwrap_or(());
    std::fs::create_dir_all(&umm_files().join("target").join("Exceptions")).unwrap_or(());

    let files = vec![
        "junit-platform-console-standalone-1.8.0-RC1.jar",
        "junit-platform-runner-1.8.0.jar",
    ];

    for file in files {
        if !umm_files().join("lib/").join(file).as_path().exists() {
            println!("{} {}", "Downloading".bright_yellow().bold(), file);
            download(
                format!(
                    "https://github.com/DhruvDh/umm/raw/main/jars_files/{}",
                    file
                )
                .as_str(),
                &umm_files().join("lib/").join(file),
            )?;
        }
    }

    if !umm_files().join("target.zip").exists() {
        println!(
            "{}",
            "Downloading pre-compiled files".bright_yellow().bold()
        );
        download(
            format!("https://www.dropbox.com/s/t7n89fv1887hacx/target.zip?raw=true").as_str(),
            &umm_files().join("target.zip"),
        )?;
    }

    let output = Command::new("unzip")
        .arg("-n")
        .arg(&umm_files().join("target.zip"))
        .output()
        .unwrap();

    if output.status.success() {
        println!("{}", "Extracting them..".bright_yellow().bold());
    } else {
        bail!(
            "Failed to unzip target.zip: {}",
            String::from_utf8(output.stderr).unwrap()
        );
    }

    Ok(())
}

fn main() -> Result<()> {
    let app = create_app();
    let matches = app.get_matches();

    match matches.subcommand_name() {
        Some("check") => {
            init()?;
            println!("{}", "For this lab, I will always only check the following files when you say `umm check`.".bright_yellow().bold());
            println!(
                "{}",
                "1. DataStructures.LinkedStack (The file you completed in Lab 2)"
                    .bright_green()
                    .bold()
            );
            println!(
                "{}",
                "2. DataStructures.LinkedQueue (File you need to complete in Lab 3)"
                    .bright_green()
                    .bold()
            );
            println!(
                "{}",
                "3. DataStructures.ArrayQueue (File you need to complete in Lab 3)"
                    .bright_green()
                    .bold()
            );
            println!(
                "{}",
                "4. Apps.RepeatStrings class (File you need to complete in Lab 3)"
                    .bright_green()
                    .bold()
            );

            compile(
                &source_dir().join("DataStructures").join("LinkedStack.java"),
                false,
            )?;
            compile(
                &source_dir().join("DataStructures").join("LinkedQueue.java"),
                false,
            )?;
            compile(
                &source_dir().join("DataStructures").join("ArrayQueue.java"),
                false,
            )?;
            compile(&source_dir().join("Apps").join("RepeatStrings.java"), false)?;
        }
        Some("run") => {
            init()?;

            println!(
                "{}",
                "For this lab, I will always only run the following files when you say `umm run`."
                    .bright_yellow()
                    .bold()
            );
            println!(
                "{}",
                "1. Apps.RepeatStrings (The file you have to complete in Lab 3)"
                    .bright_green()
                    .bold()
            );
            
            compile(
                &source_dir().join("DataStructures").join("ArrayQueue.java"),
                false,
            )?;
            compile(
                &source_dir().join("DataStructures").join("LinkedQueue.java"),
                false,
            )?;
            compile(&source_dir().join("Apps").join("RepeatStrings.java"), false)?;
            run(&source_dir().join("Apps").join("RepeatStrings.java"))?;
        }
        Some("test") => {
            init()?;
            println!(
                "{}",
                "For this lab, there is nothing to test."
                    .bright_yellow()
                    .bold()
            );
        }
        Some("clean") => {
            clean(&umm_files());
        }
        _ => {
            create_app().print_long_help().unwrap();
        }
    };

    Ok(())
}
