The application ExpressionCalculatorBenchmark manages checked calculations.

A Data has:
- expression: Text
- variables: Json
- error: Text

An Out has:
- result: Json

Action: Evaluate.
When Evaluate happens:
- try:
- the system calls Calc.evaluate with expression = data.expression; variables = data.variables into out.result
- recover JSON or TEXT into data.error:
- the system assigns {"ok":false,"value":0,"error":"REQUEST"} to out.result
- end try
- the system returns out.result
- the system records a trace event named Evaluated
