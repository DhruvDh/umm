let project = new_project();

let req_1 = grade_docs(["DataStructures.LinkedStack"], project, 20, "1");

let req_2 = grade_by_hidden_tests(
    "https://www.dropbox.com/s/h43fo5sjchy0ik2/LinkedStackTest.java?raw=1",
    "LinkedStackTest",
    80.0,
    "2"
);

show_results([req_1, req_2]);