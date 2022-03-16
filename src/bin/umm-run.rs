use anyhow::{anyhow, Result};
use std::path::PathBuf;
use umm::*;

/// Run a java file
#[fncmd::fncmd]
pub fn main(
    /// Path to file or Name of file to check
    #[opt()]
    name: String,
) -> Result<()> {
    let project = JavaProject::new()?;
    project.identify(name)?.run()?;
    Ok(())
}
