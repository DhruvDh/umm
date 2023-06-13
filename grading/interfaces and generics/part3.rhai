let project = new_java_project();

let req_1 = new_docs_grader()
    .project(project)
    .files(["UnitTesting.CoOrds"])
    .out_of(5.0)
    .req_name("CoOrds JavaDoc")
    .penalty(1.0)
    .run();

    let req_3 = new_unit_test_grader()
    .req_name("CoOrds Mutation Tests")
    .out_of(20.0)
    .target_test(["UnitTesting.CoOrdsTest"])
    .target_class(["UnitTesting.CoOrds"])
    .run();

let req_2 = new_docs_grader()
    .project(project)
    .files(["UnitTesting.Student"])
    .out_of(5.0)
    .req_name("Student JavaDoc")
    .penalty(1.0)
    .run();

let req_4 = new_unit_test_grader()
    .req_name("Student Mutation Tests")
    .out_of(20.0)
    .target_test(["UnitTesting.StudentTest"])
    .target_class(["UnitTesting.Student"])
    .run();

let reqs = [req_1, req_2, req_3, req_4];

show_results(reqs);
generate_feedback(reqs);

if (
    project
        .identify("UnitTesting.StudentTest")
        .test_methods
        .len() == 0
) {
    print("np");
} else if req_2.grade() >= 20 {
    print("p;" + (req_1.grade() + 20).to_int());
} else {
    print("np");
}