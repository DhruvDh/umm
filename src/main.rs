//! # umm
//! ## Introduction

//! A java build tool for novices.

//! ## Installation

//! You would need rust installed, ideally the nightly toolchain. You can visit https://rustup.rs/ to find out how to install this on your computer, just make sure you install the "nightly" toolchain instead of stable.

//! On Linux, Windows Subsystem for Linux (WSL), and Mac you should be able to run `curl https://sh.rustup.rs -sSf | sh -s -- --default-toolchain nightly` on a terminal to install the nightly toolchain for rust.

//! Once you are done, just type `cargo install --git=https://github.com/DhruvDh/umm.git` and it should compile and install it on your system.

#![warn(missing_docs)]
#![warn(clippy::missing_docs_in_private_items)]

use anyhow::Result;
use bpaf::*;
use dotenvy::dotenv;
use self_update::cargo_crate_version;
use tracing::{
    metadata::LevelFilter,
    Level,
};
use tracing_subscriber::{
    fmt,
    prelude::*,
    util::SubscriberInitExt,
};
use umm::{
    clean,
    grade,
    java::Project,
};

/// Updates binary based on github releases
fn update() -> Result<()> {
    let status = self_update::backends::github::Update::configure()
        .repo_owner("dhruvdh")
        .repo_name("umm")
        .bin_name("umm")
        .target_version_tag("summer_22")
        .show_download_progress(true)
        .show_output(false)
        .current_version(cargo_crate_version!())
        .no_confirm(true)
        .build()?
        .update()?;
    eprintln!("Update status: `{}`!", status.version());
    Ok(())
}

/// Enum to represent different commands
#[derive(Debug, Clone)]
enum Cmd {
    /// Run a file
    Run(String),
    /// Check a file
    Check(String),
    /// Test a file
    Test(String, Vec<String>),
    /// Check a files documentation
    DocCheck(String),
    /// Grade a file
    Grade(String),
    /// Clean the project artifacts
    Clean,
    /// Print information about the project
    Info,
    /// Update the command
    Update,
    /// Checks project health
    CheckHealth,
    /// Resets the project metadata, and redownloads libraries
    Reset,
    /// Exit the program
    Exit,
}

/// Parse the command line arguments and return a `Cmd` enum
fn options() -> Cmd {
    /// parses test names
    fn t() -> impl Parser<Vec<String>> {
        positional("TESTNAME")
            .help("Name of JUnit test to run")
            .many()
    }

    /// parsers file name
    fn f() -> impl Parser<String> {
        positional("FILENAME").help("Name of java file")
    }

    /// parses Assignment name or path to grading script file
    fn g() -> impl Parser<String> {
        positional("NAME/PATH").help("Name of assignment in database or path to grading script")
    }

    let run = construct!(Cmd::Run(f()))
        .to_options()
        .command("run")
        .help("Run a java file with a main method");

    let check = construct!(Cmd::Check(f()))
        .to_options()
        .command("check")
        .help("Check for syntax errors");

    let test = construct!(Cmd::Test(f(), t()))
        .to_options()
        .command("test")
        .help("Run JUnit tests");

    let doc_check = construct!(Cmd::DocCheck(f()))
        .to_options()
        .command("dock-check")
        .help("Check a file for missing javadoc");

    let grade = construct!(Cmd::Grade(g()))
        .to_options()
        .command("grade")
        .help("Grade your work");

    let clean = pure(Cmd::Clean)
        .to_options()
        .command("clean")
        .help("Cleans the build folder, library folder, and vscode settings");

    let info = pure(Cmd::Info)
        .to_options()
        .command("info")
        .help("Prints a JSON description of the project as parsed");

    let update = pure(Cmd::Update)
        .to_options()
        .command("update")
        .help("Update the umm command");

    let check_health = pure(Cmd::CheckHealth)
        .to_options()
        .command("check-health")
        .help("Checks the health of the project");

    let reset = pure(Cmd::Reset)
        .to_options()
        .command("reset")
        .help("Reset the project metadata, and redownload libraries");

    let exit = pure(Cmd::Exit)
        .to_options()
        .command("exit")
        .help("Exit the program");

    let cmd = construct!([
        run,
        check,
        test,
        doc_check,
        grade,
        clean,
        info,
        update,
        check_health,
        reset,
        exit
    ])
    .fallback(Cmd::Exit);

    cmd.to_options().descr("Build tool for novices").run()
}

fn main() -> Result<()> {
    dotenv().ok();

    let fmt = fmt::layer()
        .without_time()
        .with_file(false)
        .with_line_number(false);
    let filter_layer = LevelFilter::from_level(Level::INFO);
    tracing_subscriber::registry()
        .with(fmt)
        .with(filter_layer)
        .init();

    let cmd = options();

    // TODO: move this to a separate method and call that method in shell()
    match cmd {
        Cmd::Run(f) => Project::new()?.identify(f.as_str())?.run()?,
        Cmd::Check(f) => Project::new()?.identify(f.as_str())?.check()?,
        Cmd::Test(f, t) => {
            let out = if t.is_empty() {
                Project::new()?.identify(f.as_str())?.test(vec![])?
            } else {
                let t = t.iter().map(|i| i.as_str()).collect();
                Project::new()?.identify(f.as_str())?.test(t)?
            };

            let out = [
                String::from_utf8(out.stderr)?,
                String::from_utf8(out.stdout)?,
            ]
            .concat();

            println!("{out}");
        }
        Cmd::DocCheck(f) => {
            let out = Project::new()?.identify(f.as_str())?.doc_check()?;
            println!("{out}");
        }
        Cmd::Grade(g) => grade(&g)?,
        Cmd::Clean => clean()?,
        Cmd::Info => Project::new()?.info()?,
        Cmd::Update => {
            match update() {
                Ok(_) => {}
                Err(e) => eprintln!("{e}"),
            };
        }
        Cmd::CheckHealth => Project::new()?.check_health()?,
        Cmd::Reset => {
            clean()?;
            Project::new()?;
        }
        Cmd::Exit => {}
    };

    Ok(())
}
