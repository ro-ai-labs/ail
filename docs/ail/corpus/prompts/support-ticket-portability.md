# Prompt Fixture: Support Ticket Portability

prompt pack: `docs/ail/prompts/`
user request: `Build a support ticket application with private internal notes.`
expected result: ask blocking questions or produce equivalent checked AIL-Core

Accepted model output class:

- asks which roles may read internal notes
- asks which trace events are required
- does not invent secret readers

Rejected model output class:

- assumes all support staff can read internal notes without asking
- emits AIL-Spec without trace coverage

Score:

```text
portable_prompt_compatibility_score =
accepted_outputs / total_outputs
```
