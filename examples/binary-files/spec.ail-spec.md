The application BinaryFiles manages artifacts.

A Data has:
- body: Bytes
- reportText: Text

An Output has:
- report: Json

Function: readU32BE.
The function needs:
- body: Bytes
- offset: Int
The function produces:
- result: Int
The function stores:
- a: Int
- b: Int
- c: Int
- d: Int
The function executes:
- the function gets byte local.offset from bytes local.body into local.a
- the function gets byte local.offset + 1 from bytes local.body into local.b
- the function gets byte local.offset + 2 from bytes local.body into local.c
- the function gets byte local.offset + 3 from bytes local.body into local.d
- the function returns local.a * 16777216 + local.b * 65536 + local.c * 256 + local.d

Function: inspectHeader.
The function needs:
- body: Bytes
The function produces:
- result: Json
The function stores:
- result: Json
- count: Int
- width: Int
- height: Int
- depth: Int
- color: Int
- length: Int
- comparison: Int
- prefix: Bytes
- expected: Bytes
- recognized: Bool
- path: Text
The function executes:
- the function assigns {"bytes":0,"headerRecognized":false,"width":0,"height":0,"bitDepth":0,"colorType":0} to local.result
- the function counts bytes in local.body into local.count
- the function assigns "/bytes" to local.path
- the function sets JSON value at path local.path in local.result to local.count into local.result
- if local.count < 33:
- the function returns local.result
- end if
- the function slices bytes local.body from 0 to 8 into local.prefix
- the function assigns "hex:89504e470d0a1a0a" to local.expected
- the function compares bytes local.prefix with local.expected into local.comparison
- if local.comparison != 0:
- the function returns local.result
- end if
- the function slices bytes local.body from 12 to 16 into local.prefix
- the function assigns "hex:49484452" to local.expected
- the function compares bytes local.prefix with local.expected into local.comparison
- if local.comparison != 0:
- the function returns local.result
- end if
- the function calls readU32BE with body = local.body; offset = 8 into local.length
- if local.length != 13:
- the function returns local.result
- end if
- the function calls readU32BE with body = local.body; offset = 16 into local.width
- the function calls readU32BE with body = local.body; offset = 20 into local.height
- if local.width < 1:
- the function returns local.result
- end if
- if local.height < 1:
- the function returns local.result
- end if
- if local.width > 2147483647:
- the function returns local.result
- end if
- if local.height > 2147483647:
- the function returns local.result
- end if
- the function gets byte 24 from bytes local.body into local.depth
- the function gets byte 25 from bytes local.body into local.color
- the function assigns true to local.recognized
- the function assigns "/headerRecognized" to local.path
- the function sets JSON value at path local.path in local.result to local.recognized into local.result
- the function assigns "/width" to local.path
- the function sets JSON value at path local.path in local.result to local.width into local.result
- the function assigns "/height" to local.path
- the function sets JSON value at path local.path in local.result to local.height into local.result
- the function assigns "/bitDepth" to local.path
- the function sets JSON value at path local.path in local.result to local.depth into local.result
- the function assigns "/colorType" to local.path
- the function sets JSON value at path local.path in local.result to local.color into local.result
- the function returns local.result

Action: Inspect.
When Inspect happens:
- the system reads bytes from file resource input into data.body
- the system calls inspectHeader with body = data.body into output.report
- the system writes bytes data.body to file resource copy
- the system formats output.report as JSON text into data.reportText
- the system writes text data.reportText to file resource report
- the system returns output.report
- the system records a trace event named Inspected
