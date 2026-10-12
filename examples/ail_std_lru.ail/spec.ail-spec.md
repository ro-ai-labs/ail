The application LruLibrary manages immutable least-recently-used cache states.

Function: pointer.
The function needs:
- key: Text
The function produces:
- result: Text
The function stores:
- result: Text
- needle: Text
- replacement: Text
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

Function: stateOf.
The function needs:
- capacity: Int
- entries: Json
The function produces:
- result: Json
The function stores:
- result: Json
- path: Text
The function executes:
- the function assigns {"capacity":0,"entries":{}} to local.result
- the function assigns "/capacity" to local.path
- the function sets JSON value at path local.path in local.result to local.capacity into local.result
- the function assigns "/entries" to local.path
- the function sets JSON value at path local.path in local.result to local.entries into local.result
- the function returns local.result

Function: create.
The function needs:
- capacity: Int
The function produces:
- result: Json
The function stores:
- entries: Json
- result: Json
The function executes:
- if local.capacity < 0:
- the function raises error VALUE
- end if
- the function assigns {} to local.entries
- the function calls stateOf with capacity = local.capacity; entries = local.entries into local.result
- the function returns local.result

Function: normalize.
The function needs:
- state: Json
The function produces:
- result: Json
The function stores:
- kind: Text
- path: Text
- error: Text
- capacity: Int
- count: Int
- index: Int
- entries: Json
- result: Json
- keys: List<Text>
- sorted: List<Text>
- previous: Text
- current: Text
The function executes:
- try:
- the function gets JSON kind from local.state into local.kind
- if boolean text local.kind != "object":
- the function raises error VALUE
- end if
- the function counts entries in JSON local.state into local.count
- if local.count != 2:
- the function raises error VALUE
- end if
- the function assigns "/capacity" to local.path
- the function gets JSON value at path local.path from local.state into local.capacity
- if local.capacity < 0:
- the function raises error VALUE
- end if
- the function assigns "/entries" to local.path
- the function gets JSON value at path local.path from local.state into local.entries
- the function gets JSON kind from local.entries into local.kind
- if boolean text local.kind != "object":
- the function raises error VALUE
- end if
- the function counts entries in JSON local.entries into local.count
- if local.count > local.capacity:
- the function raises error VALUE
- end if
- the function gets JSON keys from local.entries into local.keys
- the function sorts list local.keys in ascending order into local.sorted
- the function assigns 1 to local.index
- while local.index < local.count:
- the function gets item local.index - 1 from list local.sorted into local.previous
- the function gets item local.index from list local.sorted into local.current
- if boolean text local.previous == local.current:
- the function raises error VALUE
- end if
- the function computes local.index as local.index + 1
- end while
- recover JSON or JSON-PATH into local.error:
- the function raises error VALUE
- end try
- the function calls stateOf with capacity = local.capacity; entries = local.entries into local.result
- the function returns local.result

Function: lookup.
The function needs:
- state: Json
- key: Text
The function produces:
- result: Json
The function stores:
- entries: Json
- capacity: Int
- path: Text
- value: Json
- error: Text
- hit: Bool
- result: Json
- next: Json
The function executes:
- the function assigns "/capacity" to local.path
- the function gets JSON value at path local.path from local.state into local.capacity
- the function assigns "/entries" to local.path
- the function gets JSON value at path local.path from local.state into local.entries
- the function calls pointer with key = local.key into local.path
- the function assigns false to local.hit
- the function assigns null to local.value
- the function calls contains with entries = local.entries; key = local.key into local.hit
- if boolean local.hit:
- the function gets JSON value at path local.path from local.entries into local.value
- the function removes JSON value at path local.path from local.entries into local.entries
- the function sets JSON value at path local.path in local.entries to local.value into local.entries
- end if
- the function calls stateOf with capacity = local.capacity; entries = local.entries into local.next
- the function assigns {"state":null,"hit":false,"value":null} to local.result
- the function assigns "/state" to local.path
- the function sets JSON value at path local.path in local.result to local.next into local.result
- the function assigns "/hit" to local.path
- the function sets JSON value at path local.path in local.result to local.hit into local.result
- the function assigns "/value" to local.path
- the function sets JSON value at path local.path in local.result to local.value into local.result
- the function returns local.result

Function: get.
The function needs:
- state: Json
- key: Text
The function produces:
- result: Json
The function stores:
- normalized: Json
- result: Json
The function executes:
- the function calls normalize with state = local.state into local.normalized
- the function calls lookup with state = local.normalized; key = local.key into local.result
- the function returns local.result

