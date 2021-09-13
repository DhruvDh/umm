use anyhow::{bail, Result};

#[derive(Debug)]
pub struct ParseResult {
    pub package_name: Option<String>,
    pub imports: Option<Vec<Vec<String>>>,
    pub class_name: String,
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

    pub rule __whitespace()
        = [' ' | '\t' | '\n' | '\r']* 
    
    pub rule _whitespace()
        = __whitespace() __whitespace()

    pub rule class_name() -> String
        =  a:upper_alphabet() b:alphanumeric() { a + &b }

    pub rule class_path() -> Vec<String>
        = s:(alphanumeric() ++ ".") { s }

    pub rule import_statement() -> Vec<String>
        = "import" _whitespace() "static"? _whitespace() s:class_path() ";" { s }

    pub rule multiple_import_statements() -> Vec<Vec<String>>
        = s:(import_statement() ** _whitespace()) _whitespace() { s }

    pub rule package_statement() -> String
        = "package" _whitespace() s:alphanumeric() ";" _whitespace() { s }

    pub rule class_declaration() -> String
        = "public"? _whitespace() "class" _whitespace() c:class_name() _whitespace() { c }
    
    #[no_eof]
    pub rule file() -> ParseResult
        = s:(package_statement()?) i:multiple_import_statements()? c:class_declaration() 
        {
            ParseResult { package_name: s, imports: i, class_name: c }
        }
    }
}

pub fn parse(input: &str, file_name: &str) -> Result<ParseResult> {
    match parser::file(input) {
        Ok(r) => return Ok(r),
        Err(e) => {
            bail!("Cannot parse Package Name, Import statements, or Class declaration for file {}. ErrorMessage is:\n{}",
                file_name.to_string(),
                e.to_string()
            )
        }
    }
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
        let inputs = vec![
            "public class ABC {}",
            "class ABC {}",
            "package graphics; import ADTs.StackADT; import AnotherClass; class ABC {}",
            "package graphics; import ADTs.StackADT; import AnotherClass; public class ABC {}",
            r#"public class Main {
                public static void main(String[] args) {
                    System.out.println("Hello World!");
                }
            }"#
        ];

        for input in inputs {
            assert!(
                dbg!(parser::file(input)).is_ok()
            );
        }
    }
}