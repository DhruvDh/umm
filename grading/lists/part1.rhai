let project = new_java_project();

let req_1 = new_docs_grader()
    .project(project)
    .files(["DataStructures.ArrayList"])
    .out_of(60.0)
    .req_name("ArrayList JavaDoc")
    .penalty(6.0)
    .run();

let req_2 = new_by_unit_test_grader()
    .project(project)
    .test_files(["DataStructures.ArrayListTest"])
    .expected_tests([])
    .out_of(60.0)
    .req_name("ArrayList Unit Tests")
    .run();

let req_3 = new_unit_test_grader()
    .req_name("ArrayListTest Mutation Testing")
    .out_of(0.0)
    .target_test(["DataStructures.ArrayListTest"])
    .target_class(["DataStructures.ArrayList"])
    .excluded_methods([])
    .avoid_calls_to([])
    .run();

let req_4 = new_by_hidden_test_grader()
    .url("https://www.dropbox.com/s/0rqscj23uveffcw/ArrayListTest.java?raw=1")
    .test_class_name("ArrayListTestHidden")
    .out_of(180.0)
    .req_name("ArrayList Hidden Tests")
    .run();


let reqs = [req_1, req_2, req_3, req_4];

show_results(reqs);

let total = 0.0;
let out_of = 0.0;
for req in reqs {
    total = total + req.grade();
    out_of = out_of + req.out_of();
}
if total > (0.7 * out_of) {
    print(total + ";p")
} else {
    print(total + ";f")
}

generate_feedback(reqs);