## Written Communication SLO - Comments

> Assume comments and documentation to be interchangeable terms.

Objective: Program includes clear and meaningful comments.

Rubric:

- Exemplary (5): Comments accurately use technical terminology, when appropriate, rather than colloquial or informal language. (For example: ""this block iterates over each element of X to do Y"" instead of ""loop to do Y to X"" or ""this part of the code does Y""). All submitted code is self-documenting. That is: variable, function, and object names; program flow; and program organization associated with problem decomposition are so clear that minimal additional comments are necessary.

- Accomplished (4): Program includes clear and meaningful comments at appropriate granularity and adhering to standard language conventions. Examples: Every function / class has comments indicating the intent / assumptions / expectations as relevant. Code blocks that need additional explanation have clear and meaningful comments. Program includes meaningful in-line comments where relevant.

- Acceptable (3): Most of the program has clear and meaningful comments at appropriate granularity and adhering to standard language conventions, but some portions have unclear, redundant or missing comments.

- Needs Improvement (2): Program has comments, but they need improvement in one or more of the following areas: comments are not clear and meaningful; comments are inconsistent; comments do not adhere to standard language conventions; important code blocks that need explanation do not have sufficient comments; some in-line comments are redundant or unhelpful, etc.

- Beginner (1): Program does not include clear and meaningful comments.

## Feedback template

```template
## Comments and Documentation - {{ Feedback_title }}

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