Function: insert.
The function needs:
- state: Json
- key: Text
- value: Json
The function produces:
- result: Json
The function stores:
- entries: Json
- capacity: Int
- path: Text
- keyPath: Text
- oldValue: Json
- error: Text
- replaced: Bool
- count: Int
- keys: List<Text>
- oldKey: Text
- evicted: Json
- next: Json
- result: Json
The function executes:
- the function assigns "/capacity" to local.path
- the function gets JSON value at path local.path from local.state into local.capacity
- the function assigns "/entries" to local.path
- the function gets JSON value at path local.path from local.state into local.entries
- the function calls pointer with key = local.key into local.keyPath
- the function assigns false to local.replaced
- the function assigns null to local.evicted
- the function calls contains with entries = local.entries; key = local.key into local.replaced
- if local.capacity > 0:
- if boolean local.replaced:
- the function removes JSON value at path local.keyPath from local.entries into local.entries
- otherwise:
- the function counts entries in JSON local.entries into local.count
- if local.count == local.capacity:
- the function gets JSON keys from local.entries into local.keys
- the function gets item 0 from list local.keys into local.oldKey
- the function calls pointer with key = local.oldKey into local.path
- the function gets JSON value at path local.path from local.entries into local.oldValue
- the function removes JSON value at path local.path from local.entries into local.entries
- the function assigns {"key":"","value":null} to local.evicted
- the function assigns "/key" to local.path
- the function sets JSON value at path local.path in local.evicted to local.oldKey into local.evicted
- the function assigns "/value" to local.path
- the function sets JSON value at path local.path in local.evicted to local.oldValue into local.evicted
- end if
- end if
- the function sets JSON value at path local.keyPath in local.entries to local.value into local.entries
- end if
- the function calls stateOf with capacity = local.capacity; entries = local.entries into local.next
- the function assigns {"state":null,"replaced":false,"evicted":null} to local.result
- the function assigns "/state" to local.path
- the function sets JSON value at path local.path in local.result to local.next into local.result
- the function assigns "/replaced" to local.path
- the function sets JSON value at path local.path in local.result to local.replaced into local.result
- the function assigns "/evicted" to local.path
- the function sets JSON value at path local.path in local.result to local.evicted into local.result
- the function returns local.result

Function: put.
The function needs:
- state: Json
- key: Text
- value: Json
The function produces:
- result: Json
The function stores:
- normalized: Json
- result: Json
The function executes:
- the function calls normalize with state = local.state into local.normalized
- the function calls insert with state = local.normalized; key = local.key; value = local.value into local.result
- the function returns local.result

Function: validateOperations.
The function needs:
- operations: Json
The function produces:
- result: Int
The function stores:
- kind: Text
- count: Int
- index: Int
- path: Text
- token: Text
- slash: Text
- op: Text
- key: Text
- operation: Json
- value: Json
- error: Text
The function executes:
- try:
- the function gets JSON kind from local.operations into local.kind
- if boolean text local.kind != "array":
- the function raises error VALUE
- end if
- the function counts entries in JSON local.operations into local.count
- the function assigns "/" to local.slash
- the function assigns 0 to local.index
- while local.index < local.count:
- the function formats local.index as JSON text into local.token
- the function joins text local.slash and local.token into local.path
- the function gets JSON value at path local.path from local.operations into local.operation
- the function gets JSON kind from local.operation into local.kind
- if boolean text local.kind != "object":
- the function raises error VALUE
- end if
- the function assigns "/op" to local.path
- the function gets JSON value at path local.path from local.operation into local.op
- if boolean text local.op != "get" and text local.op != "put":
- the function raises error VALUE
- end if
- the function assigns "/key" to local.path
- the function gets JSON value at path local.path from local.operation into local.key
- if boolean text local.op == "put":
- the function assigns "/value" to local.path
- the function gets JSON value at path local.path from local.operation into local.value
- end if
- the function computes local.index as local.index + 1
- end while
- recover JSON or JSON-PATH into local.error:
- the function raises error VALUE
- end try
- the function returns 0

