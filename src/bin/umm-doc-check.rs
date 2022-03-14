use anyhow::{anyhow, Result};
use std::path::PathBuf;
use umm::*;

/// Check a java file for syntax errors
#[fncmd::fncmd]
pub fn main(
    /// Path to file
    #[opt()]
    file: String,
) -> Result<()> {
    let project = JavaProject::new()?;
    let file = PathBuf::from(file);
    let file = file
        .file_name()
        .ok_or(anyhow!(
            "Could not get file name from discovered project object",
        ))?
        .to_str()
        .ok_or(anyhow!(
            "Could not convert file name from discovered project object to str"
        ))?
        .to_string();

    let index = project
        .files
        .iter()
        .position(|x| x.file_name == file)
        .ok_or(anyhow!("Cannot find specified file in discovered project"))?;

    let name = project
        .files
        .get(index)
        .unwrap()
        .clone()
        .proper_name
        .unwrap();
    project.check(name, true)?;
    Ok(())
}
