//! # umm
//! ## Introduction

//! A java build tool for novices.

//! ## Installation

//! You would need rust installed, ideally the nightly toolchain. You can visit https://rustup.rs/ to find out how to install this on your computer, just make sure you install the "nightly" toolchain instead of stable.

//! On Linux, Windows Subsystem for Linux (WSL), and Mac you should be able to run `curl https://sh.rustup.rs -sSf | sh -s -- --default-toolchain nightly` on a terminal to install the nightly toolchain for rust.

//! Once you are done, just type `cargo install --git=https://github.com/DhruvDh/umm.git` and it should compile and install it on your system.

#![warn(missing_docs)]
#![warn(clippy::missing_docs_in_private_items)]

use std::cmp::Ordering;

use anyhow::{
    anyhow,
    Result,
};
use bpaf::*;
use nu_ansi_term::{
    Color,
    Style,
};
use reedline::{
    default_emacs_keybindings,
    ColumnarMenu,
    DefaultCompleter,
    DefaultHinter,
    DefaultPrompt,
    Emacs,
    ExampleHighlighter,
    FileBackedHistory,
    KeyCode,
    KeyModifiers,
    Reedline,
    ReedlineEvent,
    ReedlineMenu,
    Signal,
};
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
    java::{
        FileType,
        Project,
    },
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

/// A utility method to start the terminal shell and execute requested commands.
fn shell() -> Result<()> {
    let prompt = DefaultPrompt::default();

    let mut commands: Vec<String> = vec![
        "test".into(),
        "run".into(),
        "grade".into(),
        "check".into(),
        "doc-check".into(),
        "clean".into(),
        "update".into(),
        "check-health".into(),
        "reset".into(),
        "info".into(),
        "clear".into(),
        "exit".into(),
        "history".into(),
    ];

    let mut project = Project::new()?;

    let mut test_methods = vec![];
    for file in project.files().iter() {
        match file.kind() {
            FileType::ClassWithMain => {
                commands.push(format!("run {}", file.file_name()));
            }
            FileType::Test => {
                commands.push(format!("test {}", file.file_name()));
                for method in file.test_methods() {
                    let method = method.clone();
                    #[allow(clippy::or_fun_call)]
                    let method = method
                        .split_once('#')
                        .ok_or(anyhow!("Could not parse test method - {}", method))?
                        .1;
                    commands.push(method.into());
                    test_methods.push(String::from(method));
                }
            }
            _ => {}
        };

        commands.push(format!("check {}", file.file_name()));
        commands.push(format!("doc-check {}", file.file_name()));
    }

    let mut line_editor = Reedline::create()
        .with_history(Box::new(
            FileBackedHistory::with_file(5, "history.txt".into())
                .expect("Error configuring history with file"),
        ))
        .with_highlighter(Box::new(ExampleHighlighter::new(commands.clone())))
        .with_hinter(Box::new(
            DefaultHinter::default().with_style(Style::new().italic().fg(Color::LightGray)),
        ))
        .with_completer({
            let mut inclusions = vec!['-', '_'];
            for i in '0'..='9' {
                inclusions.push(i);
            }

            let mut completer = DefaultCompleter::with_inclusions(&inclusions);
            completer.insert(commands.clone());
            Box::new(completer)
        })
        .with_quick_completions(true)
        .with_partial_completions(true)
        .with_ansi_colors(true)
        .with_menu(ReedlineMenu::EngineCompleter(Box::new(
            ColumnarMenu::default().with_name("completion_menu"),
        )))
        .with_edit_mode({
            let mut keybindings = default_emacs_keybindings();
            keybindings.add_binding(
                KeyModifiers::NONE,
                KeyCode::Tab,
                ReedlineEvent::UntilFound(vec![
                    ReedlineEvent::Menu("completion_menu".to_string()),
                    ReedlineEvent::MenuNext,
                ]),
            );

            keybindings.add_binding(
                KeyModifiers::SHIFT,
                KeyCode::BackTab,
                ReedlineEvent::MenuPrevious,
            );
            Box::new(Emacs::new(keybindings))
        });

    loop {
        let sig = line_editor.read_line(&prompt)?;
        match sig {
            Signal::Success(buffer) => match buffer.trim() {
                "exit" => break Ok(()),
                "clear" => {
                    line_editor.clear_screen()?;
                    continue;
                }
                "history" => {
                    line_editor.print_history()?;
                    continue;
                }
                b if b.starts_with("run") => {
                    let b = b.replace("run ", "");
                    if let Err(e) = project.identify(b.as_str())?.run() {
                        eprintln!("{e}");
                    }
                }
                "check-health" => {
                    if let Err(e) = project.check_health() {
                        eprintln!("{e}");
                    }
                }
                b if b.starts_with("check") => {
                    let b = b.replace("check ", "");
                    if let Err(e) = project.identify(b.as_str())?.check() {
                        eprintln!("{e}");
                    }
                }
                b if b.starts_with("doc-check") => {
                    let b = b.replace("doc-check ", "");
                    if let Err(e) = project.identify(b.as_str())?.doc_check() {
                        eprintln!("{e}");
                    }
                }
                b if test_methods.contains(&String::from(b)) => {
                    eprintln!("Try test <FILENAME> {b} instead.");
                }
                b if b.starts_with("test ") => {
                    let b = b.replace("test ", "");
                    let b = b.split_whitespace().collect::<Vec<&str>>();
                    let name = String::from(*b.first().unwrap());

                    let res = match b.len().cmp(&1) {
                        Ordering::Equal => project.identify(name.as_str())?.test(Vec::new()),
                        Ordering::Greater => {
                            let b = {
                                let mut new_b = vec![];
                                for i in b {
                                    new_b.push(String::from(i));
                                }
                                new_b
                            };

                            let b = b.iter().map(|i| i.as_str()).collect();

                            project.identify(name.as_str())?.test(b)
                        }
                        Ordering::Less => Err(anyhow!("No test file mentioned")),
                    };

                    match res {
                        Ok(out) => println!("{out}"),
                        Err(e) => eprintln!("{e:?}"),
                    };
                }
                b if b.starts_with("grade") => {
                    let b = b.replace("grade", "");
                    if let Err(e) = grade(&b) {
                        eprintln!("{e}");
                    }
                }
                "clean" => {
                    if let Err(e) = clean() {
                        eprintln!("{e}");
                    }
                }
                "info" => project.info()?,
                "update" => {
                    if let Err(e) = update() {
                        eprintln!("{e}");
                    }
                }
                "reset" => {
                    if let Err(e) = clean() {
                        break Err(e);
                    } else {
                        match Project::new() {
                            Ok(p) => project = p,
                            Err(e) => break Err(e),
                        };
                    }
                }
                _ => {
                    println!("Don't know how to {:?}", buffer.trim());
                }
            },
            Signal::CtrlD | Signal::CtrlC => {
                println!("Bye!");
                break Ok(());
            }
        }
    }
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
    /// Start a REPL
    Shell,
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

    let shell = pure(Cmd::Shell)
        .to_options()
        .command("shell")
        .help("Open a REPL");

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
        shell,
        exit
    ])
    .fallback(Cmd::Exit);

    cmd.to_options().descr("Build tool for novices").run()
}

fn main() -> Result<()> {
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
        Cmd::Shell => shell()?,
        Cmd::Exit => {}
    };

    Ok(())
}
