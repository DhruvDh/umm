// ╭───────────────────────────────────────────────────────────╮
// │                   url for this file -                     │
// │https://www.dropbox.com/s/h1rqcejfapbfwb4/sample.rhai?raw=1│
// ╰───────────────────────────────────────────────────────────╯


let project = new_project();

let req_1 = grade_docs(["pyramid_scheme.LinkedTree"], project, 10, "1");

let req_2 = grade_by_tests(
    [("pyramid_scheme.LinkedTreeTest")],
    [
        "pyramid_scheme.LinkedTreeTest#testGetRootElement",
        "pyramid_scheme.LinkedTreeTest#testAddChild",
        "pyramid_scheme.LinkedTreeTest#testFindNode",
        "pyramid_scheme.LinkedTreeTest#testContains",
        "pyramid_scheme.LinkedTreeTest#testSize",
    ],
    project,
    20.0,
    "2",
);

let req_3 = grade_unit_tests(
    "2",
    20.0,
    ["pyramid_scheme.LinkedTreeTest"],
    [
        "pyramid_scheme.LinkedTreeTest#testGetRootElement",
        "pyramid_scheme.LinkedTreeTest#testAddChild",
        "pyramid_scheme.LinkedTreeTest#testFindNode",
        "pyramid_scheme.LinkedTreeTest#testContains",
        "pyramid_scheme.LinkedTreeTest#testSize",
    ],
    [],
    []
);

let req_4 = grade_docs(
    ["pyramid_scheme.PyramidScheme"],
    project,
    10,
    "3",
);

let req_5 = grade_by_tests(
    ["pyramid_scheme.PyramidSchemeTest"],
    [
        "pyramid_scheme.PyramidSchemeTest#testWhoBenefits",
        "pyramid_scheme.PyramidSchemeTest#testAddChild",
        "pyramid_scheme.PyramidSchemeTest#testInitiateCollapse",
    ],
    project,
    30.0,
    "3",
);

let req_6 = grade_by_hidden_tests(
    "https://www.dropbox.com/s/47jd1jru1f1i0cc/ABCTest.java?raw=1",
    "ABCTest",
    30.0,
    "4"
);

show_results([req_1, req_2, req_3, req_4, req_5, req_6]);


// ╭──────────────────────────────────────────────────────────╮
// │                  SAMPLE SCRIPT OUTPUT                    │
// ╰──────────────────────────────────────────────────────────╯
// ┌─────────────────────────────────────────────────────┬
// │     Check javadoc for pyramid_scheme.LinkedTree     │
// ├─────────────────────────────────────────────────────┼
// │      File       │ Line │          Message           │
// ├─────────────────┼──────┼────────────────────────────┤
// │ LinkedTree.java │  14  │   no initial description   │
// ├─────────────────┼──────┼────────────────────────────┤
// │ LinkedTree.java │  15  │ no description for @param  │
// ├─────────────────┼──────┼────────────────────────────┤
// │ LinkedTree.java │  29  │ no description for @param  │
// ├─────────────────┼──────┼────────────────────────────┤
// │ LinkedTree.java │  56  │   Error: unknown tag: T    │
// ├─────────────────┼──────┼────────────────────────────┤
// │ LinkedTree.java │  72  │ no description for @throws │
// ├─────────────────┼──────┼────────────────────────────┤
// │ LinkedTree.java │ 251  │ no description for @param  │
// ├─────────────────┼──────┼────────────────────────────┤
// │                  -18 due to 6 nits                  │
// └─────────────────────────────────────────────────────┴

// Running Mutation tests -
// 5:43:44 PM PIT >> INFO : Verbose logging is disabled. If you encounter a problem, please enable it before reporting an issue.
// 5:43:44 PM PIT >> INFO : Incremental analysis reduced number of mutations by 0
// 5:43:44 PM PIT >> INFO : Created  0 mutation test units in pre scan
// Exception in thread "main" org.pitest.help.PitHelpError: No mutations found. This probably means there is an issue with either the supplied classpath or filters.
// See http://pitest.org for more details.
// 	at org.pitest.mutationtest.tooling.MutationCoverage.checkMutationsFound(MutationCoverage.java:352)
// 	at org.pitest.mutationtest.tooling.MutationCoverage.runReport(MutationCoverage.java:132)
// 	at org.pitest.mutationtest.tooling.EntryPoint.execute(EntryPoint.java:123)
// 	at org.pitest.mutationtest.tooling.EntryPoint.execute(EntryPoint.java:54)
// 	at org.pitest.mutationtest.commandline.MutationCoverageReport.runReport(MutationCoverageReport.java:98)
// 	at org.pitest.mutationtest.commandline.MutationCoverageReport.main(MutationCoverageReport.java:45)

// ┌────────────────────────────────────────────────────────┬
// │     Check javadoc for pyramid_scheme.PyramidScheme     │
// ├────────────────────────────────────────────────────────┼
// │        File        │ Line │          Message           │
// ├────────────────────┼──────┼────────────────────────────┤
// │ PyramidScheme.java │  10  │ Error: unknown tag: Person │
// ├────────────────────┼──────┼────────────────────────────┤
// │ PyramidScheme.java │  18  │         no comment         │
// ├────────────────────┼──────┼────────────────────────────┤
// │ PyramidScheme.java │  19  │         no comment         │
// ├────────────────────┼──────┼────────────────────────────┤
// │ PyramidScheme.java │ 165  │ no description for @throws │
// ├────────────────────┼──────┼────────────────────────────┤
// │ PyramidScheme.java │ 241  │ no description for @return │
// ├────────────────────┼──────┼────────────────────────────┤
// │                   -15 due to 5 nits                    │
// └────────────────────────────────────────────────────────┴

// ┌─────────────────────────────────────────────────────────────────┬
// │                        Grading Overview                         │
// ├─────────────────────────────────────────────────────────────────┼
// │ Requirement │   Grade    │                Reason                │
// ├─────────────┼────────────┼──────────────────────────────────────┤
// │      1      │    0/10    │              See above.              │
// ├─────────────┼────────────┼──────────────────────────────────────┤
// │      2      │ 0.00/20.00 │         - 0/5 tests passing.         │
// ├─────────────┼────────────┼──────────────────────────────────────┤
// │      2      │    0/20    │ Something went wrong while running m │
// │             │            │       utation tests, skipping.       │
// ├─────────────┼────────────┼──────────────────────────────────────┤
// │      3      │    0/10    │              See above.              │
// ├─────────────┼────────────┼──────────────────────────────────────┤
// │      3      │ 0.00/30.00 │         - 0/3 tests passing.         │
// ├─────────────┼────────────┼──────────────────────────────────────┤
// │      4      │ 0.00/30.00 │         - 0/5 tests passing.         │
// ├─────────────┼────────────┼──────────────────────────────────────┤
// │                       Total: 0.00/120.00                        │
// └─────────────────────────────────────────────────────────────────┴
