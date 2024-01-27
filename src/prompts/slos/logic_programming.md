## Basic Programming SLO - Logic

Objective: The logical flow and chosen control structures of the program are appropriate.

Rubric:

- Exemplary (5): The program demonstrates a clear and optimal logical flow, with well-chosen control structures. The program considers factors beyond what is expected (e.g., concurrency, bias, privacy, security, modularity, maintainability, avoiding redudancy).

- Accomplished (4): The program's logic is generally sound, and the chosen control structures are appropriate / efficient, leading to a coherent and comprehensible final product. Examples: the program avoids unnecessary operations / convoluted logic, utilizes loops when relevant, avoids unnecessary nesting of control structures.

- Acceptable (3): The program's logic may contain minor errors and/or inefficiencies; some chosen control structures can be simplified for readability/clarity.

- Needs Improvement (2): The program's logic has multiple issues (e.g., the chosen control structures are not appropriate for the task), leading to confusion and inefficiencies.

- Beginner (1): The program's logic is severely flawed (e.g., the chosen control structures impede the functionality and understandability of the code, program's flow is disjointed or non-functional).

## Feedback template

```template
## Logic - {{ Feedback_title }}

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

You absolutely MUST follow this template, as the system will look for a specific string as follows to determine the number of stars per your assessment:

- Exemplary (5): `### Proficiency: *****`
- Accomplished (4): `### Proficiency: ****`
- Acceptable (3): `### Proficiency: ***`
- Needs Improvement (2): `### Proficiency: **`
- Beginner (1): `### Proficiency: *`
