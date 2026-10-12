The application RecoveringJsonLines manages numeric batch rows.

A Data has:
- raw: Text
- separator: Text
- lines: List<Text>
- line: Text
- count: Int
- index: Int
- length: Int
- row: Json
- accepted: Bool
- value: Int
- error: Text
- path: Text
- prefix: Text
- errorCount: Int
- report: Text

An Output has:
- rows: Int
- accepted: Int
- rejected: Int
- values: List<Int>
- errors: Json
- document: Json

Function: parseRow.
The function needs:
- text: Text
The function produces:
- result: Json
The function stores:
- record: Json
- result: Json
- path: Text
- value: Int
- divisor: Int
- quotient: Int
- accepted: Bool
- error: Text
The function executes:
- the function assigns {"accepted":false,"value":0,"error":""} to local.result
- try:
- the function parses JSON text local.text into local.record
- the function assigns "/value" to local.path
- the function gets JSON value at path local.path from local.record into local.value
- the function assigns "/divisor" to local.path
- the function gets JSON value at path local.path from local.record into local.divisor
- the function computes local.quotient as local.value / local.divisor
- the function assigns "/value" to local.path
- the function sets JSON value at path local.path in local.result to local.quotient into local.result
- the function assigns true to local.accepted
- the function assigns "/accepted" to local.path
- the function sets JSON value at path local.path in local.result to local.accepted into local.result
- recover JSON or JSON-PATH or DIVZERO or OVERFLOW into local.error:
- the function assigns "/error" to local.path
- the function sets JSON value at path local.path in local.result to local.error into local.result
- end try
- the function returns local.result

Action: Analyze.
When Analyze happens:
- the system reads text from file resource input into data.raw
- the system assigns "\n" to data.separator
- the system splits text data.raw by data.separator into data.lines
- the system counts items in list data.lines into data.count
- the system assigns 0 to data.index
- the system assigns 0 to output.rows
- the system assigns 0 to output.accepted
- the system assigns 0 to output.rejected
- the system creates an empty list into output.values
- the system assigns {"AIL-RUNTIME-DIVZERO":0,"AIL-RUNTIME-JSON":0,"AIL-RUNTIME-JSON-PATH":0,"AIL-RUNTIME-OVERFLOW":0} to output.errors
- while data.index < data.count:
- the system gets item data.index from list data.lines into data.line
- the system computes data.index as data.index + 1
- the system trims text data.line into data.line
- the system counts characters in data.line into data.length
- if data.length == 0:
- continue
- end if
- the system computes output.rows as output.rows + 1
- the system calls parseRow with text = data.line into data.row
- the system assigns "/accepted" to data.path
- the system gets JSON value at path data.path from data.row into data.accepted
- if boolean data.accepted:
- the system computes output.accepted as output.accepted + 1
- the system assigns "/value" to data.path
- the system gets JSON value at path data.path from data.row into data.value
- the system appends integer data.value to list output.values into output.values
- otherwise:
- the system computes output.rejected as output.rejected + 1
- the system assigns "/error" to data.path
- the system gets JSON value at path data.path from data.row into data.error
- the system assigns "/" to data.prefix
- the system joins text data.prefix and data.error into data.path
- the system gets JSON value at path data.path from output.errors into data.errorCount
- the system computes data.errorCount as data.errorCount + 1
- the system sets JSON value at path data.path in output.errors to data.errorCount into output.errors
- end if
- end while
- the system sorts list output.values in ascending order into output.values
- the system assigns {} to output.document
- the system assigns "/rows" to data.path
- the system sets JSON value at path data.path in output.document to output.rows into output.document
- the system assigns "/accepted" to data.path
- the system sets JSON value at path data.path in output.document to output.accepted into output.document
- the system assigns "/rejected" to data.path
- the system sets JSON value at path data.path in output.document to output.rejected into output.document
- the system assigns "/values" to data.path
- the system sets JSON value at path data.path in output.document to output.values into output.document
- the system assigns "/errors" to data.path
- the system sets JSON value at path data.path in output.document to output.errors into output.document
- the system formats output.document as JSON text into data.report
- the system writes text data.report to file resource report
- the system returns output.accepted
- the system returns output.rejected
- the system records a trace event named Analyzed
