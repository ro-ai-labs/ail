app TicketApi

things:
  thing Ticket
    field id: Text
    field status: State<New, Open>
    field title: Text
    field assignee: Text
    field primary_tag: Text
    field request_id: Text

collections:
  collection tickets: Ticket

endpoints:
  endpoint GET /tickets -> ListTickets
    requires:
      auth.bearer_token is auth.expected_token
    respond:
      total: Int
      ids: List<Text>
      items: List<Ticket>
      total = tickets.count
      ids = tickets.keys_json
      items = tickets.records
    error:
      status: 401 Unauthorized
      code: Text
      message: Text
      code = failure
      message = failure
  endpoint GET /tickets/status/{status} -> ListTickets
    requires:
      auth.bearer_token is auth.expected_token
    respond:
      total: Int
      ids: List<Text>
      items: List<Ticket>
      total = tickets[status=status].count
      ids = tickets[status=status].keys_json
      items = tickets[status=status].records
    error:
      status: 401 Unauthorized
      code: Text
      message: Text
      code = failure
      message = failure
  endpoint GET /tickets/{id} -> ListTickets
    requires:
      auth.bearer_token is auth.expected_token
      tickets[id].id exists else NotFound
    respond:
      item: Ticket
      id: Text
      status: State<New, Open>
      title: Text
      assignee: Text
      item = tickets[id].record
      id = tickets[id].id
      status = tickets[id].status
      title = tickets[id].title
      assignee = tickets[id].assignee
    error:
      status: 401 Unauthorized
      code: Text
      message: Text
      code = failure
      message = failure
    error NotFound:
      status: 404 Not Found
      code: Text
      message: Text
      code = failure
      message = "ticket not found"
  endpoint POST /tickets/{id} -> OpenTicket
    requires:
      auth.bearer_token is auth.expected_token
    request:
      title: Text
      assignee: Text
      tags[0].name: Text
    bind:
      ticket.id = id
      ticket.title = title
      ticket.assignee = assignee
      ticket.primary_tag = tags[0].name
      ticket.request_id = headers.x_request_id
    respond:
      id: Text
      status: State<New, Open>
      tag: Text
      request_id: Text
      created: Bool
      total: Int
      meta: Map<Text, Text>
      status: 201 Created
      id = ticket_id
      status = ticket_status
      tag = ticket.primary_tag
      request_id = ticket.request_id
      created = true
      total = tickets.count
      meta = {"kind":"ticket"}
    error:
      status: 401 Unauthorized
      code: Text
      message: Text
      code = failure
      message = failure
    error BadRequest:
      status: 400 Bad Request
      code: Text
      message: Text
      code = failure
      message = failure
  endpoint DELETE /tickets/{id} -> CloseTicket
    requires:
      auth.bearer_token is auth.expected_token
    bind:
      ticket.id = id
      ticket = tickets[id]
    respond:
      id: Text
      id = ticket_id
    error:
      status: 401 Unauthorized
      code: Text
      message: Text
      code = failure
      message = failure

intent ListTickets

intent OpenTicket

subject:
  ticket: Ticket

steps:
  1. Open ticket
     set: ticket.status = Open
     set: tickets[ticket.id] = ticket

returns:
  ticket_id: ticket.id
  ticket_status: ticket.status

intent CloseTicket

subject:
  ticket: Ticket

steps:
  1. Delete ticket
     delete: tickets[ticket.id]

returns:
  ticket_id: ticket.id
