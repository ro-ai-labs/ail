app TicketEvents

things:
  thing Ticket
    field id: Text
    field status: State<New, Open>
    field title: Text
    field assignee: Text
    field primary_tag: Text

collections:
  collection tickets: Ticket

triggers:
  trigger ticket.created -> OpenTicket
    payload:
      id: Text
      title: Text
      assignee: Text
      tags[0].name: Text
    requires:
      event.name is ticket.created
    bind:
      ticket.id = event.id
      ticket.title = event.title
      ticket.assignee = event.assignee
      ticket.primary_tag = event.tags[0].name

intent OpenTicket

subject:
  ticket: Ticket

steps:
  1. Open ticket
     set: ticket.status = Open
     set: tickets[ticket.id].id = ticket.id
     set: tickets[ticket.id].status = Open
     set: tickets[ticket.id].title = ticket.title
     set: tickets[ticket.id].assignee = ticket.assignee
     set: tickets[ticket.id].primary_tag = ticket.primary_tag

returns:
  ticket_id: ticket.id
  ticket_status: ticket.status
