imports:
  import ticket_shared.rif.md

app TicketApi

things:
  thing Report
    field ticket_id: Text

intent CloseTicket

subject:
  report: Report
  ticket: Ticket

steps:
  1. Close ticket
     set: report.ticket_id = ticket.id
