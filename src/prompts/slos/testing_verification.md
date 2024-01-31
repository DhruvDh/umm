## Program Verification and Validation SLO - Testing

Objective: Design and write effective tests for programs.

Rubric:

- Exemplary (5): Student is able to design and write accurate tests for all functionality, correctly identifying all possible scenarios, including typical cases and exceptional/illegal/boundary cases. Test cases not only consider correctness of the program, but also other characteristics applicable to production-quality code, e.g., reliability, scalability, efficiency, bias, etc.

- Accomplished (4): Student is able to design and write accurate tests for all functionality, correctly identifying most expected scenarios, including typical cases and exceptional/illegal/boundary cases.

- Acceptable (3): Student is able to design and write accurate tests for most functionality, correctly identifying most common scenarios, but sometimes missing atypical/exceptional/illegal/boundary cases. Some tests may produce inaccurate results.

- Needs Improvement (2): Student attempts to examine correctness of the functionality through tests, but tests are not comprehensive, are inaccurate and/or miss critical/common scenarios.

- Beginner (1): Student did not show evidence of testing.

## Feedback template

```template
## Testing - {{ Feedback_title }}

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
