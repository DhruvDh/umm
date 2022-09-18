window.SIDEBAR_ITEMS = {"fn":[["grade_by_hidden_tests","Grades using hidden tests. Test file is downloaded, ran, and then cleaned up before returning."],["grade_by_hidden_tests_script","Macro generated variant of #fn_name that returns EvalAltResult. This allows the function to be used in scripts."],["grade_by_tests","Grades by running tests, and reports how many tests pass. Final grade is the same percentage of maximum grade as the number of tests passing."],["grade_by_tests_script","Macro generated variant of #fn_name that returns EvalAltResult. This allows the function to be used in scripts."],["grade_docs","Grades documentation by using the -Xdoclint javac flag. Scans javac output for generated warnings and grades accordingly. TODO: have customizable grade penalties"],["grade_docs_script","Macro generated variant of #fn_name that returns EvalAltResult. This allows the function to be used in scripts."],["grade_unit_tests","Runs mutation tests using Pitest to grade unit tests written by students."],["grade_unit_tests_script","Macro generated variant of #fn_name that returns EvalAltResult. This allows the function to be used in scripts."],["show_result","Print grade result"]],"mod":[["parser","includes some useful grammars for parsing JUNit/javac/pitest outputs."]],"struct":[["GradeResult","A struct to store grading results and display them"],["JavacDiagnostic","A struct representing a javac diagnostic message TODO: figure out if the dead code fields are actually needed"],["MutationDiagnostic","A struct representing a PIT diagnostic message TODO: figure out if the dead code fields are actually needed"]]};