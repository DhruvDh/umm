// check
// run
// test
// clean
use clap::{App, SubCommand};
use glob::glob;
use std::{path::PathBuf, process::Command};

fn root_dir() -> PathBuf {
    PathBuf::from("../umm_test/")
}

fn build_dir() -> PathBuf {
    root_dir().join("bin")
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
    let pattern = format!("{}/*.jar", root_dir().display());

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
        .arg(classpath)
        .arg("org.junit.runner.JUnitCore")
        .arg(path.file_stem().unwrap().to_str().unwrap())
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

fn main() {
    let matches = App::new("Umm")
        .version("0.0.1")
        .author("dhruvdh\nAnh")
        .about("A Java build for novices")
        .subcommand(
            SubCommand::with_name("check")
                .about("checks the java file for syntax errors")
                .version("0.0.1")
                .args_from_usage("<FILE_NAME>              'the Java file to check'"),
        )
        .subcommand(
            SubCommand::with_name("run")
                .about("runs the java file and shows the output")
                .version("0.0.1")
                .args_from_usage("<FILE_NAME>              'the Java file to run'"),
        )
        .subcommand(
            SubCommand::with_name("test")
                .about("runs the given junit test file")
                .version("0.0.1")
                .args_from_usage("<FILE_NAME>              'the Java file to test'"),
        )
        .subcommand(
            SubCommand::with_name("clean")
                .about("cleans all compiled classes")
                .version("0.0.1"),
        )
        .get_matches();

    match matches.subcommand_name() {
        Some("check") => {
            let path = matches
                .subcommand_matches("check")
                .unwrap()
                .value_of("FILE_NAME")
                .unwrap();

            compile(&root_dir().join(path));
        }
        Some("run") => {
            let path = matches
                .subcommand_matches("run")
                .unwrap()
                .value_of("FILE_NAME")
                .unwrap();
            compile(&root_dir().join(path));
            run(&root_dir().join(path));
        }
        Some("test") => {
            let path = matches
                .subcommand_matches("test")
                .unwrap()
                .value_of("FILE_NAME")
                .unwrap();
            compile(&root_dir().join(path));
            test(&root_dir().join(path));
        }
        Some("clean") => {
            std::fs::remove_dir_all(build_dir())
                .expect(format!("Failed to remove {} directory.", build_dir().display()).as_str());
            std::fs::create_dir_all(build_dir()).expect(
                format!(
                    "Failed to create {} directory after cleaning.",
                    build_dir().display()
                )
                .as_str(),
            );
        }
        _ => {}
    };
}
