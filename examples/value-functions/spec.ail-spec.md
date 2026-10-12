The application ValueFunctions manages reports.

A Data has:
- raw: Text
- document: Json
- records: Json
- record: Json
- path: Text
- indexText: Text
- slash: Text
- namePath: Text
- amountPath: Text
- appendPath: Text
- totalPath: Text
- countPath: Text
- namesPath: Text
- index: Int
- name: Text
- amount: Int
- names: List<Text>
- report: Text

An Output has:
- document: Json
- count: Int
- total: Int

Function: double.
The function needs:
- n: Int
The function produces:
- result: Int
The function executes:
- the function returns local.n * 2

Function: label.
The function needs:
- name: Text
The function produces:
- result: Text
The function stores:
- prefix: Text
- result: Text
The function executes:
- the function assigns "processed: " to local.prefix
- the function joins text local.prefix and local.name into local.result
- the function returns local.result

Function: addName.
The function needs:
- name: Text
- names: List<Text>
The function produces:
- result: List<Text>
The function stores:
- text: Text
- result: List<Text>
The function executes:
- the function calls label with name = local.name into local.text
- the function appends text local.text to list local.names into local.result
- the function returns local.result

Function: transformRecord.
The function needs:
- record: Json
The function produces:
- result: Json
The function stores:
- result: Json
- path: Text
- amount: Int
- processed: Bool
The function executes:
- the function assigns "/private" to local.path
- the function removes JSON value at path local.path from local.record into local.result
- the function assigns "/metrics/amount" to local.path
- the function gets JSON value at path local.path from local.result into local.amount
- the function calls double with n = local.amount into local.amount
- the function sets JSON value at path local.path in local.result to local.amount into local.result
- the function assigns "/processed" to local.path
- the function assigns true to local.processed
- the function sets JSON value at path local.path in local.result to local.processed into local.result
- the function returns local.result

Action: Transform.
When Transform happens:
- the system reads text from file resource input into data.raw
- the system parses JSON text data.raw into data.document
- the system assigns "/records" to data.path
- the system gets JSON value at path data.path from data.document into data.records
- the system counts entries in JSON data.records into output.count
- the system assigns {"records":[]} to output.document
- the system assigns [] to data.names
- the system assigns 0 to output.total
- the system assigns 0 to data.index
- the system assigns "/" to data.slash
- the system assigns "/name" to data.namePath
- the system assigns "/metrics/amount" to data.amountPath
- the system assigns "/records/-" to data.appendPath
- the system assigns "/total" to data.totalPath
- the system assigns "/count" to data.countPath
- the system assigns "/names" to data.namesPath
- while data.index < output.count:
- the system formats data.index as JSON text into data.indexText
- the system joins text data.slash and data.indexText into data.path
- the system gets JSON value at path data.path from data.records into data.record
- the system calls transformRecord with record = data.record into data.record
- the system gets JSON value at path data.namePath from data.record into data.name
- the system gets JSON value at path data.amountPath from data.record into data.amount
- the system sets JSON value at path data.appendPath in output.document to data.record into output.document
- the system calls addName with name = data.name; names = data.names into data.names
- the system computes output.total as output.total + data.amount
- the system computes data.index as data.index + 1
- end while
- the system sets JSON value at path data.totalPath in output.document to output.total into output.document
- the system sets JSON value at path data.countPath in output.document to output.count into output.document
- the system sets JSON value at path data.namesPath in output.document to data.names into output.document
- the system formats output.document as JSON text into data.report
- the system writes text data.report to file resource report
- the system returns output.count
- the system returns output.total
- the system records a trace event named Transformed
