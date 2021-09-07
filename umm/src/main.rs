// check
// run
// test
use clap::{App, Arg, SubCommand};
use glob::glob;
use std::{path::PathBuf, process::Command};

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
    let mut paths = glob("../umm_test/*.jar").expect("Failed to glob.");
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
        .arg("../umm_test/bin/")
        .arg("-sourcepath")
        .arg("../umm_test/")
        .arg("-Xlint:unchecked")
        .arg(path)
        .output()
        .expect(format!("Failed to compile {}.", path.display()).as_str());


    let err = String::from_utf8(output.stderr).expect("Failed to parse output.");

    let output = String::from_utf8(output.stdout).expect("Failed to parse output.");

    if err.len() > 0 {
        println!("{}", err);
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
                .arg(
                    Arg::with_name("FILE_NAME")
                        .short("f")
                        .help("name of the file to check"),
                ),
        )
        .subcommand(
            SubCommand::with_name("run")
                .about("runs the java file and shows the output")
                .version("0.0.1")
                .arg(
                    Arg::with_name("FILE_NAME")
                        .short("f")
                        .help("name of the file to check"),
                ),
        )
        .subcommand(
            SubCommand::with_name("test")
                .about("runs the given junit test file")
                .version("0.0.1")
                .arg(
                    Arg::with_name("FILE_NAME")
                        .short("f")
                        .help("name of the file to check"),
                ),
        )
        .get_matches();

    match matches.subcommand_name() {
        Some("check") => {
            for entry in glob("../umm_test/*.java").expect("Failed to glob.") {
                match entry {
                    Ok(path) => {
                        compile(&path);
                    }
                    Err(e) => println!("{:?}", e),
                }  
            }
        }
        Some("run") => {}
        Some("test") => {}
        _ => {}
    };
}
