The application FloatMeasurements manages temperature reports.

A Data has:
- raw: Text
- document: Json
- readings: Json
- path: Text
- indexText: Text
- slash: Text
- appendPath: Text
- index: Int
- celsius: Float
- fahrenheit: Float
- total: Float
- countFloat: Float
- report: Text

An Output has:
- document: Json
- count: Int
- belowFreezing: Int
- mean: Float
- wholeMean: Int

Function: fahrenheit.
The function needs:
- celsius: Float
The function produces:
- result: Float
The function stores:
- result: Float
The function executes:
- the function computes floating local.result as local.celsius * 9 / 5 + 32
- the function returns local.result

Action: Report.
When Report happens:
- the system reads text from file resource input into data.raw
- the system parses JSON text data.raw into data.document
- the system assigns "/celsius" to data.path
- the system gets JSON value at path data.path from data.document into data.readings
- the system counts entries in JSON data.readings into output.count
- the system assigns {"fahrenheit":[]} to output.document
- the system assigns 0 to data.index
- the system assigns 0 to output.belowFreezing
- the system assigns 0 to data.total
- the system assigns "/" to data.slash
- the system assigns "/fahrenheit/-" to data.appendPath
- while data.index < output.count:
- the system formats data.index as JSON text into data.indexText
- the system joins text data.slash and data.indexText into data.path
- the system gets JSON value at path data.path from data.readings into data.celsius
- the system calls fahrenheit with celsius = data.celsius into data.fahrenheit
- the system computes floating data.total as data.total + data.fahrenheit
- the system sets JSON value at path data.appendPath in output.document to data.fahrenheit into output.document
- if floating data.celsius < 0:
- the system computes output.belowFreezing as output.belowFreezing + 1
- end if
- the system computes data.index as data.index + 1
- end while
- the system converts integer output.count to floating data.countFloat
- the system computes floating output.mean as data.total / data.countFloat
- the system converts floating output.mean to integer output.wholeMean
- the system assigns "/count" to data.path
- the system sets JSON value at path data.path in output.document to output.count into output.document
- the system assigns "/belowFreezing" to data.path
- the system sets JSON value at path data.path in output.document to output.belowFreezing into output.document
- the system assigns "/mean" to data.path
- the system sets JSON value at path data.path in output.document to output.mean into output.document
- the system assigns "/wholeMean" to data.path
- the system sets JSON value at path data.path in output.document to output.wholeMean into output.document
- the system formats output.document as JSON text into data.report
- the system writes text data.report to file resource report
- the system returns output.count
- the system returns output.mean
- the system records a trace event named Reported
