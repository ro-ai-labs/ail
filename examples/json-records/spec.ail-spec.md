The application JsonRecords manages records.

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
- index: Int
- name: Text
- amount: Int
- namesJson: Text
- totalJson: Text

An Output has:
- count: Int
- names: List<Text>
- total: Int

Action: Process.
When Process happens:
- the system reads text from file resource input into data.raw
- the system parses JSON text data.raw into data.document
- the system assigns "/records" to data.path
- the system gets JSON value at path data.path from data.document into data.records
- the system counts entries in JSON data.records into output.count
- the system assigns [] to output.names
- the system assigns 0 to output.total
- the system assigns 0 to data.index
- the system assigns "/" to data.slash
- the system assigns "/name" to data.namePath
- the system assigns "/metrics/amount" to data.amountPath
- while data.index < output.count:
- the system formats data.index as JSON text into data.indexText
- the system joins text data.slash and data.indexText into data.path
- the system gets JSON value at path data.path from data.records into data.record
- the system gets JSON value at path data.namePath from data.record into data.name
- the system gets JSON value at path data.amountPath from data.record into data.amount
- the system appends text data.name to list output.names into output.names
- the system computes output.total as output.total + data.amount
- the system computes data.index as data.index + 1
- end while
- the system formats output.names as JSON text into data.namesJson
- the system formats output.total as JSON text into data.totalJson
- the system writes text data.namesJson to file resource names
- the system writes text data.totalJson to file resource total
- the system returns output.count
- the system returns output.total
- the system records a trace event named Processed
