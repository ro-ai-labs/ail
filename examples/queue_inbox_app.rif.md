app QueueInbox

things:
  thing Inbox
    field id: Text
    field last_message: Text

collections:
  collection inboxes: Inbox

triggers:
  trigger support.inbox -> StoreMessage
    queue: support.inbox
    payload:
      message_id: Text
      message: Text
    requires:
      event.kind is queue
      event.queue is support.inbox
    bind:
      inbox.id = event.message_id
      inbox.last_message = event.message

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
