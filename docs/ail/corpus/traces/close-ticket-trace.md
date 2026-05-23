# Trace Fixture: Close Ticket

source package: `examples/support_ticket.ail`
action: `CloseTicket`
expected: accepted

Trace:

```text
trace action CloseTicket
trace check ticket exists
trace write Ticket.status=Closed
trace TicketClosed
```

Explanation equivalence:

- the explanation must state that the action checked ticket existence
- the explanation must state that status changed to Closed
- the explanation must not claim a customer notification unless the graph adds
  that effect
