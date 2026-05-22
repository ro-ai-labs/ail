app TicketApi

things:
  thing Ticket
    field id: Text
    field status: State<Open, Closed>

intent ArchiveTicket

subject:
  ticket: Ticket

steps:
  1. Archive ticket
     set: ticket.status = Closed
