The application JsonBatch manages batches.

A Data has:
- raw: Text
- numbers: List<Int>
- index: Int
- item: Int
- squaresJson: Text
- totalJson: Text

An Output has:
- count: Int
- squares: List<Int>
- total: Int

Function: square.
The function needs:
- n: Int
The function produces:
- result: Int
The function executes:
- the function returns local.n * local.n

Action: Process.
When Process happens:
- the system reads text from file resource input into data.raw
- the system parses JSON text data.raw into data.numbers
- the system assigns [] to output.squares
- the system assigns 0 to output.total
- the system assigns 0 to data.index
- the system counts items in list data.numbers into output.count
- while data.index < output.count:
- the system gets item data.index from list data.numbers into data.item
- the system calls square with n = data.item into data.item
- the system appends integer data.item to list output.squares into output.squares
- the system computes output.total as output.total + data.item
- the system computes data.index as data.index + 1
- end while
- the system formats output.squares as JSON text into data.squaresJson
- the system formats output.total as JSON text into data.totalJson
- the system calls Save
- the system returns output.count
- the system returns output.total
- the system records a trace event named Processed

Action: Save.
When Save happens:
- the system writes text data.squaresJson to file resource squares
- the system writes text data.totalJson to file resource total
- the system records a trace event named Saved
