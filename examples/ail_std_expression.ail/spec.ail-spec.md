The application ExpressionLibrary manages checked integer expressions.

Function: classifyCharacter.
The function needs:
- character: Text
The function produces:
- result: Int
The function executes:
- if boolean text local.character == " " or text local.character == "\t" or text local.character == "\n" or text local.character == "\r" or text local.character == "\u000b" or text local.character == "\u000c":
- the function returns 0
- end if
- if boolean text local.character >= "0" and text local.character <= "9":
- the function returns 1
- end if
- if boolean (text local.character >= "A" and text local.character <= "Z") or (text local.character >= "a" and text local.character <= "z") or text local.character == "_":
- the function returns 2
- end if
- if boolean text local.character == "+" or text local.character == "-" or text local.character == "*" or text local.character == "/" or text local.character == "%" or text local.character == "(" or text local.character == ")":
- the function returns 3
- end if
- the function returns 4

Function: lex.
The function needs:
- expression: Text
The function produces:
- result: Json
The function stores:
- tokens: List<Text>
- token: Text
- nextCharacter: Text
- path: Text
- result: Json
- length: Int
- index: Int
- start: Int
- count: Int
- kind: Int
- nextKind: Int
The function executes:
- the function counts characters in local.expression into local.length
- if local.length > 4096:
- the function assigns {"error":"LIMIT","tokens":[]} to local.result
- the function returns local.result
- end if
- the function assigns [] to local.tokens
- the function assigns 0 to local.index
- the function assigns 0 to local.count
- while local.index < local.length:
- the function slices text local.expression from local.index to local.index + 1 into local.token
- the function calls classifyCharacter with character = local.token into local.kind
- if local.kind == 0:
- the function computes local.index as local.index + 1
- continue
- end if
- if local.kind == 4:
- the function assigns {"error":"LEX","tokens":[]} to local.result
- the function returns local.result
- end if
- the function computes local.start as local.index
- the function computes local.index as local.index + 1
- if boolean integer local.kind == 1 or integer local.kind == 2:
- while local.index < local.length:
- the function slices text local.expression from local.index to local.index + 1 into local.nextCharacter
- the function calls classifyCharacter with character = local.nextCharacter into local.nextKind
- if boolean integer local.nextKind == 1 or (integer local.kind == 2 and integer local.nextKind == 2):
- the function computes local.index as local.index + 1
- otherwise:
- break
- end if
- end while
- the function slices text local.expression from local.start to local.index into local.token
- end if
- if local.count == 512:
- the function assigns {"error":"LIMIT","tokens":[]} to local.result
- the function returns local.result
- end if
- the function appends text local.token to list local.tokens into local.tokens
- the function computes local.count as local.count + 1
- end while
- the function assigns {"error":"","tokens":[]} to local.result
- the function assigns "/tokens" to local.path
- the function sets JSON value at path local.path in local.result to local.tokens into local.result
- the function returns local.result

Function: precedence.
The function needs:
- operator: Text
The function produces:
- result: Int
The function executes:
- if boolean text local.operator == "u+" or text local.operator == "u-":
- the function returns 3
- end if
- if boolean text local.operator == "*" or text local.operator == "/" or text local.operator == "%":
- the function returns 2
- end if
- if boolean text local.operator == "+" or text local.operator == "-":
- the function returns 1
- end if
- the function returns 0

