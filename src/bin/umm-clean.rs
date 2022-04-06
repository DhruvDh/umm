use anyhow::{Context, Result};
use umm::{*, constants::BUILD_DIR};

/// Cleans javac artifacts produced by `umm`
#[fncmd::fncmd]
pub fn main() -> Result<()> {
    std::fs::remove_dir_all(BUILD_DIR.as_path())
        .with_context(|| format!("Could not delete {}", BUILD_DIR.display()))
}
