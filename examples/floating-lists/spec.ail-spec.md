The application FloatingLists manages numerical reports.

A Data has:
- raw: Text
- document: Json
- values: List<Float>
- centered: List<Float>
- path: Text
- index: Int
- item: Float
- denominator: Float
- report: Text

An Output has:
- count: Int
- mean: Float
- variance: Float

Function: mean.
The function needs:
- values: List<Float>
The function produces:
- result: Float
The function stores:
- count: Int
- index: Int
- item: Float
- total: Float
- denominator: Float
- result: Float
The function executes:
- the function counts items in list local.values into local.count
- the function assigns 0 to local.index
- the function assigns 0 to local.total
- while local.index < local.count:
- the function gets item local.index from list local.values into local.item
- the function computes floating local.total as local.total + local.item
- the function computes local.index as local.index + 1
- end while
- the function converts integer local.count to floating local.denominator
- the function computes floating local.result as local.total / local.denominator
- the function returns local.result

Function: center.
The function needs:
- values: List<Float>
The function produces:
- result: List<Float>
The function stores:
- count: Int
- index: Int
- item: Float
- mean: Float
- result: List<Float>
The function executes:
- the function calls mean with values = local.values into local.mean
- the function counts items in list local.values into local.count
- the function creates an empty list into local.result
- the function assigns 0 to local.index
- while local.index < local.count:
- the function gets item local.index from list local.values into local.item
- the function appends floating local.item - local.mean to list local.result into local.result
- the function computes local.index as local.index + 1
- end while
- the function returns local.result

Action: Analyze.
When Analyze happens:
- the system reads text from file resource input into data.raw
- the system parses JSON text data.raw into data.document
- the system assigns "/values" to data.path
- the system gets JSON value at path data.path from data.document into data.values
- the system counts items in list data.values into output.count
- the system calls mean with values = data.values into output.mean
- the system calls center with values = data.values into data.centered
- the system assigns 0 to output.variance
- the system assigns 0 to data.index
- while data.index < output.count:
- the system gets item data.index from list data.centered into data.item
- the system computes floating output.variance as output.variance + data.item * data.item
- the system computes data.index as data.index + 1
- end while
- the system converts integer output.count to floating data.denominator
- the system computes floating output.variance as output.variance / data.denominator
- the system assigns "/count" to data.path
- the system sets JSON value at path data.path in data.document to output.count into data.document
- the system assigns "/mean" to data.path
- the system sets JSON value at path data.path in data.document to output.mean into data.document
- the system assigns "/centered" to data.path
- the system sets JSON value at path data.path in data.document to data.centered into data.document
- the system assigns "/variance" to data.path
- the system sets JSON value at path data.path in data.document to output.variance into data.document
- the system formats data.document as JSON text into data.report
- the system writes text data.report to file resource report
- the system returns output.count
- the system returns output.mean
- the system returns output.variance
- the system records a trace event named Analyzed
