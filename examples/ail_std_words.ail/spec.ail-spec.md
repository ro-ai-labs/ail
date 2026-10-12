The application WordsLibrary manages exact Unicode whitespace word frequencies.

Function: tokens.
The function needs:
- text: Text
The function produces:
- result: List<Text>
The function stores:
- normalized: Text
- separator: Text
- delimiters: List<Text>
- needle: Text
- parts: List<Text>
- result: List<Text>
- word: Text
- index: Int
- count: Int
- found: Int
The function executes:
- the function trims text local.text into local.normalized
- the function assigns [] to local.result
- if boolean text local.normalized == "":
- the function returns local.result
- end if
- the function assigns " " to local.separator
- the function assigns ["\t","\n","\u000b","\f","\r","\u001c","\u001d","\u001e","\u001f","\u0085","\u00a0","\u1680","\u2000","\u2001","\u2002","\u2003","\u2004","\u2005","\u2006","\u2007","\u2008","\u2009","\u200a","\u2028","\u2029","\u202f","\u205f","\u3000"] to local.delimiters
- the function counts items in list local.delimiters into local.count
- the function assigns 0 to local.index
- while local.index < local.count:
- the function gets item local.index from list local.delimiters into local.needle
- the function finds text local.needle in local.normalized into local.found
- if local.found >= 0:
- the function replaces text local.needle in local.normalized with local.separator into local.normalized
- end if
- the function computes local.index as local.index + 1
- end while
- the function splits text local.normalized by local.separator into local.parts
- the function counts items in list local.parts into local.count
- the function assigns 0 to local.index
- while local.index < local.count:
- the function gets item local.index from list local.parts into local.word
- if boolean text local.word != "":
- the function appends text local.word to list local.result into local.result
- end if
- the function computes local.index as local.index + 1
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

Function: frequencies.
The function needs:
- words: List<Text>
The function produces:
- result: Json
The function stores:
- result: Json
- word: Text
- path: Text
- error: Text
- index: Int
- size: Int
- count: Int
The function executes:
- the function assigns {} to local.result
- the function counts items in list local.words into local.size
- the function assigns 0 to local.index
- while local.index < local.size:
- the function gets item local.index from list local.words into local.word
- the function calls pointer with key = local.word into local.path
- try:
- the function gets JSON value at path local.path from local.result into local.count
- recover JSON-PATH into local.error:
- the function assigns 0 to local.count
- end try
- the function computes local.count as local.count + 1
- the function sets JSON value at path local.path in local.result to local.count into local.result
- the function computes local.index as local.index + 1
- end while
- the function returns local.result

Function: rank.
The function needs:
- counts: Json
The function produces:
- result: Json
The function stores:
- remaining: Json
- keys: List<Text>
- word: Text
- bestWord: Text
- path: Text
- index: Int
- size: Int
- count: Int
- best: Int
- round: Int
- entry: Json
- result: Json
The function executes:
- the function assigns [] to local.result
- the function assigns "" to local.path
- the function gets JSON value at path local.path from local.counts into local.remaining
- the function assigns 0 to local.round
- while local.round < 5:
- the function gets JSON keys from local.remaining into local.keys
- the function sorts list local.keys in ascending order into local.keys
- the function counts items in list local.keys into local.size
- if local.size == 0:
- break
- end if
- the function assigns 0 to local.best
- the function assigns 0 to local.index
- while local.index < local.size:
- the function gets item local.index from list local.keys into local.word
- the function calls pointer with key = local.word into local.path
- the function gets JSON value at path local.path from local.remaining into local.count
- if local.count > local.best:
- the function computes local.best as local.count
- the function gets item local.index from list local.keys into local.bestWord
- end if
- the function computes local.index as local.index + 1
- end while
- the function assigns {"word":"","count":0} to local.entry
- the function assigns "/word" to local.path
- the function sets JSON value at path local.path in local.entry to local.bestWord into local.entry
- the function assigns "/count" to local.path
- the function sets JSON value at path local.path in local.entry to local.best into local.entry
- the function assigns "/-" to local.path
- the function sets JSON value at path local.path in local.result to local.entry into local.result
- the function calls pointer with key = local.bestWord into local.path
- the function removes JSON value at path local.path from local.remaining into local.remaining
- the function computes local.round as local.round + 1
- end while
- the function returns local.result

Function: summarize.
The function needs:
- text: Text
The function produces:
- result: Json
The function stores:
- words: List<Text>
- counts: Json
- top: Json
- result: Json
- path: Text
- total: Int
- unique: Int
The function executes:
- the function calls tokens with text = local.text into local.words
- the function counts items in list local.words into local.total
- the function calls frequencies with words = local.words into local.counts
- the function counts entries in JSON local.counts into local.unique
- the function calls rank with counts = local.counts into local.top
- the function assigns {"ok":true,"words":0,"unique":0,"top":[],"error":""} to local.result
- the function assigns "/words" to local.path
- the function sets JSON value at path local.path in local.result to local.total into local.result
- the function assigns "/unique" to local.path
- the function sets JSON value at path local.path in local.result to local.unique into local.result
- the function assigns "/top" to local.path
- the function sets JSON value at path local.path in local.result to local.top into local.result
- the function returns local.result

Function: summarizeRequest.
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
The function executes:
- try:
- the function parses JSON text local.text into local.request
- the function gets JSON kind from local.request into local.kind
- if boolean text local.kind != "object":
- the function raises error JSON
- end if
- the function assigns "/text" to local.path
- the function gets JSON value at path local.path from local.request into local.selected
- recover JSON or JSON-PATH into local.error:
- the function assigns {"ok":false,"words":0,"unique":0,"top":[],"error":"REQUEST"} to local.result
- the function returns local.result
- end try
- the function calls summarize with text = local.selected into local.result
- the function returns local.result

