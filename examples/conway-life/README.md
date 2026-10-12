# Conway Life

Analyze accepts one `data.request` Text argument and returns only UTF-8
`out.result` Bytes, with the Simulated trace. Import `Life` to compose the pure
library functions. Example:

```sh
target/debug/ail ail-run examples/conway-life --action Analyze 'data.request={"width":1,"height":1,"cells":[1],"generations":1}'
```

Success JSON is `{"ok":true,"cells":[0],"population":0,"error":""}`.
Malformed requests produce REQUEST, invalid grids GRID, and negative generation
counts GENERATIONS. Fatal runtime bounds propagate. The stdin Run entry and STE
export are supplied by the application integration task.
