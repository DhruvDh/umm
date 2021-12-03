use anyhow::{bail, Context, Result};
use inquire::{error::InquireError, Select};
use umm::*;

fn main() -> Result<()> {
    let commands: Vec<&str> = vec!["grade", "check", "run", "test", "clean"];

    let ans: Result<&str, InquireError> = Select::new("What do you want to do?", commands).prompt();
    let ans = ans.context("Failed to get answer for some reason.")?;

    match ans {
        "check" => check_prompt(),
        "run" => run_prompt(),
        "test" => test_prompt(),
        "clean" => {
            clean();
            Ok(())
        }
        "grade" => {
            grade()
        }
        _ => bail!("For some reason, I don't know what you want me to do."),
    }
}