Function: executeValidated.
The function needs:
- state: Json
- operations: Json
The function produces:
- result: Json
The function stores:
- events: Json
- operation: Json
- transition: Json
- event: Json
- value: Json
- flag: Bool
- result: Json
- path: Text
- slash: Text
- token: Text
- op: Text
- key: Text
- count: Int
- index: Int
The function executes:
- the function assigns [] to local.events
- the function counts entries in JSON local.operations into local.count
- the function assigns "/" to local.slash
- the function assigns 0 to local.index
- while local.index < local.count:
- the function formats local.index as JSON text into local.token
- the function joins text local.slash and local.token into local.path
- the function gets JSON value at path local.path from local.operations into local.operation
- the function assigns "/op" to local.path
- the function gets JSON value at path local.path from local.operation into local.op
- the function assigns "/key" to local.path
- the function gets JSON value at path local.path from local.operation into local.key
- if boolean text local.op == "get":
- the function calls lookup with state = local.state; key = local.key into local.transition
- the function assigns {"op":"","key":"","hit":false,"value":null} to local.event
- the function assigns "/hit" to local.path
- the function gets JSON value at path local.path from local.transition into local.flag
- the function sets JSON value at path local.path in local.event to local.flag into local.event
- the function assigns "/value" to local.path
- the function gets JSON value at path local.path from local.transition into local.value
- the function sets JSON value at path local.path in local.event to local.value into local.event
- otherwise:
- the function assigns "/value" to local.path
- the function gets JSON value at path local.path from local.operation into local.value
- the function calls insert with state = local.state; key = local.key; value = local.value into local.transition
- the function assigns {"op":"","key":"","replaced":false,"evicted":null} to local.event
- the function assigns "/replaced" to local.path
- the function gets JSON value at path local.path from local.transition into local.flag
- the function sets JSON value at path local.path in local.event to local.flag into local.event
- the function assigns "/evicted" to local.path
- the function gets JSON value at path local.path from local.transition into local.value
- the function sets JSON value at path local.path in local.event to local.value into local.event
- end if
- the function assigns "/op" to local.path
- the function sets JSON value at path local.path in local.event to local.op into local.event
- the function assigns "/key" to local.path
- the function sets JSON value at path local.path in local.event to local.key into local.event
- the function assigns "/state" to local.path
- the function gets JSON value at path local.path from local.transition into local.state
- the function assigns "/-" to local.path
- the function sets JSON value at path local.path in local.events to local.event into local.events
- the function computes local.index as local.index + 1
- end while
- the function assigns {"state":null,"events":[]} to local.result
- the function assigns "/state" to local.path
- the function sets JSON value at path local.path in local.result to local.state into local.result
- the function assigns "/events" to local.path
- the function sets JSON value at path local.path in local.result to local.events into local.result
- the function returns local.result

Function: run.
The function needs:
- capacity: Int
- operations: Json
The function produces:
- result: Json
The function stores:
- state: Json
- validated: Int
- result: Json
The function executes:
- the function calls create with capacity = local.capacity into local.state
- the function calls validateOperations with operations = local.operations into local.validated
- the function calls executeValidated with state = local.state; operations = local.operations into local.result
- the function returns local.result

Function: failure.
The function needs:
- category: Text
The function produces:
- result: Json
The function stores:
- result: Json
- path: Text
The function executes:
- the function assigns {"ok":false,"state":null,"events":[],"error":""} to local.result
- the function assigns "/error" to local.path
- the function sets JSON value at path local.path in local.result to local.category into local.result
- the function returns local.result

Function: runRequest.
The function needs:
- text: Text
The function produces:
- result: Json
The function stores:
- request: Json
- operations: Json
- state: Json
- batch: Json
- events: Json
- result: Json
- capacity: Int
- validated: Int
- kind: Text
- path: Text
- error: Text
- category: Text
The function executes:
- try:
- the function parses JSON text local.text into local.request
- the function gets JSON kind from local.request into local.kind
- if boolean text local.kind != "object":
- the function raises error JSON
- end if
- the function assigns "/capacity" to local.path
- the function gets JSON value at path local.path from local.request into local.capacity
- the function assigns "/operations" to local.path
- the function gets JSON value at path local.path from local.request into local.operations
- the function gets JSON kind from local.operations into local.kind
- if boolean text local.kind != "array":
- the function raises error JSON
- end if
- recover JSON or JSON-PATH into local.error:
- the function assigns "REQUEST" to local.category
- the function calls failure with category = local.category into local.result
- the function returns local.result
- end try
- if local.capacity < 0:
- the function assigns "CAPACITY" to local.category
- the function calls failure with category = local.category into local.result
- the function returns local.result
- end if
- try:
- the function calls validateOperations with operations = local.operations into local.validated
- recover VALUE into local.error:
- the function assigns "OPERATIONS" to local.category
- the function calls failure with category = local.category into local.result
- the function returns local.result
- end try
- the function calls create with capacity = local.capacity into local.state
- the function calls executeValidated with state = local.state; operations = local.operations into local.batch
- the function assigns "/state" to local.path
- the function gets JSON value at path local.path from local.batch into local.state
- the function assigns "/events" to local.path
- the function gets JSON value at path local.path from local.batch into local.events
- the function assigns {"ok":true,"state":null,"events":[],"error":""} to local.result
- the function assigns "/state" to local.path
- the function sets JSON value at path local.path in local.result to local.state into local.result
- the function assigns "/events" to local.path
- the function sets JSON value at path local.path in local.result to local.events into local.result
- the function returns local.result

Function: contains.
The function needs:
- entries: Json
- key: Text
The function produces:
- result: Bool
The function stores:
- keys: List<Text>
- count: Int
- index: Int
- current: Text
- result: Bool
The function executes:
- the function gets JSON keys from local.entries into local.keys
- the function counts items in list local.keys into local.count
- the function assigns 0 to local.index
- the function assigns false to local.result
- while local.index < local.count:
- the function gets item local.index from list local.keys into local.current
- if boolean text local.current == local.key:
- the function assigns true to local.result
- the function returns local.result
- end if
- the function computes local.index as local.index + 1
- end while
- the function returns local.result
