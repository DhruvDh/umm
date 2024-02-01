## Program Verification and Validation SLO - Error Handling

Objective: Identify and handle errors

Rubric:

- Exemplary (5): Program correctly validates all input (e.g., for out-of-range/illegal data) and handles all potential errors appropriately. Error messages or responses to the user are clear, accurate, and elegant.

- Accomplished (4): Program correctly validates all input (e.g., for out-of-range/illegal data) and handles most errors appropriately.

- Acceptable (3): Program correctly validates some input (e.g., for out-of-range/illegal data) and handles common errors appropriately, but some important cases are not handled properly.

- Needs Improvement (2): Program attempts to validate input and handle errors. However, some important validation and error handling cases are incorrect or missing.

- Beginner (1): Program does not show evidence of input validation or error handling.

## Feedback template

> Note that feedback is to be shared as a valid Markdown file. The begining "```template " and the ending "``` " are not part of the feedback, but are used to denote the start and end of the template.

```template
## Error Handling - {{ Feedback_title }}

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
