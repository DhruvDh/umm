use glob::glob;
use which::which;
pub mod constants;

pub mod util {
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
    fn find_files(extension: &str, search_depth: i8, root_dir: &Path) -> Result<Vec<PathBuf>> {
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

    /// Find class, jar files in library path and build directory to populate classpath and return it
    pub fn classpath() -> Result<String> {
        let mut path: Vec<String> = vec![
            BUILD_DIR.display().to_string(),
            LIB_DIR.display().to_string(),
            SOURCE_DIR.display().to_string(),
            ROOT_DIR.display().to_string(),
        ];

        path.append(
            &mut find_files("jar", 4, &LIB_DIR)?
                .iter()
                .map(|p| p.as_path().display().to_string())
                .collect(),
        );
        // path.append(
        //     &mut find_files("java", 4, &SOURCE_DIR)?
        //         .iter()
        //         .map(|p| p.as_path().display().to_string())
        //         .collect(),
        // );

        Ok(path.join(&SEPARATOR))
    }
}
