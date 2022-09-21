// TODO: check if java files are in the right place according to the package
// TODO: make recommendations for the above

// TODO: the following
// if BUILD_DIR.join(".vscode").exists() {
//     std::fs::remove_dir_all(BUILD_DIR.join(".vscode").as_path())
//         .with_context(|| format!("Could not delete {}",
// BUILD_DIR.join(".vscode").display()))?; }

// if BUILD_DIR.join(LIB_DIR.display().to_string()).exists() {
//     std::fs::remove_dir_all(BUILD_DIR.join(LIB_DIR.display().to_string()).
// as_path())         .with_context(|| {
//             format!(
//                 "Could not delete {}",
//                 BUILD_DIR.join(LIB_DIR.display().to_string()).display()
//             )
//         })?;
// }

use std::ops::Deref;

use anyhow::Result;
use futures::{
    future::try_join_all,
    stream::FuturesUnordered,
};
use tokio::fs::OpenOptions;
use walkdir::WalkDir;

use crate::{
    clean,
    constants::{
        ROOT_DIR,
        RUNTIME,
    },
    java::{
        FileType,
        Project,
    },
};

impl Project {
    /// Checks the project for common CodingRooms errors
    pub fn check_health(&self) -> Result<()> {
        tracing::info!("Resetting project metadata and libraries");
        clean()?;
        let project = Project::new()?;
        tracing::info!("Done.");

        let rt = RUNTIME.handle().clone();
        let _guard = rt.enter();

        let handle1 = rt.spawn(async {
            let files = WalkDir::new(ROOT_DIR.as_path())
                .into_iter()
                .filter_map(|e| e.ok())
                .filter(|e| e.file_type().is_file())
                .map(|e| e.path().to_path_buf())
                .map(|path| {
                    tokio::spawn(async move {
                        match tokio::fs::metadata(path.clone()).await {
                            Ok(m) => {
                                if m.len() == 0 {
                                    tracing::warn!("File {} is empty", &path.display())
                                }
                                if let Err(e) =
                                    OpenOptions::new().read(true).write(true).open(&path).await
                                {
                                    tracing::warn!(
                                        "File {} could not be opened (read + write): {}",
                                        &path.display(),
                                        e
                                    )
                                }
                            }
                            Err(e) => {
                                tracing::warn!("Could not read file {}: {}", path.display(), e)
                            }
                        };

                        if path.extension().unwrap_or_default() == "jar" {
                            let output = tokio::process::Command::new("zip")
                                .arg("-T")
                                .arg(&path)
                                .output()
                                .await
                                .unwrap_or_else(|_| {
                                    panic!("Could not run zip -T on {}", &path.display())
                                });

                            if !output.status.success() {
                                tracing::warn!(
                                    "File {} is not a valid zip file: {}",
                                    &path.display(),
                                    String::from_utf8_lossy(&output.stderr)
                                )
                            }
                        }
                    })
                })
                .collect::<FuturesUnordered<_>>();

            try_join_all(files).await
        });

        let handle2 = rt.spawn(async move {
            let files = project
                .files()
                .into_iter()
                .map(|file| {
                    let file = file.clone();
                    tokio::spawn(async move {
                        match file.kind() {
                            FileType::Test => {
                                if &file
                                    .path()
                                    .parent()
                                    .unwrap_or(&ROOT_DIR)
                                    .to_string_lossy()
                                    .to_string()
                                    == file.package_name().unwrap_or(&".".to_string() {
                                      
                                    })
                            }
                        };
                    })
                })
                .collect::<Vec<_>>();
        });

        tracing::info!("If there are no warnings above, your project is healthy!");
        Ok(())
    }
}
