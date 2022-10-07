let project = new_project();

let req_1 = grade_docs(["DataStructures.LinkedStack"], project, 20, "1");

let req_2 = grade_by_hidden_tests(
    "https://ummfiles.fra1.digitaloceanspaces.com/grading_scripts/stacks/LinkedStackTest.java",
    "LinkedStackTest",
    80.0,
    "2"
);

show_results([req_1, req_2]);