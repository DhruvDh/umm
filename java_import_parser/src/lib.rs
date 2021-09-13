use anyhow::Result;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ParseError {
    #[error("Cannot find class declaration statement in file `{0}`")]
    NoClassFound(String),
    #[error(
        "Cannot parse Package Name or Import statements for file `{0}`. ErrorMessage is:\n`{1}`"
    )]
    PegParserError(String, String),
}

#[derive(Debug)]
pub struct ParseResult {
    package_name: Option<String>,
    imports: Vec<Vec<String>>,
}

peg::parser! {
  grammar parser() for str {
    pub rule upper_alphabet() -> String
        = s:$(['A'..='Z']+) { s.to_string() }

    pub rule lower_alphabet() -> String
        = s:$(['a'..='z']+) { s.to_string() }

    pub rule number() -> u32
        = n:$(['0'..='9']+) { n.parse::<u32>().unwrap() }

    pub rule alphanumeric() -> String
        = s:$(['A'..='Z' | 'a'..='z' | '0'..='9']*) { s.to_string() }

    pub rule whitespace()
        = [' ' | '\t' | '\n' | '\r']+

    pub rule class_name() -> String
        =  a:upper_alphabet() b:alphanumeric() { a + &b }

    pub rule class_path() -> Vec<String>
        = s:(class_name() ++ ".") { s }

    pub rule import_statement() -> Vec<String>
        = "import" whitespace() s:class_path() ";" { s }

    pub rule multiple_import_statements() -> Vec<Vec<String>>
        = s:(import_statement() ** whitespace()) { s }

    pub rule package_statement() -> String
        = "package" whitespace() s:alphanumeric() ";" whitespace()? { s }

    pub rule file() -> ParseResult
        = s:(package_statement()?) i:multiple_import_statements()
        {
            ParseResult { package_name: s, imports: i }
        }
    }
}

pub fn parse(input: &str, file_name: &str) -> Result<ParseResult, ParseError> {
    let input = match input.strip_suffix("public class") {
        Some(s) => s,
        None => return Err(ParseError::NoClassFound(file_name.to_string())),
    };

    match parser::file(input) {
        Ok(r) => return Ok(r),
        Err(e) => {
            return Err(ParseError::PegParserError(
                file_name.to_string(),
                e.to_string(),
            ))
        }
    };
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn class_name() {
        assert!(parser::class_name("ClassName").is_ok());
        assert!(parser::class_name("cLassName").is_err());
    }

    #[test]
    fn class_path() {
        assert!(parser::class_path("ADTs.StackADT").is_ok());
        assert!(parser::class_path("cLassName.ADt").is_err());
    }

    #[test]
    fn import_statement() {
        assert!(parser::import_statement("import ADTs.StackADT;").is_ok());
        assert!(parser::import_statement("import cLassName.ADt;").is_err());
    }

    #[test]
    fn multiple_import_statements() {
        assert!(
            parser::multiple_import_statements("import ADTs.StackADT; import ADTs.StackADT;")
                .is_ok()
        );
        assert!(
            parser::multiple_import_statements("import ADTs.StackADT; import cLassName.ADt;")
                .is_err()
        );
    }

    #[test]
    fn package_statement() {
        assert!(parser::package_statement("package graphics;").is_ok());
    }

    #[test]
    fn file() {
        assert!(
            parser::file("package graphics; import ADTs.StackADT; import AnotherClass;").is_ok()
        );
    }
}
