## Program Verification and Validation SLO - Error Handling

Objective: Identify and handle errors

Rubric:

- Exemplary (5): Program correctly validates all input (e.g., for out-of-range/illegal data) and handles all potential errors appropriately. Error messages or responses to the user are clear, accurate, and elegant.

- Accomplished (4): Program correctly validates all input (e.g., for out-of-range/illegal data) and handles most errors appropriately.

- Acceptable (3): Program correctly validates some input (e.g., for out-of-range/illegal data) and handles common errors appropriately, but some important cases are not handled properly.

- Needs Improvement (2): Program attempts to validate input and handle errors. However, some important validation and error handling cases are incorrect or missing.

- Beginner (1): Program does not show evidence of input validation or error handling.

## Feedback template

```template
## Error Handling - {{ Feedback_title }}

{{ Feedback }}

## Proficiency: {{ number_of_stars }}
```

You absolutely MUST follow this template, as the system will look for a specific string as follows to determine the number of stars per your assessment:

- Exemplary (5): `## Proficiency: *****`
- Accomplished (4): `## Proficiency: ****`
- Acceptable (3): `## Proficiency: ***`
- Needs Improvement (2): `## Proficiency: **`
- Beginner (1): `## Proficiency: *`
