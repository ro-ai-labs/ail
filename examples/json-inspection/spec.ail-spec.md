The application JsonInspection manages arbitrary data.

A Data has:
- raw: Text
- document: Json
- path: Text
- report: Text

An Output has:
- kind: Text
- leaves: Int
- document: Json

Function: pointerToken.
The function needs:
- key: Text
The function produces:
- result: Text
The function stores:
- result: Text
- character: Text
- slashEscape: Text
- tildeEscape: Text
- count: Int
- index: Int
The function executes:
- the function assigns "" to local.result
- the function assigns "~1" to local.slashEscape
- the function assigns "~0" to local.tildeEscape
- the function counts characters in local.key into local.count
- the function assigns 0 to local.index
- while local.index < local.count:
- the function slices text local.key from local.index to local.index + 1 into local.character
- if boolean text local.character == "/":
- the function joins text local.result and local.slashEscape into local.result
- otherwise:
- if boolean text local.character == "~":
- the function joins text local.result and local.tildeEscape into local.result
- otherwise:
- the function joins text local.result and local.character into local.result
- end if
- end if
- the function computes local.index as local.index + 1
- end while
- the function returns local.result

Function: leafCount.
The function needs:
- value: Json
The function produces:
- result: Int
The function stores:
- kind: Text
- count: Int
- index: Int
- subtotal: Int
- result: Int
- keys: List<Text>
- key: Text
- token: Text
- slash: Text
- path: Text
- child: Json
The function executes:
- the function gets JSON kind from local.value into local.kind
- if boolean text local.kind != "object" and text local.kind != "array":
- the function returns 1
- end if
- the function counts entries in JSON local.value into local.count
- if boolean text local.kind == "object":
- the function gets JSON keys from local.value into local.keys
- end if
- the function assigns "/" to local.slash
- the function assigns 0 to local.index
- the function assigns 0 to local.result
- while local.index < local.count:
- if boolean text local.kind == "object":
- the function gets item local.index from list local.keys into local.key
- the function calls pointerToken with key = local.key into local.token
- otherwise:
- the function formats local.index as JSON text into local.token
- end if
- the function joins text local.slash and local.token into local.path
- the function gets JSON value at path local.path from local.value into local.child
- the function calls leafCount with value = local.child into local.subtotal
- the function computes local.result as local.result + local.subtotal
- the function computes local.index as local.index + 1
- end while
- the function returns local.result

Action: Inspect.
When Inspect happens:
- the system reads text from file resource input into data.raw
- the system parses JSON text data.raw into data.document
- the system gets JSON kind from data.document into output.kind
- the system calls leafCount with value = data.document into output.leaves
- the system assigns {} to output.document
- the system assigns "/kind" to data.path
- the system sets JSON value at path data.path in output.document to output.kind into output.document
- the system assigns "/leaf_count" to data.path
- the system sets JSON value at path data.path in output.document to output.leaves into output.document
- the system assigns "/source" to data.path
- the system sets JSON value at path data.path in output.document to data.document into output.document
- the system formats output.document as JSON text into data.report
- the system writes text data.report to file resource report
- the system returns output.kind
- the system returns output.leaves
- the system records a trace event named Inspected
