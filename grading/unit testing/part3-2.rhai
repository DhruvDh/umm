let project = new_project();

let req_1 = grade_by_tests(
    [("UnitTesting.StudentTest")],
    [],
    project,
    10.0,
    "1",
);

let req_2 = grade_unit_tests(
    "2",
    40.0,
    ["UnitTesting.StudentTest"],
    ["UnitTesting.Student"],
    [],
    []
);

show_results([req_1, req_2]);