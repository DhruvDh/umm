use lazy_static::lazy_static;
use tree_sitter;

pub mod constants {
    lazy_static! {
        /// Path to project root
        pub static ref ROOT_DIR: PathBuf = PathBuf::from(".");
        /// Directory for source files
        pub static ref SOURCE_DIR: PathBuf = PathBuf::from(".").join("src");
        /// Directory to store compiler artifacts
        pub static ref BUILD_DIR: PathBuf = PathBuf::from(".").join("target");
        /// Directory for test files
        pub static ref TEST_DIR: PathBuf = PathBuf::from(".").join("test");
        /// Directory for libraries, jars
        pub static ref LIB_DIR: PathBuf = PathBuf::from(".").join("lib");
        /// Directory for `umm` artifacts
        pub static ref UMM_DIR: PathBuf = PathBuf::from(".").join(".umm");
        /// Platform specific separator charactor for javac paths
        pub static ref SEPARATOR: &'static str = if cfg!(windows) { ";" } else { ":" };
        /// Reference to treesitter language struct
        pub static ref JAVA_TS_LANG: tree_sitter::Language = tree_sitter_java::language();
    }
}
