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

show_results([req_1, req_2, req_3, req_4, req_5]);