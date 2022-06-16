let project = new_project();

let req_1 = grade_docs(["DataStructures.ArrayList"], project, 20, "1");

let req_2 = grade_by_hidden_tests(
    "https://www.dropbox.com/s/0rqscj23uveffcw/ArrayListTest.java?raw=1",
    "ArrayListTest",
    80.0,
    "2"
);

show_results([req_1, req_2]);