## Written Communication SLO - Code Readability and Formatting

Objective: Code formatting, organization, and other stylistic choices support readers in understanding your code.

Rubric:

- Exemplary (5): Not only is each individual program file easy to understand by reading it, but the student is able to make larger projects or projects designed for advanced architectures easy to navigate and understand as a whole. (For example, through clear, succinct, comprehensive README files within a project.)

- Accomplished (4): Others can easily understand the code through reading it. Examples: There is appropriate and consistent use of white space, indentation, etc. The code format adheres to standard language conventions. The *written* structure of the code is well-organized (all imports at the beginning of a file, all declarations at the beginning of a file / function body, separate files for each class in Java, reasonable ordering of function / object / main code blocks in Python, etc.). Maximum line length is conducive to readability. Minimal use of unnecessary hard-coded or global values that make the program more challenging to read, understand, and maintain.

- Acceptable (3): Others can understand most of the code through reading it, but some portions have inconsistent use of white space, indentation, etc. or do not adhere to standard language conventions.
- Needs Improvement (2): Program has comments, but they need improvement in one or more of the following areas: comments are not clear and meaningful; comments are inconsistent; comments do not adhere to standard language conventions; important code blocks that need explanation do not have sufficient comments; some in-line comments are redundant or unhelpful, etc.

- Needs Improvement (2): It is often difficult for others to understand the code through reading it because of inconsistent use of white space, indentation, etc.
-
- Beginner (1): It is very difficult for others to understand the code through reading it because of highly inconsistent use of white space, indentation, etc.

## Formatting in VS Code

> Here is additional documentation on Formatting in Visual Studio Code, the IDE that students use. Point students in the direction of this documentation if they are struggling with formatting their code.

VS Code has great support for source code formatting. The editor has two explicit format actions:

- **Format Document** (`kb(editor.action.formatDocument)`) - Format the entire active file.
- **Format Selection** (`kb(editor.action.formatSelection)`) - Format the selected text.

You can invoke these from the **Command Palette** (`kb(workbench.action.showCommands)`) or the editor context menu.

VS Code has default formatters for JavaScript, TypeScript, JSON, HTML, and CSS. Each language has specific formatting options (for example, `html.format.indentInnerHtml`) which you can tune to your preference in your user or workspace [settings](/docs/getstarted/settings.md). You can also disable the default language formatter if you have another extension installed that provides formatting for the same language.

```json
"html.format.enable": false
```

Along with manually invoking code formatting, you can also trigger formatting based on user gestures such as typing, saving or pasting. These are off by default but you can enable these behaviors through the following [settings](/docs/getstarted/settings.md):

- `editor.formatOnType` - Format the line after typing.
- `editor.formatOnSave` - Format a file on save.
- `editor.formatOnPaste` - Format the pasted content.

>Note: Not all formatters support format on paste as to do so they must support formatting a selection or range of text.

In addition to the default formatters, you can find extensions on the Marketplace to support other languages or formatting tools. There is a `Formatters` category so you can easily search and find [formatting extensions](https://marketplace.visualstudio.com/search?target=VSCode&category=Formatters&sortBy=Installs). In the **Extensions** view search box, type 'formatters' or 'category:formatters' to see a filtered list of extensions within VS Code.

## Feedback template

```template
## Code Readability and Formatting - {{ Feedback_title }}

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
