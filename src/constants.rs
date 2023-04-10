#![warn(missing_docs)]
#![warn(clippy::missing_docs_in_private_items)]

use std::{
    path::PathBuf,
    sync::{
        Arc,
        Mutex,
    },
};

use lazy_static::lazy_static;
use postgrest::Postgrest;
use rhai::AST;
use tree_sitter;

// TODO: replace with https://lib.rs/crates/state
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
    /// Platform specific separator character for javac paths
    pub static ref SEPARATOR: &'static str = if cfg!(windows) { ";" } else { ":" };
    /// Reference to treesitter language struct
    pub static ref JAVA_TS_LANG: tree_sitter::Language = tree_sitter_java::language();
    /// Supabase public api key
    pub static ref SUPABASE_KEY: String = String::from("eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.eyJpc3MiOiJzdXBhYmFzZSIsInJlZiI6InV5YW5jenRtempsZWtvamVwcm9qIiwicm9sZSI6ImFub24iLCJpYXQiOjE2NjA4NDA1NzgsImV4cCI6MTk3NjQxNjU3OH0.yMvOYM0AM61v6MRsHUSgO0BPrQHTde2AiKzE0b4H4lo");
    /// PostGrest client
    pub static ref POSTGREST_CLIENT: Postgrest = Postgrest::new("https://uyancztmzjlekojeproj.supabase.co/rest/v1")
            .insert_header("apiKey", SUPABASE_KEY.clone());
    /// Runtime
    pub static ref RUNTIME: tokio::runtime::Runtime = tokio::runtime::Runtime::new().unwrap();
    /// ChatGPT System Message intro
    pub static ref SYSTEM_MESSAGE_INTRO: String = include_str!("prompts/system_message_intro.md").into();
    /// ChatGPT System Message outro
    pub static ref SYSTEM_MESSAGE_OUTRO: String = include_str!("prompts/system_message_outro.md").into();
    /// Entire ChatGPT System Message
    pub static ref SYSTEM_MESSAGE: String = format!("{}\n{}", *SYSTEM_MESSAGE_INTRO, *SYSTEM_MESSAGE_OUTRO);
    /// Rhai script as a AST, behind an mutex.
    pub static ref SCRIPT_AST: Arc<Mutex<AST>> = Arc::new(Mutex::new(AST::empty()));
}

/// Current term. TODO: Move this to init script
pub const TERM: &str = "Fall 2022";

/// Current course. TODO: Move this to init script
pub const COURSE: &str = "ITSC 2214";

/// Prompt truncation length
pub const PROMPT_TRUNCATE: usize = 6000;

/// file name for JUnit platform console standard jar
pub const JUNIT_PLATFORM: &str = "junit-platform-console-standalone-1.9.0-RC1.jar";

/// Tree-sitter query that returns imports made
/// * `path`: java name of the import as it appears in the source code.
/// * `asterisk`: true if the import path ends in an asterisk
pub const IMPORT_QUERY: &str = include_str!("queries/import.scm");

/// Tree-sitter query that returns name of the package
/// * `name`: name of the package
pub const PACKAGE_QUERY: &str = include_str!("queries/package.scm");

/// Tree-sitter query that returns name of the class
/// * `name`: name of the class
pub const CLASSNAME_QUERY: &str = include_str!("queries/class_name.scm");

/// Tree-sitter query that returns name of the interface
/// * `name`: name of the interface
pub const INTERFACENAME_QUERY: &str = include_str!("queries/interface_name.scm");

/// Tree-sitter query that returns name of the JUnit `@Test` annotated methods
/// * `name`: name of the test method
pub const TEST_ANNOTATION_QUERY: &str = include_str!("queries/test_annotation.scm");

/// Tree-sitter query to check the existence of a main method.
pub const MAIN_METHOD_QUERY: &str = include_str!("queries/main_method.scm");

/// Tree-sitter query that returns class declaration statements
/// * `className`: class name
/// * `typeParameters`: type parameters
/// * `interfaces`: interfaces
pub const CLASS_DECLARATION_QUERY: &str = include_str!("queries/class_declaration.scm");

/// * `field`: entire field declaration
pub const CLASS_FIELDS_QUERY: &str = include_str!("queries/class_fields.scm");

/// Tree-sitter query that returns class constructor signatures
/// * `modifier`: constructor modifiers
/// * `annotation`: constructor annotations
/// * `identifier`: constructor identifier
/// * `parameters`: constructor parameters
/// * `throws`: constructor throws
pub const CLASS_CONSTRUCTOR_QUERY: &str = include_str!("queries/class_constructors.scm");

/// Tree-sitter query that returns class method signatures
/// * `modifier`: method modifiers
/// * `annotation`: method annotations
/// * `returnType`: method return type
/// * `identifier`: method identifier
/// * `parameters`: method parameters
/// * `throws`: method throws
pub const CLASS_METHOD_QUERY: &str = include_str!("queries/class_methods.scm");

/// Tree-sitter query that returns interface declaration statements
/// * `identifier`: interface name
/// * `parameters`: type parameters
/// * `extends`: extends interfaces
pub const INTERFACE_DECLARATION_QUERY: &str = include_str!("queries/interface_declaration.scm");

/// Tree-sitter query that returns interface constants
/// * `constant`: entire constant declaration
pub const INTERFACE_CONSTANTS_QUERY: &str = include_str!("queries/interface_constants.scm");

/// Tree-sitter query that returns interface methods signatures
/// * `signature`: entire method signature
pub const INTERFACE_METHODS_QUERY: &str = include_str!("queries/interface_methods.scm");

/// Tree-sitter query that returns method call identifiers
/// * `name`: method call identifier
pub const METHOD_CALL_QUERY: &str = include_str!("queries/method_invocation.scm");
