let project = new_project();

let req_1 = grade_by_tests(
    [("DataStructures.WorkAheadQueueTest")],
    [
        "DataStructures.WorkAheadQueueTest#testEnqueue",
        "DataStructures.WorkAheadQueueTest#testIsEmpty",
        "DataStructures.WorkAheadQueueTest#testSize",
        "DataStructures.WorkAheadQueueTest#testFirst_int",
        "DataStructures.WorkAheadQueueTest#testFirst_0args",
        "DataStructures.WorkAheadQueueTest#testFirstThreeNodes",
        "DataStructures.WorkAheadQueueTest#testFirstThreeElements",
        "DataStructures.WorkAheadQueueTest#testDequeue_0args",
        "DataStructures.WorkAheadQueueTest#testDequeue_int",
        "DataStructures.WorkAheadQueueTest#testDequeue_int2",
        "DataStructures.WorkAheadQueueTest#testDequeue_int_ECE",
        "DataStructures.WorkAheadQueueTest#testDequeue_int_IAE",

    ],
    project,
    30.0,
    "Student-written Tests",
);

let req_2 = grade_by_hidden_tests(
    "https://ummfiles.fra1.digitaloceanspaces.com/grading_scripts/projects/workaheadqueue/WorkAheadQueueHiddenTest.java",
    "WorkAheadQueueHiddenTest",
    50.0,
    "Hidden Tests",
);

show_results([req_1, req_2]);