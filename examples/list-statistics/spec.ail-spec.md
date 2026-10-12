The application ListStatistics manages measurement batches.

A Data has:
- raw: Text
- document: Json
- path: Text
- report: Text
- first: List<Float>
- second: List<Float>
- merged: List<Float>
- sorted: List<Float>
- highest: List<Float>
- limit: Int
- median: Float

An Output has:
- count: Int
- document: Json

Function: mergeBatches.
The function needs:
- first: List<Float>
- second: List<Float>
The function produces:
- result: List<Float>
The function stores:
- result: List<Float>
The function executes:
- the function concatenates list local.first and list local.second into local.result
- the function returns local.result

Function: highestValues.
The function needs:
- sorted: List<Float>
- limit: Int
The function produces:
- result: List<Float>
The function stores:
- count: Int
- start: Int
- window: List<Float>
- result: List<Float>
The function executes:
- the function counts items in list local.sorted into local.count
- if local.limit < 0:
- the function assigns 0 to local.limit
- end if
- if local.limit > local.count:
- the function assigns 0 to local.start
- otherwise:
- the function computes local.start as local.count - local.limit
- end if
- the function slices list local.sorted from local.start to local.count into local.window
- the function reverses list local.window into local.result
- the function returns local.result

Function: sortedMedian.
The function needs:
- sorted: List<Float>
The function produces:
- result: Float
The function stores:
- count: Int
- middle: Int
- lower: Float
- upper: Float
- result: Float
The function executes:
- the function counts items in list local.sorted into local.count
- the function computes local.middle as local.count / 2
- the function gets item local.middle from list local.sorted into local.upper
- if local.count % 2 == 1:
- the function returns local.upper
- end if
- the function gets item local.middle - 1 from list local.sorted into local.lower
- the function computes floating local.result as (local.lower + local.upper) / 2
- the function returns local.result

Action: Analyze.
When Analyze happens:
- the system reads text from file resource input into data.raw
- the system parses JSON text data.raw into data.document
- the system assigns "/first" to data.path
- the system gets JSON value at path data.path from data.document into data.first
- the system assigns "/second" to data.path
- the system gets JSON value at path data.path from data.document into data.second
- the system assigns "/limit" to data.path
- the system gets JSON value at path data.path from data.document into data.limit
- the system calls mergeBatches with first = data.first; second = data.second into data.merged
- the system sorts list data.merged in ascending order into data.sorted
- the system counts items in list data.sorted into output.count
- the system calls highestValues with sorted = data.sorted; limit = data.limit into data.highest
- the system assigns {"median":null} to output.document
- if output.count > 0:
- the system calls sortedMedian with sorted = data.sorted into data.median
- the system assigns "/median" to data.path
- the system sets JSON value at path data.path in output.document to data.median into output.document
- end if
- the system assigns "/count" to data.path
- the system sets JSON value at path data.path in output.document to output.count into output.document
- the system assigns "/sorted" to data.path
- the system sets JSON value at path data.path in output.document to data.sorted into output.document
- the system assigns "/highest" to data.path
- the system sets JSON value at path data.path in output.document to data.highest into output.document
- the system formats output.document as JSON text into data.report
- the system writes text data.report to file resource report
- the system returns output.count
- the system records a trace event named Analyzed
