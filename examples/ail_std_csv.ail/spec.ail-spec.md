The application CsvLibrary manages explicit CSV parsing and signed category totals.

Function: delimiter.
The function needs:
- text: Text
- start: Int
- quoted: Bool
The function produces:
- result: Int
The function stores:
- length: Int
- offset: Int
- found: Int
- best: Int
- window: Text
- needle: Text
The function executes:
- the function counts characters in local.text into local.length
- the function computes local.offset as local.start
- while local.offset < local.length:
- the function slices text local.text from local.offset to local.offset + 512 into local.window
- the function assigns 512 to local.best
- the function assigns "\"" to local.needle
- the function finds text local.needle in local.window into local.found
- if boolean integer local.found >= 0:
- the function computes local.best as local.found
- end if
- if boolean not local.quoted:
- the function assigns "," to local.needle
- the function finds text local.needle in local.window into local.found
- if boolean integer local.found >= 0 and integer local.found < local.best:
- the function computes local.best as local.found
- end if
- the function assigns "\r" to local.needle
- the function finds text local.needle in local.window into local.found
- if boolean integer local.found >= 0 and integer local.found < local.best:
- the function computes local.best as local.found
- end if
- the function assigns "\n" to local.needle
- the function finds text local.needle in local.window into local.found
- if boolean integer local.found >= 0 and integer local.found < local.best:
- the function computes local.best as local.found
- end if
- end if
- if local.best < 512:
- the function returns local.offset + local.best
- end if
- the function computes local.offset as local.offset + 512
- end while
- the function returns local.length

Function: field.
The function needs:
- text: Text
- start: Int
The function produces:
- result: Json
The function stores:
- length: Int
- cursor: Int
- finish: Int
- next: Int
- character: Text
- value: Text
- pair: Text
- quote: Text
- separator: Text
- path: Text
- result: Json
- quoted: Bool
The function executes:
- the function counts characters in local.text into local.length
- the function slices text local.text from local.start to local.start + 1 into local.character
- the function assigns false to local.quoted
- if boolean text local.character == "\"":
- the function assigns true to local.quoted
- the function computes local.cursor as local.start + 1
- while local.cursor <= local.length:
- the function calls delimiter with text = local.text; start = local.cursor; quoted = local.quoted into local.finish
- if local.finish == local.length:
- the function raises error TEXT
- end if
- the function slices text local.text from local.finish + 1 to local.finish + 2 into local.character
- if boolean text local.character == "\"":
- the function computes local.cursor as local.finish + 2
- otherwise:
- break
- end if
- end while
- the function slices text local.text from local.start + 1 to local.finish into local.value
- the function assigns "\"\"" to local.pair
- the function assigns "\"" to local.quote
- the function replaces text local.pair in local.value with local.quote into local.value
- the function computes local.next as local.finish + 1
- otherwise:
- the function calls delimiter with text = local.text; start = local.start; quoted = local.quoted into local.finish
- the function slices text local.text from local.finish to local.finish + 1 into local.character
- if boolean text local.character == "\"":
- the function raises error TEXT
- end if
- the function slices text local.text from local.start to local.finish into local.value
- the function computes local.next as local.finish
- end if
- the function slices text local.text from local.next to local.next + 1 into local.separator
- if boolean text local.separator != "" and text local.separator != "," and text local.separator != "\r" and text local.separator != "\n":
- the function raises error TEXT
- end if
- if boolean text local.separator != "":
- the function computes local.next as local.next + 1
- if boolean text local.separator == "\r":
- the function slices text local.text from local.next to local.next + 1 into local.character
- if boolean text local.character == "\n":
- the function computes local.next as local.next + 1
- end if
- end if
- end if
- the function assigns {"value":"","next":0,"separator":""} to local.result
- the function assigns "/value" to local.path
- the function sets JSON value at path local.path in local.result to local.value into local.result
- the function assigns "/next" to local.path
- the function sets JSON value at path local.path in local.result to local.next into local.result
- the function assigns "/separator" to local.path
- the function sets JSON value at path local.path in local.result to local.separator into local.result
- the function returns local.result

