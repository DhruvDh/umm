use clap::{App, SubCommand};
use colored::*;
use glob::glob;
use std::fs::File;
use std::io::{Read, Write};
use std::{path::PathBuf, process::Command};

use anyhow::{bail, Context, Result};

fn root_dir() -> PathBuf {
    PathBuf::from("./")
}

fn build_dir() -> PathBuf {
    root_dir().join("target/")
}

fn source_dir() -> PathBuf {
    root_dir().join("src/")
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
    
    Ok(String::from_utf8(output)
        .context("Failed to parse output.")?
        .trim()
        .to_string())
}

fn find_classpath() -> Result<String> {
    let mut files = Vec::new();

    files.push(root_dir().display().to_string());
    files.push(build_dir().display().to_string());

    let pattern_1 = format!("{}**/**/*.class", build_dir().display());
    let pattern_2 = format!("{}**/**/*.jar", umm_files().display());

    let paths = glob(&pattern_1).context("Failed to glob for pattern {build_dir}**/**/*.class.")?;
    let paths = paths
        .chain(glob(&pattern_2).context("Failed to glob for pattern {build_dir}**/**/*.jar.")?);

    for entry in paths {
        match entry {
            Ok(path) => {
                let path = format!("{}", path.display());
                files.push(path)
            }
            Err(e) => bail!("while globbing: {}", e),
        }
    }

    Ok(files.join(":"))
}

fn compile(path: &PathBuf) -> Result<()> {
    let name = path.file_name().unwrap().to_str().unwrap();

    print!("{} {}", "Compiling".bright_green().bold(), path.display());

    if !path.exists() {
        bail!(
            "{}: {} does not exist.\n {}: All source files must be inside the src directory",
            "Fail".bright_red().bold(),
            path.display(),
            "Note".bright_yellow().bold()
        );
    }

    let javac_path = find("javac")?;

    let output = match Command::new(javac_path)
        .arg("-cp")
        .arg(find_classpath()?)
        .arg("-d")
        .arg(build_dir().as_path().to_str().unwrap())
        .arg("-sourcepath")
        .arg(root_dir().as_path().to_str().unwrap())
        .arg("-Xlint:unchecked")
        .arg(path)
        .output()
    {
        Ok(output) => output,
        Err(e) => bail!("Failed to execute javac for {}: {}", name, e),
    };

    match String::from_utf8(output.stderr) {
        Ok(err) => {
            if err.len() > 0 {
                print!("\n{}", err);
            } else {
                println!(" {}", "✔".bright_green());
            }
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

    Ok(())
}

fn test(path: &PathBuf) -> Result<()> {
    let name = path.file_name().unwrap().to_str().unwrap();
    let java_path = find("java")?;

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
        .arg("--scan-classpath")
        .arg("--details=tree")
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

    let output = match Command::new(java_path)
        .arg("-cp")
        .arg(classpath)
        .arg(path.file_stem().unwrap().to_str().unwrap())
        .output()
    {
        Ok(output) => output,
        Err(e) => bail!("Failed to execute java run command for {}: {}", name, e),
    };

    match String::from_utf8(output.stderr) {
        Ok(err) => {
            if err.len() > 0 {
                println!("{}", err);
            } else {
                println!(
                    "Note: There were no errors or warnings while running\t{}!",
                    path.display()
                );
            }
        }
        Err(e) => bail!("Failed to parse stderr for {}: {}", name, e),
    };

    match String::from_utf8(output.stdout) {
        Ok(out) => {
            if out.len() > 0 {
                println!("--------------- OUTPUT ---------------");
                println!("{}", out);
            }
        }
        Err(e) => bail!("Failed to parse stderr for {}: {}", name, e),
    };

    Ok(())
}

fn create_app() -> App<'static, 'static> {
    App::new("Umm")
        .version("0.0.1")
        .about("A Java build for novices")
        .subcommand(
            SubCommand::with_name("check")
                .about("Checks the java file for syntax errors.")
                .version("0.0.1")
                .args_from_usage("<FILE_NAME>              'the Java file to check'"),
        )
        .subcommand(
            SubCommand::with_name("run")
                .about("Runs the java file and shows the output.")
                .version("0.0.1")
                .args_from_usage("<FILE_NAME>              'the Java file to run'"),
        )
        .subcommand(
            SubCommand::with_name("test")
                .about("Runs the given junit test file.")
                .version("0.0.1")
                .args_from_usage("<FILE_NAME>              'the Java file to test'"),
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

    let files = vec![
        "hamcrest-core-1.3.jar",
        "junit-4.13.2.jar",
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

    Ok(())
}

fn main() -> Result<()> {
    let app = create_app();
    let matches = app.get_matches();

    match matches.subcommand_name() {
        Some("check") => {
            init()?;

            let path = matches
                .subcommand_matches("check")
                .unwrap()
                .value_of("FILE_NAME")
                .unwrap();

            compile(&source_dir().join(path))?;
        }
        Some("run") => {
            init()?;

            let path = matches
                .subcommand_matches("run")
                .unwrap()
                .value_of("FILE_NAME")
                .unwrap();
            compile(&source_dir().join(path))?;
            run(&source_dir().join(path))?;
        }
        Some("test") => {
            init()?;

            let path = matches
                .subcommand_matches("test")
                .unwrap()
                .value_of("FILE_NAME")
                .unwrap();
            compile(&source_dir().join(path))?;
            test(&source_dir().join(path))?;
        }
        Some("clean") => {
            clean(&build_dir());
            clean(&umm_files());
        }
        _ => {
            create_app().print_long_help().unwrap();
        }
    };

    Ok(())
}
