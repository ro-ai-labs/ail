The application Constants manages batches.

A Data has:
- numbers: List<Int>
- index: Int
- count: Int
- item: Int
- greeting: Text
- name: Text

An Output has:
- message: Text
- squares: List<Int>
- total: Int
- ready: Bool

Function: square.
The function needs:
- n: Int
The function produces:
- result: Int
The function executes:
- the function returns local.n * local.n

Action: Demo.
When Demo happens:
- the system assigns [1, -2, 3] to data.numbers
- the system assigns [] to output.squares
- the system assigns 0 to output.total
- the system assigns 0 to data.index
- the system assigns "Hello, " to data.greeting
- the system assigns "world! 🙂" to data.name
- the system assigns false to output.ready
- the system joins text data.greeting and data.name into output.message
- the system counts items in list data.numbers into data.count
- while data.index < data.count:
- the system gets item data.index from list data.numbers into data.item
- the system calls square with n = data.item into data.item
- the system appends integer data.item to list output.squares into output.squares
- the system computes output.total as output.total + data.item
- the system computes data.index as data.index + 1
- end while
- the system assigns true to output.ready
- the system returns output.message
- the system returns output.squares
- the system returns output.total
- the system returns output.ready
- the system records a trace event named Constructed
