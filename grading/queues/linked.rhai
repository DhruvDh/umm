let project = new_project();

let req_1 = grade_docs(["DataStructures.LinkedQueue"], project, 20, "1");

let req_2 = grade_by_hidden_tests(
    "https://www.dropbox.com/s/ow7e0de6iug0bc3/LinkedQueueTest.java?raw=1",
    "LinkedQueueTest",
    80.0,
    "2"
);

show_results([req_1, req_2]);