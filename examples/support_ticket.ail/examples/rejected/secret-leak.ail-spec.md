# Secret Leak AIL-Spec Example

The application Broken Tickets manages invalid ticket examples.

A Ticket has:

- id: Text
- internal notes: Secret<List<Text>>

Action: Publish notes.

When a support agent publishes notes:

- the system requires the ticket to exist
- the system changes customer visible internal notes
- the system records a trace event named NotesPublished
