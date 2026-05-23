# Interview Fixture: Support Ticket

status: accepted question set
prompt: `prompts/interview.system.md`

User request:

```text
Build a support ticket application with internal notes.
```

Required questions:

- Which roles may read internal notes?
- Which actions can change ticket status?
- Which trace events must be recorded for create, assign, close, and overdue
  transitions?
- What should happen when a ticket is not found?

Acceptance:

- the agent must ask about secret readers before drafting AIL-Spec
- the agent must ask about trace events before producing an executable spec
