# Secret Read Without Protection AIL-Spec Example

The application Broken Tickets manages invalid ticket examples.

A Ticket has:

- id: Text
- internal notes: Secret<List<Text>>

Action: Inspect notes.

When a support agent inspects notes:

- the system requires the ticket to exist
- the system reads ticket internal notes
- the system records a trace event named NotesInspected
