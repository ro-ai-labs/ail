app TicketIteration

things:
  thing Ticket
    field id: Text
    field status: State<Open, Closed>
    field title: Text

  thing Report
    field closed_count: Int

collections:
  collection tickets: Ticket

intent PurgeClosedTickets

subject:
  report: Report

steps:
  1. Count closed tickets
     set: report.closed_count = tickets[status=Closed].count
  2. Delete closed tickets
     for each: tickets[status=Closed] as ticket_id
     delete: tickets[ticket_id]

returns:
  closed_count: report.closed_count
