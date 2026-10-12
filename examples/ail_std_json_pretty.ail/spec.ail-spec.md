The application JsonPrettyLibrary manages lossless JSON layout.

Function: quotedEnd.
The function needs:
- text: Text
- start: Int
- length: Int
The function produces:
- result: Int
The function stores:
- index: Int
- found: Int
- position: Int
- back: Int
- slashes: Int
- window: Text
- quote: Text
- character: Text
The function executes:
- the function assigns "\"" to local.quote
- the function computes local.index as local.start + 1
- while local.index < local.length:
- the function slices text local.text from local.index to local.index + 256 into local.window
- the function finds text local.quote in local.window into local.found
- if local.found < 0:
- the function computes local.index as local.index + 256
- continue
- end if
- the function computes local.position as local.index + local.found
- the function computes local.back as local.position - 1
- the function assigns 0 to local.slashes
- while local.back > local.start:
- the function slices text local.text from local.back to local.back + 1 into local.character
- if boolean text local.character != "\\":
- break
- end if
- the function computes local.slashes as local.slashes + 1
- the function computes local.back as local.back - 1
- end while
- if local.slashes % 2 == 0:
- the function returns local.position + 1
- end if
- the function computes local.index as local.position + 1
- end while
- the function raises error JSON
- the function returns local.length

Function: scalarEnd.
The function needs:
- text: Text
- start: Int
- length: Int
The function produces:
- result: Int
The function stores:
- index: Int
- found: Int
- first: Int
- window: Text
- comma: Text
- arrayClose: Text
- objectClose: Text
The function executes:
- the function assigns "," to local.comma
- the function assigns "]" to local.arrayClose
- the function assigns "}" to local.objectClose
- the function computes local.index as local.start
- while local.index < local.length:
- the function slices text local.text from local.index to local.index + 256 into local.window
- the function assigns 256 to local.first
- the function finds text local.comma in local.window into local.found
- if boolean integer local.found >= 0 and integer local.found < local.first:
- the function computes local.first as local.found
- end if
- the function finds text local.arrayClose in local.window into local.found
- if boolean integer local.found >= 0 and integer local.found < local.first:
- the function computes local.first as local.found
- end if
- the function finds text local.objectClose in local.window into local.found
- if boolean integer local.found >= 0 and integer local.found < local.first:
- the function computes local.first as local.found
- end if
- if local.first < 256:
- the function returns local.index + local.first
- end if
- the function computes local.index as local.index + 256
- end while
- the function returns local.length

Function: render.
The function needs:
- value: Json
The function produces:
- result: Text
The function stores:
- compact: Text
- result: Text
- character: Text
- next: Text
- token: Text
- indent: Text
- spaces: Text
- newline: Text
- colon: Text
- length: Int
- index: Int
- end: Int
- indentLength: Int
The function executes:
- the function formats local.value as JSON text into local.compact
- the function counts characters in local.compact into local.length
- the function assigns "" to local.result
- the function assigns "" to local.indent
- the function assigns "  " to local.spaces
- the function assigns "\n" to local.newline
- the function assigns ": " to local.colon
- the function assigns 0 to local.index
- while local.index < local.length:
- the function slices text local.compact from local.index to local.index + 1 into local.character
- if boolean text local.character == "{" or text local.character == "[":
- the function slices text local.compact from local.index + 1 to local.index + 2 into local.next
- if boolean text local.next == "}" or text local.next == "]":
- the function slices text local.compact from local.index to local.index + 2 into local.token
- the function joins text local.result and local.token into local.result
- the function computes local.index as local.index + 2
- continue
- end if
- the function joins text local.result and local.character into local.result
- the function joins text local.indent and local.spaces into local.indent
- the function joins text local.result and local.newline into local.result
- the function joins text local.result and local.indent into local.result
- otherwise:
- if boolean text local.character == "}" or text local.character == "]":
- the function counts characters in local.indent into local.indentLength
- the function slices text local.indent from 0 to local.indentLength - 2 into local.indent
- the function joins text local.result and local.newline into local.result
- the function joins text local.result and local.indent into local.result
- the function joins text local.result and local.character into local.result
- otherwise:
- if boolean text local.character == ",":
- the function joins text local.result and local.character into local.result
- the function joins text local.result and local.newline into local.result
- the function joins text local.result and local.indent into local.result
- otherwise:
- if boolean text local.character == ":":
- the function joins text local.result and local.colon into local.result
- otherwise:
- if boolean text local.character == "\"":
- the function calls quotedEnd with text = local.compact; start = local.index; length = local.length into local.end
- otherwise:
- the function calls scalarEnd with text = local.compact; start = local.index; length = local.length into local.end
- end if
- the function slices text local.compact from local.index to local.end into local.token
- the function joins text local.result and local.token into local.result
- the function computes local.index as local.end
- continue
- end if
- end if
- end if
- end if
- the function computes local.index as local.index + 1
- end while
- the function returns local.result

Function: renderText.
The function needs:
- text: Text
The function produces:
- result: Text
The function stores:
- value: Json
- result: Text
The function executes:
- the function parses JSON text local.text into local.value
- the function calls render with value = local.value into local.result
- the function returns local.result

Function: renderRequest.
The function needs:
- text: Text
The function produces:
- result: Json
The function stores:
- request: Json
- result: Json
- selected: Text
- path: Text
- kind: Text
- error: Text
- pretty: Text
The function executes:
- try:
- the function parses JSON text local.text into local.request
- the function gets JSON kind from local.request into local.kind
- if boolean text local.kind != "object":
- the function raises error JSON
- end if
- the function assigns "/json" to local.path
- the function gets JSON value at path local.path from local.request into local.selected
- recover JSON or JSON-PATH into local.error:
- the function assigns {"ok":false,"text":"","error":"REQUEST"} to local.result
- the function returns local.result
- end try
- try:
- the function calls renderText with text = local.selected into local.pretty
- recover JSON into local.error:
- the function assigns {"ok":false,"text":"","error":"JSON"} to local.result
- the function returns local.result
- end try
- the function assigns {"ok":true,"text":"","error":""} to local.result
- the function assigns "/text" to local.path
- the function sets JSON value at path local.path in local.result to local.pretty into local.result
- the function returns local.result

