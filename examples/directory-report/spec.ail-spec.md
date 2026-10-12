The application DirectoryReport manages discovered numeric batch files.

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
- pointer: Text
- prefix: Text
- errorCount: Int
- report: Text

A Report has:
- rows: Int
- accepted: Int
- rejected: Int
- values: List<Int>
- errors: Json
- document: Json

A Batch has:
- names: List<Text>
- name: Text
- root: Text
- count: Int
- index: Int
- indexText: Text
- prefix: Text
- suffix: Text
- reportPath: Text
- entry: Json
- document: Json
- error: Text
- written: Int
- failed: Int

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

Action: Generate.
When Generate happens:
- the system assigns "" to batch.root
- the system lists names in directory resource workspace at batch.root into batch.names
- the system counts items in list batch.names into batch.count
- the system assigns 0 to batch.index
- the system assigns 0 to batch.written
- the system assigns 0 to batch.failed
- the system assigns "result-" to batch.prefix
- the system assigns ".json" to batch.suffix
- the system assigns {"entries":[]} to batch.document
- while batch.index < batch.count:
- the system gets item batch.index from list batch.names into batch.name
- the system assigns {"name":"","report":"","error":""} to batch.entry
- the system assigns "/name" to data.pointer
- the system sets JSON value at path data.pointer in batch.entry to batch.name into batch.entry
- try:
- the system reads text from directory resource workspace at batch.name into data.raw
- the system calls Summarize
- the system formats batch.index as JSON text into batch.indexText
- the system joins text batch.prefix and batch.indexText into batch.reportPath
- the system joins text batch.reportPath and batch.suffix into batch.reportPath
- the system writes text data.report to directory resource reports at batch.reportPath
- the system assigns "/report" to data.pointer
- the system sets JSON value at path data.pointer in batch.entry to batch.reportPath into batch.entry
- the system computes batch.written as batch.written + 1
- recover PATH or TEXT into batch.error:
- the system assigns "/error" to data.pointer
- the system sets JSON value at path data.pointer in batch.entry to batch.error into batch.entry
- the system computes batch.failed as batch.failed + 1
- end try
- the system assigns "/entries/-" to data.pointer
- the system sets JSON value at path data.pointer in batch.document to batch.entry into batch.document
- the system computes batch.index as batch.index + 1
- end while
- the system formats batch.document as JSON text into data.report
- the system assigns "manifest.json" to batch.reportPath
- the system writes text data.report to directory resource reports at batch.reportPath
- the system returns batch.count
- the system returns batch.written
- the system returns batch.failed
- the system records a trace event named Generated

Action: Summarize.
When Summarize happens:
- the system assigns "\n" to data.separator
- the system splits text data.raw by data.separator into data.lines
- the system counts items in list data.lines into data.count
- the system assigns 0 to data.index
- the system assigns 0 to report.rows
- the system assigns 0 to report.accepted
- the system assigns 0 to report.rejected
- the system creates an empty list into report.values
- the system assigns {"AIL-RUNTIME-DIVZERO":0,"AIL-RUNTIME-JSON":0,"AIL-RUNTIME-JSON-PATH":0,"AIL-RUNTIME-OVERFLOW":0} to report.errors
- while data.index < data.count:
- the system gets item data.index from list data.lines into data.line
- the system computes data.index as data.index + 1
- the system trims text data.line into data.line
- the system counts characters in data.line into data.length
- if data.length == 0:
- continue
- end if
- the system computes report.rows as report.rows + 1
- the system calls parseRow with text = data.line into data.row
- the system assigns "/accepted" to data.pointer
- the system gets JSON value at path data.pointer from data.row into data.accepted
- if boolean data.accepted:
- the system computes report.accepted as report.accepted + 1
- the system assigns "/value" to data.pointer
- the system gets JSON value at path data.pointer from data.row into data.value
- the system appends integer data.value to list report.values into report.values
- otherwise:
- the system computes report.rejected as report.rejected + 1
- the system assigns "/error" to data.pointer
- the system gets JSON value at path data.pointer from data.row into data.error
- the system assigns "/" to data.prefix
- the system joins text data.prefix and data.error into data.pointer
- the system gets JSON value at path data.pointer from report.errors into data.errorCount
- the system computes data.errorCount as data.errorCount + 1
- the system sets JSON value at path data.pointer in report.errors to data.errorCount into report.errors
- end if
- end while
- the system sorts list report.values in ascending order into report.values
- the system assigns {} to report.document
- the system assigns "/rows" to data.pointer
- the system sets JSON value at path data.pointer in report.document to report.rows into report.document
- the system assigns "/accepted" to data.pointer
- the system sets JSON value at path data.pointer in report.document to report.accepted into report.document
- the system assigns "/rejected" to data.pointer
- the system sets JSON value at path data.pointer in report.document to report.rejected into report.document
- the system assigns "/values" to data.pointer
- the system sets JSON value at path data.pointer in report.document to report.values into report.document
- the system assigns "/errors" to data.pointer
- the system sets JSON value at path data.pointer in report.document to report.errors into report.document
- the system formats report.document as JSON text into data.report
- the system records a trace event named Summarized
