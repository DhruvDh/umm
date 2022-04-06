use anyhow::{Result};

use umm::{java::Project};

/// Check a java file for syntax errors
#[fncmd::fncmd]
pub fn main(
    /// Path to file or Name of file to check
    #[opt()]
    name: String,
) -> Result<()> {
    let project = Project::new()?;
    project.identify(name)?.check()?;
    Ok(())
}