Function: postfix.
The function needs:
- tokens: List<Text>
The function produces:
- result: Json
The function stores:
- operators: List<Text>
- output: List<Text>
- token: Text
- first: Text
- top: Text
- path: Text
- result: Json
- expected: Bool
- matched: Bool
- count: Int
- index: Int
- depth: Int
- kind: Int
- priority: Int
- topPriority: Int
The function executes:
- the function assigns [] to local.operators
- the function assigns [] to local.output
- the function assigns true to local.expected
- the function assigns 0 to local.index
- the function counts items in list local.tokens into local.count
- while local.index < local.count:
- the function gets item local.index from list local.tokens into local.token
- the function computes local.index as local.index + 1
- if boolean text local.token == "(":
- if boolean not local.expected:
- the function assigns {"error":"SYNTAX","tokens":[]} to local.result
- the function returns local.result
- end if
- the function appends text local.token to list local.operators into local.operators
- continue
- end if
- if boolean text local.token == ")":
- if boolean local.expected:
- the function assigns {"error":"SYNTAX","tokens":[]} to local.result
- the function returns local.result
- end if
- the function assigns false to local.matched
- the function counts items in list local.operators into local.depth
- while local.depth > 0:
- the function gets item local.depth - 1 from list local.operators into local.top
- the function computes local.depth as local.depth - 1
- the function slices list local.operators from 0 to local.depth into local.operators
- if boolean text local.top == "(":
- the function assigns true to local.matched
- break
- end if
- the function appends text local.top to list local.output into local.output
- end while
- if boolean not local.matched:
- the function assigns {"error":"SYNTAX","tokens":[]} to local.result
- the function returns local.result
- end if
- continue
- end if
- the function slices text local.token from 0 to 1 into local.first
- the function calls classifyCharacter with character = local.first into local.kind
- if boolean integer local.kind == 1 or integer local.kind == 2:
- if boolean not local.expected:
- the function assigns {"error":"SYNTAX","tokens":[]} to local.result
- the function returns local.result
- end if
- the function appends text local.token to list local.output into local.output
- the function assigns false to local.expected
- continue
- end if
- if boolean local.expected:
- if boolean text local.token == "+":
- the function assigns "u+" to local.token
- otherwise:
- if boolean text local.token == "-":
- the function assigns "u-" to local.token
- otherwise:
- the function assigns {"error":"SYNTAX","tokens":[]} to local.result
- the function returns local.result
- end if
- end if
- end if
- the function calls precedence with operator = local.token into local.priority
- the function counts items in list local.operators into local.depth
- while local.depth > 0:
- the function gets item local.depth - 1 from list local.operators into local.top
- if boolean text local.top == "(":
- break
- end if
- the function calls precedence with operator = local.top into local.topPriority
- if boolean integer local.topPriority < local.priority or (integer local.topPriority == local.priority and local.expected):
- break
- end if
- the function appends text local.top to list local.output into local.output
- the function computes local.depth as local.depth - 1
- the function slices list local.operators from 0 to local.depth into local.operators
- end while
- the function appends text local.token to list local.operators into local.operators
- the function assigns true to local.expected
- end while
- if boolean local.expected:
- the function assigns {"error":"SYNTAX","tokens":[]} to local.result
- the function returns local.result
- end if
- the function counts items in list local.operators into local.depth
- while local.depth > 0:
- the function gets item local.depth - 1 from list local.operators into local.top
- if boolean text local.top == "(":
- the function assigns {"error":"SYNTAX","tokens":[]} to local.result
- the function returns local.result
- end if
- the function appends text local.top to list local.output into local.output
- the function computes local.depth as local.depth - 1
- end while
- the function assigns {"error":"","tokens":[]} to local.result
- the function assigns "/tokens" to local.path
- the function sets JSON value at path local.path in local.result to local.output into local.result
- the function returns local.result

Function: decimal.
The function needs:
- token: Text
The function produces:
- result: Int
The function stores:
- digits: Text
- digit: Text
- normalized: Text
- start: Int
- position: Int
- index: Int
- length: Int
- value: Int
The function executes:
- the function assigns "123456789" to local.digits
- the function assigns -1 to local.start
- the function assigns 0 to local.index
- while local.index < 9:
- the function slices text local.digits from local.index to local.index + 1 into local.digit
- the function finds text local.digit in local.token into local.position
- if boolean integer local.position >= 0 and (integer local.start == -1 or integer local.position < local.start):
- the function computes local.start as local.position
- end if
- the function computes local.index as local.index + 1
- end while
- if local.start == -1:
- the function returns 0
- end if
- the function counts characters in local.token into local.length
- the function slices text local.token from local.start to local.length into local.normalized
- the function parses JSON text local.normalized into local.value
- the function returns local.value

Function: applyBinary.
The function needs:
- left: Int
- right: Int
- operator: Text
The function produces:
- result: Int
The function executes:
- if boolean text local.operator == "+":
- the function returns local.left + local.right
- end if
- if boolean text local.operator == "-":
- the function returns local.left - local.right
- end if
- if boolean text local.operator == "*":
- the function returns local.left * local.right
- end if
- if boolean text local.operator == "/":
- the function returns local.left / local.right
- end if
- the function returns local.left % local.right

Function: result.
The function needs:
- value: Int
- error: Text
The function produces:
- result: Json
The function stores:
- result: Json
- path: Text
- ok: Bool
The function executes:
- the function assigns {"ok":false,"value":0,"error":""} to local.result
- if boolean text local.error == "":
- the function assigns true to local.ok
- the function assigns "/ok" to local.path
- the function sets JSON value at path local.path in local.result to local.ok into local.result
- the function assigns "/value" to local.path
- the function sets JSON value at path local.path in local.result to local.value into local.result
- otherwise:
- the function assigns "/error" to local.path
- the function sets JSON value at path local.path in local.result to local.error into local.result
- end if
- the function returns local.result

