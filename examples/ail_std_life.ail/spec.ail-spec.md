The application LifeLibrary manages immutable finite grids.

Function: population.
The function needs:
- cells: List<Int>
The function produces:
- result: Int
The function stores:
- count: Int
- index: Int
- bit: Int
- total: Int
The function executes:
- the function counts items in list local.cells into local.count
- the function assigns 0 to local.index
- the function assigns 0 to local.total
- while local.index < local.count:
- the function gets item local.index from list local.cells into local.bit
- if boolean integer local.bit != 0 and integer local.bit != 1:
- the function raises error VALUE
- end if
- the function computes local.total as local.total + local.bit
- the function computes local.index as local.index + 1
- end while
- the function returns local.total

Function: validateGrid.
The function needs:
- width: Int
- height: Int
- cells: List<Int>
The function produces:
- result: List<Int>
The function stores:
- count: Int
- population: Int
The function executes:
- if boolean integer local.width <= 0 or integer local.height <= 0:
- the function raises error VALUE
- end if
- the function counts items in list local.cells into local.count
- if boolean integer local.count / local.height != local.width or integer local.count % local.height != 0:
- the function raises error VALUE
- end if
- the function calls population with cells = local.cells into local.population
- the function returns local.cells

Function: neighbors.
The function needs:
- width: Int
- height: Int
- cells: List<Int>
- x: Int
- y: Int
The function produces:
- result: Int
The function stores:
- dx: Int
- dy: Int
- nx: Int
- ny: Int
- bit: Int
- count: Int
The function executes:
- the function assigns 0 to local.count
- the function assigns -1 to local.dy
- while local.dy <= 1:
- the function computes local.ny as local.y + local.dy
- if boolean integer local.ny >= 0 and integer local.ny < local.height:
- the function assigns -1 to local.dx
- while local.dx <= 1:
- if boolean integer local.dx != 0 or integer local.dy != 0:
- the function computes local.nx as local.x + local.dx
- if boolean integer local.nx >= 0 and integer local.nx < local.width:
- the function gets item local.ny * local.width + local.nx from list local.cells into local.bit
- the function computes local.count as local.count + local.bit
- end if
- end if
- the function computes local.dx as local.dx + 1
- end while
- end if
- the function computes local.dy as local.dy + 1
- end while
- the function returns local.count

Function: generation.
The function needs:
- width: Int
- height: Int
- cells: List<Int>
The function produces:
- result: List<Int>
The function stores:
- result: List<Int>
- x: Int
- y: Int
- count: Int
- alive: Int
- bit: Int
The function executes:
- the function creates an empty list into local.result
- the function assigns 0 to local.y
- while local.y < local.height:
- the function assigns 0 to local.x
- while local.x < local.width:
- the function calls neighbors with width = local.width; height = local.height; cells = local.cells; x = local.x; y = local.y into local.count
- the function gets item local.y * local.width + local.x from list local.cells into local.alive
- the function assigns 0 to local.bit
- if boolean integer local.count == 3 or (integer local.count == 2 and integer local.alive == 1):
- the function assigns 1 to local.bit
- end if
- the function appends integer local.bit to list local.result into local.result
- the function computes local.x as local.x + 1
- end while
- the function computes local.y as local.y + 1
- end while
- the function returns local.result

Function: step.
The function needs:
- width: Int
- height: Int
- cells: List<Int>
The function produces:
- result: List<Int>
The function stores:
- result: List<Int>
The function executes:
- the function calls validateGrid with width = local.width; height = local.height; cells = local.cells into local.result
- the function calls generation with width = local.width; height = local.height; cells = local.result into local.result
- the function returns local.result

Function: evolveValidated.
The function needs:
- width: Int
- height: Int
- cells: List<Int>
- generations: Int
The function produces:
- result: List<Int>
The function stores:
- result: List<Int>
- index: Int
- count: Int
The function executes:
- the function counts items in list local.cells into local.count
- the function slices list local.cells from 0 to local.count into local.result
- the function assigns 0 to local.index
- while local.index < local.generations:
- the function calls generation with width = local.width; height = local.height; cells = local.result into local.result
- the function computes local.index as local.index + 1
- end while
- the function returns local.result

Function: evolve.
The function needs:
- width: Int
- height: Int
- cells: List<Int>
- generations: Int
The function produces:
- result: List<Int>
The function stores:
- result: List<Int>
The function executes:
- the function calls validateGrid with width = local.width; height = local.height; cells = local.cells into local.result
- if local.generations < 0:
- the function raises error VALUE
- end if
- the function calls evolveValidated with width = local.width; height = local.height; cells = local.result; generations = local.generations into local.result
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
- the function assigns {"ok":false,"cells":[],"population":0,"error":""} to local.result
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
- result: Json
- cells: List<Int>
- width: Int
- height: Int
- generations: Int
- population: Int
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
- the function assigns "/width" to local.path
- the function gets JSON value at path local.path from local.request into local.width
- the function assigns "/height" to local.path
- the function gets JSON value at path local.path from local.request into local.height
- the function assigns "/cells" to local.path
- the function gets JSON value at path local.path from local.request into local.cells
- the function assigns "/generations" to local.path
- the function gets JSON value at path local.path from local.request into local.generations
- recover JSON or JSON-PATH into local.error:
- the function assigns "REQUEST" to local.category
- the function calls failure with category = local.category into local.result
- the function returns local.result
- end try
- try:
- the function calls validateGrid with width = local.width; height = local.height; cells = local.cells into local.cells
- recover VALUE into local.error:
- the function assigns "GRID" to local.category
- the function calls failure with category = local.category into local.result
- the function returns local.result
- end try
- if local.generations < 0:
- the function assigns "GENERATIONS" to local.category
- the function calls failure with category = local.category into local.result
- the function returns local.result
- end if
- the function calls evolveValidated with width = local.width; height = local.height; cells = local.cells; generations = local.generations into local.cells
- the function calls population with cells = local.cells into local.population
- the function assigns {"ok":true,"cells":[],"population":0,"error":""} to local.result
- the function assigns "/cells" to local.path
- the function sets JSON value at path local.path in local.result to local.cells into local.result
- the function assigns "/population" to local.path
- the function sets JSON value at path local.path in local.result to local.population into local.result
- the function returns local.result
