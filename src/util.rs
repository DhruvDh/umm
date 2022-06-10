#![warn(missing_docs)]
#![warn(clippy::missing_docs_in_private_items)]

use std::{
    ffi::OsString,
    fs::File,
    io::{
        Read,
        Write,
    },
    path::{
        Path,
        PathBuf,
    },
};

use anyhow::{
    Context,
    Result,
};
use glob::glob;
use which::which;

use crate::constants::*;

/// Finds an returns the path to javac binary
pub fn javac_path() -> Result<OsString> {
    which("javac")
        .map(PathBuf::into_os_string)
        .context("Cannot find a Java Compiler on path (javac)")
}

/// Finds an returns the path to java binary
pub fn java_path() -> Result<OsString> {
    which("java")
        .map(PathBuf::into_os_string)
        .context("Cannot find a Java runtime on path (java)")
}

/// A glob utility function to find paths to files with certain extension
///
/// * `extension`: the file extension to find paths for
/// * `search_depth`: how many folders deep to search for
/// * `root_dir`: the root directory where search starts
pub fn find_files(extension: &str, search_depth: i8, root_dir: &Path) -> Result<Vec<PathBuf>> {
    let mut root_dir = PathBuf::from(root_dir);

    for _ in 0..search_depth {
        root_dir.push("**");
    }

    root_dir.push(format!("*.{}", extension));
    let root_dir = root_dir
        .to_str()
        .context("Could not convert root_dir to string")?;

    Ok(glob(root_dir)
        .context("Could not create glob")?
        .filter_map(Result::ok)
        .collect())
}

/// Find class, jar files in library path and build directory to populate
/// classpath and return it
pub fn classpath() -> Result<String> {
    let mut path: Vec<String> = vec![
        BUILD_DIR.display().to_string(),
        LIB_DIR.display().to_string(),
        // SOURCE_DIR.display().to_string(),
        // ROOT_DIR.display().to_string(),
    ];

    path.append(
        &mut find_files("jar", 4, &ROOT_DIR)?
            .iter()
            .map(|p| p.as_path().display().to_string())
            .collect(),
    );
    // path.append(
    //     &mut find_files("java", 4, &ROOT_DIR)?
    //         .iter()
    //         .map(|p| p.as_path().display().to_string())
    //         .collect(),
    // );

    Ok(path.join(&SEPARATOR))
}

/// Find java files in source path and root directory to populate
/// sourcepath and return it
pub fn sourcepath() -> Result<String> {
    let mut path: Vec<String> = vec![
        // BUILD_DIR.display().to_string(),
        // LIB_DIR.display().to_string(),
        SOURCE_DIR.display().to_string(),
        ROOT_DIR.display().to_string(),
    ];

    path.append(
        &mut find_files("java", 4, &ROOT_DIR)?
            .iter()
            .map(|p| p.as_path().display().to_string())
            .collect(),
    );

    Ok(path.join(&SEPARATOR))
}

/// TODO: Add docs
pub fn download(url: &str, path: &PathBuf, replace: bool) -> Result<()> {
    if !replace && path.exists() {
        Ok(())
    } else {
        let resp = ureq::get(url)
            .call()
            .context(format!("Failed to download {}", url))?;

        let len = resp
            .header("Content-Length")
            .and_then(|s| s.parse::<usize>().ok())
            .unwrap();

        let mut bytes: Vec<u8> = Vec::with_capacity(len);

        resp.into_reader()
            .take(10_000_000)
            .read_to_end(&mut bytes)
            .context(format!(
                "Failed to read response till the end while downloading file at {}",
                url,
            ))?;

        let name = path.file_name().unwrap().to_str().unwrap();

        let mut file = File::create(path).context(format!("Failed to create file at {}", name))?;

        file.write_all(&bytes)
            .context(format!("Failed to write to file at {}", name))
    }
}
