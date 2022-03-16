use anyhow::{anyhow, Result};
use std::path::PathBuf;
use umm::*;

/// Check a java file for syntax errors
#[fncmd::fncmd]
pub fn main(
    /// Path to file or Name of file to check
    #[opt()]
    name: String,
) -> Result<()> {
    let project = JavaProject::new()?;
    project.identify(name)?.check()?;
    Ok(())
}
