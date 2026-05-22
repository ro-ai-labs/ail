app QueueDrain

things:
  thing Queue
    field count: Int

intent DrainQueue

subject:
  queue: Queue

steps:
  1. Drain one item
     repeat while: queue.count > 0
     compute: queue.count = queue.count - 1
     changes: queue.count

guarantees:
  if this intent succeeds:
    queue.count is 0