Function: record.
The function needs:
- text: Text
- start: Int
The function produces:
- result: Json
The function stores:
- cursor: Int
- row: Json
- field: Json
- result: Json
- path: Text
- value: Text
- separator: Text
- first: Bool
- character: Text
The function executes:
- the function assigns [] to local.row
- the function computes local.cursor as local.start
- the function slices text local.text from local.start to local.start + 1 into local.character
- the function assigns true to local.first
- while boolean local.first or text local.separator == ",":
- the function calls field with text = local.text; start = local.cursor into local.field
- the function assigns "/value" to local.path
- the function gets JSON value at path local.path from local.field into local.value
- the function assigns "/next" to local.path
- the function gets JSON value at path local.path from local.field into local.cursor
- the function assigns "/separator" to local.path
- the function gets JSON value at path local.path from local.field into local.separator
- if boolean not local.first or (text local.character != "\r" and text local.character != "\n"):
- the function assigns "/-" to local.path
- the function sets JSON value at path local.path in local.row to local.value into local.row
- end if
- the function assigns false to local.first
- end while
- the function assigns {"row":[],"next":0} to local.result
- the function assigns "/row" to local.path
- the function sets JSON value at path local.path in local.result to local.row into local.result
- the function assigns "/next" to local.path
- the function sets JSON value at path local.path in local.result to local.cursor into local.result
- the function returns local.result

Function: parse.
The function needs:
- text: Text
The function produces:
- result: Json
The function stores:
- length: Int
- cursor: Int
- record: Json
- row: Json
- result: Json
- path: Text
The function executes:
- the function counts characters in local.text into local.length
- the function assigns 0 to local.cursor
- the function assigns [] to local.result
- while local.cursor < local.length:
- the function calls record with text = local.text; start = local.cursor into local.record
- the function assigns "/row" to local.path
- the function gets JSON value at path local.path from local.record into local.row
- the function assigns "/next" to local.path
- the function gets JSON value at path local.path from local.record into local.cursor
- the function assigns "/-" to local.path
- the function sets JSON value at path local.path in local.result to local.row into local.result
- end while
- the function returns local.result

Function: pointer.
The function needs:
- key: Text
The function produces:
- result: Text
The function stores:
- needle: Text
- replacement: Text
- result: Text
- slash: Text
The function executes:
- the function assigns "~" to local.needle
- the function assigns "~0" to local.replacement
- the function replaces text local.needle in local.key with local.replacement into local.result
- the function assigns "/" to local.needle
- the function assigns "~1" to local.replacement
- the function replaces text local.needle in local.result with local.replacement into local.result
- the function assigns "/" to local.slash
- the function joins text local.slash and local.result into local.result
- the function returns local.result

Function: item.
The function needs:
- array: Json
- index: Int
The function produces:
- result: Json
The function stores:
- token: Text
- slash: Text
- path: Text
- result: Json
The function executes:
- the function formats local.index as JSON text into local.token
- the function assigns "/" to local.slash
- the function joins text local.slash and local.token into local.path
- the function gets JSON value at path local.path from local.array into local.result
- the function returns local.result

Function: textItem.
The function needs:
- array: Json
- index: Int
The function produces:
- result: Text
The function stores:
- token: Text
- slash: Text
- path: Text
- result: Text
The function executes:
- the function formats local.index as JSON text into local.token
- the function assigns "/" to local.slash
- the function joins text local.slash and local.token into local.path
- the function gets JSON value at path local.path from local.array into local.result
- the function returns local.result

Function: failure.
The function needs:
- code: Text
The function produces:
- result: Json
The function stores:
- result: Json
- path: Text
The function executes:
- the function assigns {"ok":false,"rows":0,"total":0,"groups":[],"error":""} to local.result
- the function assigns "/error" to local.path
- the function sets JSON value at path local.path in local.result to local.code into local.result
- the function returns local.result

