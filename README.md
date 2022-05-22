# umm

## Introduction

A java build tool for novices.

## Installation

You would need rust installed, ideally the nightly toolchain. You can visit https://rustup.rs/ to find out how to install this on your computer, just make sure you install the "nightly" toolchain instead of stable.

On Linux, Windows Subsystem for Linux (WSL), and Mac you should be able to run `curl https://sh.rustup.rs -sSf | sh -s -- --default-toolchain nightly` on a terminal to install the nightly toolchain for rust.

Once you are done, just type `cargo install --git=https://github.com/DhruvDh/umm.git` and it should compile and install it on your system.

## Auto-grading

Also allows for running auto-grading scripts based on [Rhai](https://rhai.rs/book/about/index.html).

### Sample grading script

```rust
// url for this file -
// https://www.dropbox.com/s/h1rqcejfapbfwb4/sample.rhai?raw=1
// to run this script install binary and -
// `umm grade "https://www.dropbox.com/s/h1rqcejfapbfwb4/sample.rhai?raw=1"

let project = new_project();

let req_1 = grade_docs(["pyramid_scheme.LinkedTree"], project, 10, "1");

let req_2 = grade_by_tests(
    [("pyramid_scheme.LinkedTreeTest")],
    [
        "pyramid_scheme.LinkedTreeTest#testGetRootElement",
        "pyramid_scheme.LinkedTreeTest#testAddChild",
        "pyramid_scheme.LinkedTreeTest#testFindNode",
        "pyramid_scheme.LinkedTreeTest#testContains",
        "pyramid_scheme.LinkedTreeTest#testSize",
    ],
    project,
    20.0,
    "2",
);

let req_3 = grade_unit_tests(
    "2",
    20.0,
    ["pyramid_scheme.LinkedTreeTest"],
    [
        "pyramid_scheme.LinkedTreeTest#testGetRootElement",
        "pyramid_scheme.LinkedTreeTest#testAddChild",
        "pyramid_scheme.LinkedTreeTest#testFindNode",
        "pyramid_scheme.LinkedTreeTest#testContains",
        "pyramid_scheme.LinkedTreeTest#testSize",
    ],
    [],
    []
);

let req_4 = grade_docs(
    ["pyramid_scheme.PyramidScheme"],
    project,
    10,
    "3",
);

let req_5 = grade_by_tests(
    ["pyramid_scheme.PyramidSchemeTest"],
    [
        "pyramid_scheme.PyramidSchemeTest#testWhoBenefits",
        "pyramid_scheme.PyramidSchemeTest#testAddChild",
        "pyramid_scheme.PyramidSchemeTest#testInitiateCollapse",
    ],
    project,
    30.0,
    "3",
);

let req_6 = grade_by_hidden_tests(
    "https://www.dropbox.com/s/47jd1jru1f1i0cc/ABCTest.java?raw=1",
    "ABCTest",
    30.0,
    "4"
);

show_results([req_1, req_2, req_3, req_4, req_5, req_6]);
```

### Output

## License

See `license.html` for a list of all licenses used in this project.