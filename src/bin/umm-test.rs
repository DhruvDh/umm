use anyhow::{anyhow, Result};
use std::path::PathBuf;
use umm::{*, java::Project};

/// Run JUnit tests from a JUnit test class (source) file
#[fncmd::fncmd]
pub fn main(
    /// Path to file or Name of file to check
    #[opt()]
    name: String,
) -> Result<()> {
    let project = Project::new()?;
    let output = project.identify(name)?.test()?;
    println!("{}", output);
    Ok(())
}
