let project = new_project();

let req_1 = grade_docs(["DataStructures.ArrayQueue"], project, 20, "1");

let req_2 = grade_by_hidden_tests(
    "https://www.dropbox.com/s/3204wyshtpt03ex/ArrayQueueTest.java?raw=1",
    "ArrayQueueTest",
    80.0,
    "2"
);

show_results([req_1, req_2]);