#![warn(missing_docs)]
#![warn(clippy::missing_docs_in_private_items)]

use std::path::PathBuf;

use lazy_static::lazy_static;
use postgrest::Postgrest;
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
    pub static ref SUPABASE_KEY: String = "eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.eyJpc3MiOiJzdXBhYmFzZSIsInJlZiI6InV5YW5jenRtempsZWtvamVwcm9qIiwicm9sZSI6ImFub24iLCJpYXQiOjE2NjA4NDA1NzgsImV4cCI6MTk3NjQxNjU3OH0.yMvOYM0AM61v6MRsHUSgO0BPrQHTde2AiKzE0b4H4lo".to_string();
    /// PostGrest client
    pub static ref POSTGREST_CLIENT: Postgrest = Postgrest::new("https://uyancztmzjlekojeproj.supabase.co/rest/v1")
            .insert_header("apiKey", SUPABASE_KEY.clone());
    /// Runtime
    pub static ref RUNTIME_HANDLE: tokio::runtime::Runtime = tokio::runtime::Runtime::new().unwrap();

}

/// Current term. TODO: Move this to init script
pub const TERM: &str = "Fall 2022";

/// Current course. TODO: Move this to init script
pub const COURSE: &str = "ITSC 2214";

/// file name for JUnit platform console standard jar
pub const JUNIT_PLATFORM: &str = "junit-platform-console-standalone-1.9.0-RC1.jar";

/// Tree-sitter query that returns imports made
/// * `path`: java name of the import as it appears in the source code.
/// * `asterisk`: true if the import path ends in an asterisk
pub const IMPORT_QUERY: &str = r#"
(import_declaration 
    (
        [	
        	(scoped_identifier) @path           	
        	(identifier) @path
        ]
        (asterisk)? @asterisk
    )
)
"#;

/// Tree-sitter query that returns name of the package
/// * `name`: name of the package
pub const PACKAGE_QUERY: &str = r#"
(package_declaration 
    (identifier) @name
)
"#;

/// Tree-sitter query that returns name of the class
/// * `name`: name of the class
pub const CLASSNAME_QUERY: &str = r#"
(
    class_declaration
    name: (identifier) @name
)
"#;

/// Tree-sitter query that returns name of the interface
/// * `name`: name of the interface
pub const INTERFACENAME_QUERY: &str = r#"
(
    interface_declaration
    name: (identifier) @name
)
"#;

/// Tree-sitter query that returns name of the JUnit `@Test` annotated methods
/// * `name`: name of the test method
pub const TEST_ANNOTATION_QUERY: &str = r#"
(method_declaration
	(modifiers
        (annotation
            name: (identifier) @annotation
            arguments: (annotation_argument_list)
        )
    )
    name: (identifier) @name
)

(method_declaration
	(modifiers
	(marker_annotation
    	name: (identifier) @annotation)
    )
    name: (identifier) @name
    (#eq? @annotation "Test")
)
"#;

/// Tree-sitter query to check the existence of a main method.
pub const MAIN_METHOD_QUERY: &str = r#"
(method_declaration
	(modifiers) @modifier
    type: (void_type) @return_type
    name: (identifier) @name
    parameters: (formal_parameters
      (formal_parameter
          type: (array_type
          	element: (type_identifier) @para_type
            dimensions: (dimensions) @dim
          )
          name: (identifier) @para_name
      )
    )
    (#eq? @name "main")
    (#eq? @return_type "void")
    (#eq? @para_type "String")
    (#eq? @dim "[]")
)
"#;
