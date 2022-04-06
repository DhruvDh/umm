use anyhow::Result;
use tabled::Table;
use umm::java::Project;
use umm::grade::*;

#[fncmd::fncmd]
/// Run JUnit tests from a JUnit test class (source) file
pub fn main() -> Result<()> {
    let project = Project::new()?;

    let req_1 = grade_by_tests(
        vec![String::from("DataStructures.LinkedStackTest")],
        vec![
            String::from("DataStructures.LinkedStackTest#testPop"),
            String::from("DataStructures.LinkedStackTest#testPush"),
            String::from("DataStructures.LinkedStackTest#testPeek"),
            String::from("DataStructures.LinkedStackTest#testSize"),
            String::from("DataStructures.LinkedStackTest#testToString"),
            String::from("DataStructures.LinkedStackTest#testIsEmpty"),
        ],
        &project,
        50.0,
        "1".to_string(),
    )?;

    let req_2 = grade_docs(vec!["DataStructures.LinkedStack"], &project, 20, "2".into())?;

    let req_3 = grade_unit_tests(
        "3".to_string(),
        30.0,
        vec![String::from("DataStructures.LinkedStackTest")],
        vec![String::from("DataStructures.LinkedStack")],
        vec![
            String::from("LinkedStack"),
            String::from("isEmpty"),
            String::from("size"),
            String::from("toString"),
            String::from("main"),
        ],
    )?;
    println!(
        "{}",
        Table::new(vec![req_1, req_2, req_3]).with(tabled::Style::modern())
    );
    Ok(())
}
