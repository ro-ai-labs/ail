The application JsonTransform manages reports.

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
- privatePath: Text
- processedPath: Text
- appendPath: Text
- totalPath: Text
- countPath: Text
- namesPath: Text
- index: Int
- name: Text
- amount: Int
- processed: Bool
- names: List<Text>
- report: Text

An Output has:
- document: Json
- count: Int
- total: Int

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
- the system assigns true to data.processed
- the system assigns "/" to data.slash
- the system assigns "/name" to data.namePath
- the system assigns "/metrics/amount" to data.amountPath
- the system assigns "/private" to data.privatePath
- the system assigns "/processed" to data.processedPath
- the system assigns "/records/-" to data.appendPath
- the system assigns "/total" to data.totalPath
- the system assigns "/count" to data.countPath
- the system assigns "/names" to data.namesPath
- while data.index < output.count:
- the system formats data.index as JSON text into data.indexText
- the system joins text data.slash and data.indexText into data.path
- the system gets JSON value at path data.path from data.records into data.record
- the system removes JSON value at path data.privatePath from data.record into data.record
- the system gets JSON value at path data.namePath from data.record into data.name
- the system gets JSON value at path data.amountPath from data.record into data.amount
- the system computes data.amount as data.amount * 2
- the system sets JSON value at path data.amountPath in data.record to data.amount into data.record
- the system sets JSON value at path data.processedPath in data.record to data.processed into data.record
- the system sets JSON value at path data.appendPath in output.document to data.record into output.document
- the system appends text data.name to list data.names into data.names
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