Function: header.
The function needs:
- row: Json
The function produces:
- result: Json
The function stores:
- width: Int
- index: Int
- category: Int
- amount: Int
- name: Text
- path: Text
- error: Text
- present: Bool
- seen: Json
- result: Json
The function executes:
- the function counts entries in JSON local.row into local.width
- the function assigns -1 to local.category
- the function assigns -1 to local.amount
- the function assigns 0 to local.index
- the function assigns {} to local.seen
- while local.index < local.width:
- the function calls textItem with array = local.row; index = local.index into local.name
- if boolean text local.name == "":
- the function raises error VALUE
- end if
- the function calls pointer with key = local.name into local.path
- the function assigns false to local.present
- try:
- the function gets JSON value at path local.path from local.seen into local.present
- recover JSON-PATH into local.error:
- the function assigns false to local.present
- end try
- if boolean local.present:
- the function raises error VALUE
- end if
- the function assigns true to local.present
- the function sets JSON value at path local.path in local.seen to local.present into local.seen
- if boolean text local.name == "category":
- the function computes local.category as local.index
- end if
- if boolean text local.name == "amount":
- the function computes local.amount as local.index
- end if
- the function computes local.index as local.index + 1
- end while
- if boolean integer local.category < 0 or integer local.amount < 0:
- the function raises error VALUE
- end if
- the function assigns {"category":0,"amount":0,"width":0} to local.result
- the function assigns "/category" to local.path
- the function sets JSON value at path local.path in local.result to local.category into local.result
- the function assigns "/amount" to local.path
- the function sets JSON value at path local.path in local.result to local.amount into local.result
- the function assigns "/width" to local.path
- the function sets JSON value at path local.path in local.result to local.width into local.result
- the function returns local.result

Function: amount.
The function needs:
- text: Text
The function produces:
- result: Int
The function stores:
- length: Int
- index: Int
- position: Int
- digits: Text
- character: Text
- value: Int
- error: Text
The function executes:
- the function counts characters in local.text into local.length
- if boolean integer local.length == 0 or integer local.length > 20:
- the function raises error TEXT
- end if
- the function assigns 0 to local.index
- the function slices text local.text from 0 to 1 into local.character
- if boolean text local.character == "-":
- the function assigns 1 to local.index
- end if
- if local.index == local.length:
- the function raises error TEXT
- end if
- the function slices text local.text from local.index to local.index + 1 into local.character
- if boolean text local.character == "0" and integer local.length > local.index + 1:
- the function raises error TEXT
- end if
- the function assigns "0123456789" to local.digits
- while local.index < local.length:
- the function slices text local.text from local.index to local.index + 1 into local.character
- the function finds text local.character in local.digits into local.position
- if local.position < 0:
- the function raises error TEXT
- end if
- the function computes local.index as local.index + 1
- end while
- try:
- the function parses JSON text local.text into local.value
- recover JSON into local.error:
- the function raises error TEXT
- end try
- the function returns local.value

Function: addRow.
The function needs:
- row: Json
- categoryIndex: Int
- amountIndex: Int
- groups: Json
- total: Int
The function produces:
- result: Json
The function stores:
- category: Text
- token: Text
- path: Text
- error: Text
- amount: Int
- subtotal: Int
- count: Int
- group: Json
- result: Json
The function executes:
- the function calls textItem with array = local.row; index = local.categoryIndex into local.category
- the function calls textItem with array = local.row; index = local.amountIndex into local.token
- the function calls amount with text = local.token into local.amount
- the function calls pointer with key = local.category into local.path
- try:
- the function gets JSON value at path local.path from local.groups into local.group
- recover JSON-PATH into local.error:
- the function assigns {"category":"","rows":0,"total":0} to local.group
- the function assigns "/category" to local.path
- the function sets JSON value at path local.path in local.group to local.category into local.group
- end try
- the function assigns "/total" to local.path
- the function gets JSON value at path local.path from local.group into local.subtotal
- the function computes local.subtotal as local.subtotal + local.amount
- the function computes local.total as local.total + local.amount
- the function sets JSON value at path local.path in local.group to local.subtotal into local.group
- the function assigns "/rows" to local.path
- the function gets JSON value at path local.path from local.group into local.count
- the function computes local.count as local.count + 1
- the function sets JSON value at path local.path in local.group to local.count into local.group
- the function calls pointer with key = local.category into local.path
- the function sets JSON value at path local.path in local.groups to local.group into local.groups
- the function assigns {"groups":{},"total":0} to local.result
- the function assigns "/groups" to local.path
- the function sets JSON value at path local.path in local.result to local.groups into local.result
- the function assigns "/total" to local.path
- the function sets JSON value at path local.path in local.result to local.total into local.result
- the function returns local.result

Function: sortedGroups.
The function needs:
- groups: Json
The function produces:
- result: Json
The function stores:
- keys: List<Text>
- key: Text
- path: Text
- count: Int
- index: Int
- group: Json
- result: Json
The function executes:
- the function gets JSON keys from local.groups into local.keys
- the function sorts list local.keys in ascending order into local.keys
- the function counts items in list local.keys into local.count
- the function assigns 0 to local.index
- the function assigns [] to local.result
- while local.index < local.count:
- the function gets item local.index from list local.keys into local.key
- the function calls pointer with key = local.key into local.path
- the function gets JSON value at path local.path from local.groups into local.group
- the function assigns "/-" to local.path
- the function sets JSON value at path local.path in local.result to local.group into local.result
- the function computes local.index as local.index + 1
- end while
- the function returns local.result

