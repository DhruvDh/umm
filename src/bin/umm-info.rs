use anyhow::{Context, Result};
use std::{fs::File, io::Write};
use umm::{*, constants::UMM_DIR, java::Project};

/// Writes project info to `UMM_DIR`
#[fncmd::fncmd]
pub fn main() -> Result<()> {
    let project = Project::new()?;

    let json = serde_json::to_string(&project)?;
    std::fs::create_dir_all(UMM_DIR.as_path())
        .with_context(|| "Could not create $UMM_DIR folder")?;
    let mut output = File::create(UMM_DIR.join("info.json"))
        .with_context(|| "Could not create $UMM_DIR/info.json")?;
    write!(output, "{}", json).with_context(|| "Could not write to info.json")?;
    Ok(())
}
