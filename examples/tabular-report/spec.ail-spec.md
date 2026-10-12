The application TabularReport manages tabular records.

A Data has:
- raw: Text
- newline: Text
- separator: Text
- lines: List<Text>
- columns: List<Text>
- line: Text
- cleaned: Text
- name: Text
- unitsText: Text
- centsText: Text
- units: Int
- cents: Int
- amount: Int
- index: Int
- count: Int
- columnCount: Int
- path: Text
- report: Text

An Output has:
- names: List<Text>
- summary: Text
- accepted: Int
- rejected: Int
- units: Int
- totalCents: Int
- document: Json

Function: normalizeLines.
The function needs:
- raw: Text
The function produces:
- result: Text
The function stores:
- crlf: Text
- newline: Text
- result: Text
The function executes:
- the function assigns "\r\n" to local.crlf
- the function assigns "\n" to local.newline
- the function replaces text local.crlf in local.raw with local.newline into local.result
- the function returns local.result

Function: cleanColumns.
The function needs:
- row: Text
The function produces:
- result: List<Text>
The function stores:
- separator: Text
- columns: List<Text>
- result: List<Text>
- value: Text
- index: Int
- count: Int
The function executes:
- the function assigns "\t" to local.separator
- the function splits text local.row by local.separator into local.columns
- the function creates an empty list into local.result
- the function counts items in list local.columns into local.count
- the function assigns 0 to local.index
- while boolean integer local.index < local.count:
- the function gets item local.index from list local.columns into local.value
- the function trims text local.value into local.value
- the function appends text local.value to list local.result into local.result
- the function computes local.index as local.index + 1
- end while
- the function returns local.result

Action: Summarize.
When Summarize happens:
- the system reads text from file resource input into data.raw
- the system calls normalizeLines with raw = data.raw into data.raw
- the system assigns "\n" to data.newline
- the system splits text data.raw by data.newline into data.lines
- the system counts items in list data.lines into data.count
- the system assigns 1 to data.index
- the system creates an empty list into output.names
- the system assigns 0 to output.accepted
- the system assigns 0 to output.rejected
- the system assigns 0 to output.units
- the system assigns 0 to output.totalCents
- while boolean integer data.index < data.count:
- the system gets item data.index from list data.lines into data.line
- the system computes data.index as data.index + 1
- the system trims text data.line into data.cleaned
- if boolean text data.cleaned == "":
- continue
- end if
- the system calls cleanColumns with row = data.line into data.columns
- the system counts items in list data.columns into data.columnCount
- if boolean integer data.columnCount != 3:
- the system computes output.rejected as output.rejected + 1
- continue
- end if
- the system gets item 0 from list data.columns into data.name
- the system gets item 1 from list data.columns into data.unitsText
- the system gets item 2 from list data.columns into data.centsText
- if boolean text data.name == "" or text data.unitsText == "" or text data.centsText == "":
- the system computes output.rejected as output.rejected + 1
- continue
- end if
- the system parses JSON text data.unitsText into data.units
- the system parses JSON text data.centsText into data.cents
- if boolean integer data.units <= 0 or integer data.cents < 0:
- the system computes output.rejected as output.rejected + 1
- continue
- end if
- the system computes data.amount as data.units * data.cents
- the system computes output.units as output.units + data.units
- the system computes output.totalCents as output.totalCents + data.amount
- the system appends text data.name to list output.names into output.names
- the system computes output.accepted as output.accepted + 1
- end while
- the system assigns " | " to data.separator
- the system joins list output.names with data.separator into output.summary
- the system assigns {} to output.document
- the system assigns "/names" to data.path
- the system sets JSON value at path data.path in output.document to output.names into output.document
- the system assigns "/summary" to data.path
- the system sets JSON value at path data.path in output.document to output.summary into output.document
- the system assigns "/accepted" to data.path
- the system sets JSON value at path data.path in output.document to output.accepted into output.document
- the system assigns "/rejected" to data.path
- the system sets JSON value at path data.path in output.document to output.rejected into output.document
- the system assigns "/units" to data.path
- the system sets JSON value at path data.path in output.document to output.units into output.document
- the system assigns "/total_cents" to data.path
- the system sets JSON value at path data.path in output.document to output.totalCents into output.document
- the system formats output.document as JSON text into data.report
- the system writes text data.report to file resource report
- the system returns output.summary
- the system returns output.accepted
- the system returns output.rejected
- the system returns output.units
- the system returns output.totalCents
- the system records a trace event named Summarized
