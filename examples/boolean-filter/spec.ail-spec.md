The application BooleanFilter manages work records.

A Data has:
- raw: Text
- records: Json
- record: Json
- path: Text
- name: Text
- kind: Text
- enabled: Bool
- count: Int
- score: Float
- selected: Bool
- index: Int
- length: Int
- report: Text

An Output has:
- names: List<Text>
- accepted: Int
- rejected: Int
- document: Json

Function: eligible.
The function needs:
- enabled: Bool
- kind: Text
- count: Int
- score: Float
The function produces:
- result: Bool
The function stores:
- result: Bool
The function executes:
- the function computes boolean local.result as local.enabled and (text local.kind == "task" or text local.kind == "issue") and integer local.count > 0 and floating local.score >= 0.75
- the function returns local.result

Action: Filter.
When Filter happens:
- the system reads text from file resource input into data.raw
- the system parses JSON text data.raw into data.records
- the system counts entries in JSON data.records into data.length
- the system assigns 0 to data.index
- the system assigns 0 to output.accepted
- the system assigns 0 to output.rejected
- the system creates an empty list into output.names
- while boolean integer data.index < data.length:
- the system formats data.index as JSON text into data.path
- the system assigns "/" to data.raw
- the system joins text data.raw and data.path into data.path
- the system gets JSON value at path data.path from data.records into data.record
- the system assigns "/name" to data.path
- the system gets JSON value at path data.path from data.record into data.name
- the system assigns "/kind" to data.path
- the system gets JSON value at path data.path from data.record into data.kind
- the system assigns "/enabled" to data.path
- the system gets JSON value at path data.path from data.record into data.enabled
- the system assigns "/count" to data.path
- the system gets JSON value at path data.path from data.record into data.count
- the system assigns "/score" to data.path
- the system gets JSON value at path data.path from data.record into data.score
- the system calls eligible with enabled = data.enabled; kind = data.kind; count = data.count; score = data.score into data.selected
- if boolean data.selected:
- the system appends text data.name to list output.names into output.names
- the system computes output.accepted as output.accepted + 1
- otherwise:
- the system computes output.rejected as output.rejected + 1
- end if
- the system computes data.index as data.index + 1
- end while
- the system assigns {} to output.document
- the system assigns "/names" to data.path
- the system sets JSON value at path data.path in output.document to output.names into output.document
- the system assigns "/accepted" to data.path
- the system sets JSON value at path data.path in output.document to output.accepted into output.document
- the system assigns "/rejected" to data.path
- the system sets JSON value at path data.path in output.document to output.rejected into output.document
- the system formats output.document as JSON text into data.report
- the system writes text data.report to file resource report
- the system returns output.names
- the system returns output.accepted
- the system returns output.rejected
- the system records a trace event named Filtered
