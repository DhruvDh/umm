// https://www.dropbox.com/s/svav5ikceqge6ef/part2.rhai?raw=1

let project = new_project();

let req_1 = grade_by_tests(
    [("Calculator.CalculatorTest")],
    [
        "Calculator.CalculatorTest#testAdd",
        "Calculator.CalculatorTest#testSubtract",
        "Calculator.CalculatorTest#testMultiply",
        "Calculator.CalculatorTest#testDivide",
    ],
    project,
    10.0,
    "1",
);

let req_2 = grade_unit_tests(
    "2",
    40.0,
    ["Calculator.CalculatorTest"],
    ["Calculator.Calculator"],
    [],
    []
);

show_results([req_1, req_2]);