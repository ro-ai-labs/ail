# Accepted Core Fixture: Support Ticket

source: `docs/ail/examples/support-ticket.ail-core.md`
expected: accepted
profile: Application

Acceptance:

- graph contains `Action CloseTicket`
- graph contains `Trace TicketClosed`
- graph contains secret protection for internal notes