Function: summarize.
The function needs:
- text: Text
The function produces:
- result: Json
The function stores:
- rows: Json
- row: Json
- header: Json
- groups: Json
- updated: Json
- result: Json
- path: Text
- error: Text
- code: Text
- count: Int
- width: Int
- actualWidth: Int
- category: Int
- amount: Int
- index: Int
- total: Int
The function executes:
- try:
- the function calls parse with text = local.text into local.rows
- recover TEXT into local.error:
- the function assigns "CSV" to local.code
- the function calls failure with code = local.code into local.result
- the function returns local.result
- end try
- the function counts entries in JSON local.rows into local.count
- if local.count == 0:
- the function assigns "HEADER" to local.code
- the function calls failure with code = local.code into local.result
- the function returns local.result
- end if
- the function assigns 0 to local.index
- the function calls item with array = local.rows; index = local.index into local.row
- try:
- the function calls header with row = local.row into local.header
- recover VALUE into local.error:
- the function assigns "HEADER" to local.code
- the function calls failure with code = local.code into local.result
- the function returns local.result
- end try
- the function assigns "/width" to local.path
- the function gets JSON value at path local.path from local.header into local.width
- the function assigns "/category" to local.path
- the function gets JSON value at path local.path from local.header into local.category
- the function assigns "/amount" to local.path
- the function gets JSON value at path local.path from local.header into local.amount
- the function assigns 1 to local.index
- while local.index < local.count:
- the function calls item with array = local.rows; index = local.index into local.row
- the function counts entries in JSON local.row into local.actualWidth
- if local.actualWidth != local.width:
- the function assigns "ROW" to local.code
- the function calls failure with code = local.code into local.result
- the function returns local.result
- end if
- the function computes local.index as local.index + 1
- end while
- the function assigns {} to local.groups
- the function assigns 0 to local.total
- the function assigns 1 to local.index
- while local.index < local.count:
- the function calls item with array = local.rows; index = local.index into local.row
- try:
- the function calls addRow with row = local.row; categoryIndex = local.category; amountIndex = local.amount; groups = local.groups; total = local.total into local.updated
- recover TEXT or OVERFLOW into local.error:
- the function assigns "AMOUNT" to local.code
- if boolean text local.error == "AIL-RUNTIME-OVERFLOW":
- the function assigns "OVERFLOW" to local.code
- end if
- the function calls failure with code = local.code into local.result
- the function returns local.result
- end try
- the function assigns "/groups" to local.path
- the function gets JSON value at path local.path from local.updated into local.groups
- the function assigns "/total" to local.path
- the function gets JSON value at path local.path from local.updated into local.total
- the function computes local.index as local.index + 1
- end while
- the function calls sortedGroups with groups = local.groups into local.groups
- the function assigns {"ok":true,"rows":0,"total":0,"groups":[],"error":""} to local.result
- the function computes local.count as local.count - 1
- the function assigns "/rows" to local.path
- the function sets JSON value at path local.path in local.result to local.count into local.result
- the function assigns "/total" to local.path
- the function sets JSON value at path local.path in local.result to local.total into local.result
- the function assigns "/groups" to local.path
- the function sets JSON value at path local.path in local.result to local.groups into local.result
- the function returns local.result

Function: summarizeRequest.
The function needs:
- text: Text
The function produces:
- result: Json
The function stores:
- request: Json
- result: Json
- csv: Text
- path: Text
- kind: Text
- error: Text
- code: Text
The function executes:
- try:
- the function parses JSON text local.text into local.request
- the function gets JSON kind from local.request into local.kind
- if boolean text local.kind != "object":
- the function raises error JSON
- end if
- the function assigns "/csv" to local.path
- the function gets JSON value at path local.path from local.request into local.csv
- recover JSON or JSON-PATH into local.error:
- the function assigns "REQUEST" to local.code
- the function calls failure with code = local.code into local.result
- the function returns local.result
- end try
- the function calls summarize with text = local.csv into local.result
- the function returns local.result