Function: evaluatePostfix.
The function needs:
- tokens: List<Text>
- variables: Json
The function produces:
- result: Json
The function stores:
- stack: List<Int>
- token: Text
- first: Text
- slash: Text
- path: Text
- error: Text
- result: Json
- count: Int
- index: Int
- depth: Int
- kind: Int
- left: Int
- right: Int
- value: Int
The function executes:
- the function assigns [] to local.stack
- the function assigns "/" to local.slash
- the function assigns "" to local.error
- the function assigns 0 to local.index
- the function counts items in list local.tokens into local.count
- try:
- while local.index < local.count:
- the function gets item local.index from list local.tokens into local.token
- the function computes local.index as local.index + 1
- if boolean text local.token == "u+" or text local.token == "u-":
- the function counts items in list local.stack into local.depth
- the function gets item local.depth - 1 from list local.stack into local.value
- if boolean text local.token == "u-":
- the function computes local.value as 0 - local.value
- end if
- the function sets item local.depth - 1 in list local.stack to integer local.value into local.stack
- continue
- end if
- the function slices text local.token from 0 to 1 into local.first
- the function calls classifyCharacter with character = local.first into local.kind
- if local.kind == 1:
- the function calls decimal with token = local.token into local.value
- otherwise:
- if local.kind == 2:
- the function joins text local.slash and local.token into local.path
- try:
- the function gets JSON value at path local.path from local.variables into local.value
- recover JSON or JSON-PATH into local.error:
- the function assigns {"ok":false,"value":0,"error":"NAME"} to local.result
- the function returns local.result
- end try
- otherwise:
- the function counts items in list local.stack into local.depth
- the function gets item local.depth - 2 from list local.stack into local.left
- the function gets item local.depth - 1 from list local.stack into local.right
- the function calls applyBinary with left = local.left; right = local.right; operator = local.token into local.value
- the function slices list local.stack from 0 to local.depth - 2 into local.stack
- end if
- end if
- the function appends integer local.value to list local.stack into local.stack
- end while
- the function gets item 0 from list local.stack into local.value
- recover JSON or OVERFLOW or DIVZERO into local.error:
- if boolean text local.error == "AIL-RUNTIME-DIVZERO":
- the function assigns {"ok":false,"value":0,"error":"DIVZERO"} to local.result
- otherwise:
- the function assigns {"ok":false,"value":0,"error":"OVERFLOW"} to local.result
- end if
- the function returns local.result
- end try
- the function calls result with value = local.value; error = local.error into local.result
- the function returns local.result

Function: evaluate.
The function needs:
- expression: Text
- variables: Json
The function produces:
- result: Json
The function stores:
- kind: Text
- error: Text
- errorPath: Text
- tokensPath: Text
- stage: Json
- result: Json
- tokens: List<Text>
The function executes:
- the function gets JSON kind from local.variables into local.kind
- if boolean text local.kind != "object":
- the function assigns {"ok":false,"value":0,"error":"REQUEST"} to local.result
- the function returns local.result
- end if
- the function assigns "/error" to local.errorPath
- the function assigns "/tokens" to local.tokensPath
- the function calls lex with expression = local.expression into local.stage
- the function gets JSON value at path local.errorPath from local.stage into local.error
- if boolean text local.error != "":
- the function calls result with value = 0; error = local.error into local.result
- the function returns local.result
- end if
- the function gets JSON value at path local.tokensPath from local.stage into local.tokens
- the function calls postfix with tokens = local.tokens into local.stage
- the function gets JSON value at path local.errorPath from local.stage into local.error
- if boolean text local.error != "":
- the function calls result with value = 0; error = local.error into local.result
- the function returns local.result
- end if
- the function gets JSON value at path local.tokensPath from local.stage into local.tokens
- the function calls evaluatePostfix with tokens = local.tokens; variables = local.variables into local.result
- the function returns local.result

Function: evaluateRequest.
The function needs:
- text: Text
The function produces:
- result: Json
The function stores:
- request: Json
- variables: Json
- result: Json
- expression: Text
- kind: Text
- path: Text
- error: Text
The function executes:
- the function assigns {"ok":false,"value":0,"error":"REQUEST"} to local.result
- try:
- the function parses JSON text local.text into local.request
- the function gets JSON kind from local.request into local.kind
- if boolean text local.kind != "object":
- the function returns local.result
- end if
- the function assigns "/expression" to local.path
- the function gets JSON value at path local.path from local.request into local.expression
- the function assigns "/variables" to local.path
- the function gets JSON value at path local.path from local.request into local.variables
- the function gets JSON kind from local.variables into local.kind
- if boolean text local.kind != "object":
- the function returns local.result
- end if
- recover JSON or JSON-PATH into local.error:
- the function returns local.result
- end try
- the function calls evaluate with expression = local.expression; variables = local.variables into local.result
- the function returns local.result
