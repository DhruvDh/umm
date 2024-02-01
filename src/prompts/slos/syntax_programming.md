## Basic Programming SLO - Syntax

Objective: Program uses valid syntax for basic statements, expressions, control structures, and data structures

Rubric:

- Exemplary (5): Program adheres to valid syntax for statements, expressions, control structures, and data structures in the chosen programming language. When multiple equivalent and valid syntax choices exist for an operation, the simplest or easiest to read is used consistently throughout the program.

- Accomplished (4): Program adheres to valid syntax for statements, expressions, control structures, and data structures in the chosen programming language. For compiled languages, this means the program compiles with no errors; for interpreted languages, this means the program runs with no syntax errors.

- Acceptable (3): Program uses mostly valid syntax for statements, expressions, control structures, and data structures in the chosen programming language. For compiled languages, this means the program has a few minor compile-time errors; for interpreted languages, this means the program contains a few minor syntax errors. These syntax errors are not pervasive and can be fixed by the student with minimal feedback.

- Needs Improvement (2): Program includes significant and/or frequent syntax errors, causing it not to compile/run successfully and signaling that the student may have misunderstanding of language syntax.

- Beginner (1): Program does not compile/run successfully due to widespread use of invalid syntax, making it very hard to understand the student's intent.

## Feedback template

> Note that feedback is to be shared as a valid Markdown file. The begining "```template " and the ending "``` " are not part of the feedback, but are used to denote the start and end of the template.

```template
## Syntax - {{ Feedback_title }}

{{ Snippet_from_submission_1 }}

{{ Feedback_1 }}

{{ Snippet_from_submission_2 }}

{{ Feedback_2 }}
...

{{ Snippet_from_submission_N }}

{{ Feedback_N }}

---

### Proficiency: {{ number_of_stars }}

{{ tips_and_suggestions_to_improve }}
```

`number_of_stars` here must be formatted as `***` for Acceptable (3). You absolutely MUST follow this template, as the system will look for these specific strings as shown below to determine the proficiency level for the student.

- Exemplary (5): `### Proficiency: *****`
- Accomplished (4): `### Proficiency: ****`
- Acceptable (3): `### Proficiency: ***`
- Needs Improvement (2): `### Proficiency: **`
- Beginner (1): `### Proficiency: *`

If the template includes `### Proficiency: ***`, the system will automatically assess the student as having met the Acceptable (3) level of proficiency.
