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
    Context,
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
use umm::{
    clean,
    constants::{
        BUILD_DIR,
        LIB_DIR,
    },
    grade,
    java::{
        self,
        FileType,
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
        "grade".into(),
        "clean".into(),
        "info".into(),
        "clear".into(),
        "exit".into(),
        "history".into(),
    ];

    let project = java::Project::new()?;

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
                    let res = project.identify(b.as_str())?.run();
                    if res.is_err() {
                        eprintln!("{:?}", res);
                    }
                }
                b if b.starts_with("check") => {
                    let b = b.replace("check ", "");
                    let res = project.identify(b.as_str())?.check();
                    if res.is_err() {
                        eprintln!("{:?}", res);
                    }
                }
                b if b.starts_with("doc-check") => {
                    let b = b.replace("doc-check ", "");
                    let res = project.identify(b.as_str())?.doc_check();
                    if res.is_err() {
                        eprintln!("{:?}", res);
                    }
                }
                b if test_methods.contains(&String::from(b)) => {
                    eprintln!("Try test <FILENAME> {} instead.", b);
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
                        Ok(out) => println!("{}", out),
                        Err(e) => eprintln!("{:?}", e),
                    };
                }
                b if b.starts_with("grade") => {
                    let b = b.replace("grade", "");
                    let res = grade(&b);
                    if res.is_err() {
                        eprintln!("{:?}", res);
                    }
                }
                "clean" => {
                    let res = clean();
                    if res.is_err() {
                        eprintln!("{:?}", res);
                    }
                }
                "info" => project.info()?,
                "update" => {
                    match update() {
                        Ok(_) => {}
                        Err(e) => eprintln!("{}", e),
                    };
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

fn main() -> Result<()> {
    let t: Parser<Option<String>> = positional("TESTNAME").optional();
    let f: Parser<Option<String>> = positional("FILENAME").optional();
    let cmd: Parser<Option<String>> = positional("COMMAND").optional();
    let combined_parser = construct!(cmd, f, t);

    let (cmd, f, t) = Info::default()
        .descr("Build tool for novices")
        .for_parser(combined_parser)
        .run();

    let c = cmd.clone().unwrap_or_default();
    if c.as_str() == "clean" {
        clean()?;
        return Ok(());
    }

    let project = java::Project::new()?;
    let f = f.unwrap_or_default();
    let t = t.unwrap_or_default();

    match cmd {
        // TODO: move this to a separate method and call that method in shell()
        Some(a) => match a.as_str() {
            "run" => project.identify(f.as_str())?.run()?,
            "check" => project.identify(f.as_str())?.check()?,
            "test" => {
                let out = if t.is_empty() {
                    project.identify(f.as_str())?.test(vec![])?
                } else {
                    project.identify(f.as_str())?.test(vec![&t])?
                };
                println!("{}", out);
            }
            "doc-check" => {
                let out = project.identify(f.as_str())?.doc_check()?;
                println!("{}", out);
            }
            "grade" => grade(&f)?,
            "info" => project.info()?,
            "update" => {
                match update() {
                    Ok(_) => {}
                    Err(e) => eprintln!("{}", e),
                };
            }
            _ => eprintln!("{} is not a valid subcommand.", a),
        },
        None => shell()?,
    };

    if BUILD_DIR.join(".vscode").exists() {
        std::fs::remove_dir_all(BUILD_DIR.join(".vscode").as_path())
            .with_context(|| format!("Could not delete {}", BUILD_DIR.join(".vscode").display()))?;
    }

    if BUILD_DIR.join(LIB_DIR.display().to_string()).exists() {
        std::fs::remove_dir_all(BUILD_DIR.join(LIB_DIR.display().to_string()).as_path())
            .with_context(|| {
                format!(
                    "Could not delete {}",
                    BUILD_DIR.join(LIB_DIR.display().to_string()).display()
                )
            })?;
    }

    Ok(())
}
