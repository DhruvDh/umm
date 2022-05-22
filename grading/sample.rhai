let project = JavaProject::new();

// let req_1 = grade_docs(vec!["pyramid_scheme.LinkedTree"], &project, 10, "1".into())?;

// let req_2 = grade_by_tests(
//     vec![String::from("pyramid_scheme.LinkedTreeTest")],
//     vec![
//         String::from("pyramid_scheme.LinkedTreeTest#testGetRootElement"),
//         "pyramid_scheme.LinkedTreeTest#testAddChild".into(),
//         "pyramid_scheme.LinkedTreeTest#testFindNode".into(),
//         "pyramid_scheme.LinkedTreeTest#testContains".into(),
//         "pyramid_scheme.LinkedTreeTest#testSize".into(),
//     ],
//     &project,
//     20.0,
//     "2".to_string(),
// )?;

// let req_3 = grade_unit_tests(
//     "2".into(),
//     20.0,
//     vec![String::from("pyramid_scheme.LinkedTreeTest")],
//     vec![
//         String::from("pyramid_scheme.LinkedTreeTest#testGetRootElement"),
//         "pyramid_scheme.LinkedTreeTest#testAddChild".into(),
//         "pyramid_scheme.LinkedTreeTest#testFindNode".into(),
//         "pyramid_scheme.LinkedTreeTest#testContains".into(),
//         "pyramid_scheme.LinkedTreeTest#testSize".into(),
//     ],
//     vec![],
// )?;

// let req_4 = grade_docs(
//     vec!["pyramid_scheme.PyramidScheme"],
//     &project,
//     10,
//     "3".into(),
// )?;

// let req_5 = grade_by_tests(
//     vec![String::from("pyramid_scheme.PyramidSchemeTest")],
//     vec![
//         String::from("pyramid_scheme.PyramidSchemeTest#testWhoBenefits"),
//         String::from("pyramid_scheme.PyramidSchemeTest#testAddChild"),
//         String::from("pyramid_scheme.PyramidSchemeTest#testInitiateCollapse"),
//     ],
//     &project,
//     30.0,
//     "3".into(),
// )?;