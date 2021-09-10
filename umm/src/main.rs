// check
// run
// test
// clean
use clap::{App, SubCommand};
use glob::glob;
use std::fs::File;
use std::io::{Read, Write};
use std::{path::PathBuf, process::Command};

fn root_dir() -> PathBuf {
    PathBuf::from("./")
}

fn build_dir() -> PathBuf {
    root_dir().join("bin")
}

fn umm_files() -> PathBuf {
    root_dir().join(".umm_files")
}

fn find(name: &str) -> String {
    let output = Command::new("which")
        .arg(name)
        .output()
        .expect(format!("Failed to find {} executable.", name).as_str())
        .stdout;

    let output = String::from_utf8(output).expect("Failed to parse output.");
    output.trim().to_string()
}

fn find_jar_classpath() -> String {
    let mut jars = Vec::new();
    let pattern = format!("{}*.jar", root_dir().display());

    let paths = glob(&pattern).expect("Failed to glob.");
    for entry in paths {
        match entry {
            Ok(path) => {
                let path = format!("{}", path.display());
                jars.push(path)
            }
            Err(e) => println!("{:?}", e),
        }
    }

    let pattern = format!("{}*.jar", &umm_files().join("lib/").display());
    let paths = glob(&pattern).expect("Failed to glob.");
    for entry in paths {
        match entry {
            Ok(path) => {
                let path = format!("{}", path.display());
                jars.push(path)
            }
            Err(e) => println!("{:?}", e),
        }
    }

    jars.join(":")
}

fn compile(path: &PathBuf) {
    let javac_path = find("javac");

    let output = Command::new(javac_path)
        .arg("-cp")
        .arg(find_jar_classpath())
        .arg("-d")
        .arg(build_dir().as_path().to_str().unwrap())
        .arg("-sourcepath")
        .arg(root_dir().as_path().to_str().unwrap())
        .arg("-Xlint:unchecked")
        .arg(path)
        .output()
        .expect(format!("Failed to compile {}.", path.display()).as_str());

    let err = String::from_utf8(output.stderr).expect("Failed to parse stderr.");
    let err = err.trim();
    let output = String::from_utf8(output.stdout).expect("Failed to parse stdout.");
    let output = output.trim();

    if err.len() > 0 {
        println!("{}", err);
    } else {
        println!(
            "Note: There were no errors or warnings while compiling {}!",
            path.display()
        );
    }

    if output.len() > 0 {
        println!("{}", output);
    }
}

fn test(path: &PathBuf) {
    let java_path = find("java");
    let classpath = find_jar_classpath();
    let classpath = format!("{}:{}", classpath, build_dir().as_path().to_str().unwrap());

    let output = Command::new(java_path)
        .arg("-cp")
        .arg(&classpath)
        .arg("-jar")
        .arg(
            &umm_files()
                .join("lib/junit-platform-console-standalone-1.8.0-RC1.jar")
                .as_path()
                .to_str()
                .unwrap(),
        )
        // .arg("-f")
        // .arg(path.to_str().unwrap())
        .arg("--disable-banner")
        .arg("--scan-classpath")
        .arg(&classpath)
        .arg("--include-classname=.*")
        .output()
        .expect(format!("Failed to compile {}.", path.display()).as_str());

    let err = String::from_utf8(output.stderr).expect("Failed to parse stderr.");
    let err = err.trim();
    let output = String::from_utf8(output.stdout).expect("Failed to parse stdout.");
    let output = output.trim();

    if err.len() > 0 {
        println!("{}", err);
    }
    // } else {
    //     println!(
    //         "Note: There were no errors or warnings while testing\t{}!",
    //         path.display()
    //     );
    // }

    if output.len() > 0 {
        println!("--------------- OUTPUT ---------------");
        println!("{}", output);
    }
}

fn run(path: &PathBuf) {
    let java_path = find("java");
    let classpath = find_jar_classpath();
    let classpath = format!("{}:{}", classpath, build_dir().as_path().to_str().unwrap());

    let output = Command::new(java_path)
        .arg("-cp")
        .arg(classpath)
        .arg(path.file_stem().unwrap().to_str().unwrap())
        .output()
        .expect(format!("Failed to compile {}.", path.display()).as_str());

    let err = String::from_utf8(output.stderr).expect("Failed to parse stderr.");
    let err = err.trim();
    let output = String::from_utf8(output.stdout).expect("Failed to parse stdout.");
    let output = output.trim();

    if err.len() > 0 {
        println!("{}", err);
    } else {
        println!(
            "Note: There were no errors or warnings while running\t{}!",
            path.display()
        );
    }

    if output.len() > 0 {
        println!("--------------- OUTPUT ---------------");
        println!("{}", output);
    }
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

fn download(url: &str, path: &PathBuf) {
    let resp = ureq::get(url)
        .call()
        .expect(format!("Could not download file at {}", url).as_str());

    let len = resp
        .header("Content-Length")
        .and_then(|s| s.parse::<usize>().ok())
        .unwrap();

    let mut bytes: Vec<u8> = Vec::with_capacity(len);
    resp.into_reader()
        .take(10_000_000)
        .read_to_end(&mut bytes)
        .expect("Failed to read response till the end.");

    let mut file = File::create(path).expect("Failed to create file.");
    file.write_all(&bytes).expect("Failed to write to file.");
}

fn init() {
    std::fs::create_dir_all(&umm_files().join("lib")).unwrap_or(());

    let files = vec![
        "hamcrest-core-1.3.jar",
        "junit-4.13.2.jar",
        "junit-platform-console-standalone-1.8.0-RC1.jar",
        "junit-platform-runner-1.8.0.jar"
    ];

    for file in files {
        if !umm_files().join("lib/").join(file).as_path().exists() {
            println!("Downloading {}...", file);
            download(
                format!("https://github.com/DhruvDh/umm/raw/main/jars_files/{}", file).as_str(),
                &umm_files().join("lib/").join(file),
            );
        }
    }
}

fn main() {
    let app = create_app();
    let matches = app.get_matches();

    match matches.subcommand_name() {
        Some("check") => {
            init();

            let path = matches
                .subcommand_matches("check")
                .unwrap()
                .value_of("FILE_NAME")
                .unwrap();

            compile(&root_dir().join(path));
        }
        Some("run") => {
            init();

            let path = matches
                .subcommand_matches("run")
                .unwrap()
                .value_of("FILE_NAME")
                .unwrap();
            compile(&root_dir().join(path));
            run(&root_dir().join(path));
        }
        Some("test") => {
            init();

            let path = matches
                .subcommand_matches("test")
                .unwrap()
                .value_of("FILE_NAME")
                .unwrap();
            compile(&root_dir().join(path));
            test(&root_dir().join(path));
        }
        Some("clean") => {
            clean(&build_dir());
            clean(&umm_files());
        }
        _ => {
            create_app().print_long_help().unwrap();
        }
    };
}
