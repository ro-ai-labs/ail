app SessionInbox

things:
  thing Inbox
    field id: Text
    field last_message: Text

collections:
  collection inboxes: Inbox

endpoints:
  endpoint POST /inbox -> StoreMessage
    requires:
      auth.session_id is auth.expected_session
    bind:
      inbox.id = auth.session_id
      inbox.last_message = message

intent StoreMessage

subject:
  inbox: Inbox

steps:
  1. Store message
     set: inboxes[inbox.id].id = inbox.id
     set: inboxes[inbox.id].last_message = inbox.last_message

returns:
  inbox_id: inbox.id
  inbox_last_message: inbox.last_message
