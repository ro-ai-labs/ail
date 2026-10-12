The application ExpressionCalculator manages checked calculations.

A Data has:
- expression: Text
- variables: Json
- raw: Text
- newline: Text
- lines: List<Text>
- line: Text
- cleaned: Text
- report: Text
- error: Text
- count: Int
- index: Int
- length: Int

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

Action: Stream.
When Stream happens:
- the system reads text from standard input into data.raw
- the system assigns "\n" to data.newline
- the system splits text data.raw by data.newline into data.lines
- the system counts items in list data.lines into data.count
- the system assigns 0 to data.index
- while data.index < data.count:
- the system gets item data.index from list data.lines into data.line
- the system computes data.index as data.index + 1
- the system trims text data.line into data.cleaned
- the system counts characters in data.cleaned into data.length
- if data.length == 0:
- continue
- end if
- the system calls Calc.evaluateRequest with text = data.line into out.result
- the system formats out.result as JSON text into data.report
- the system joins text data.report and data.newline into data.report
- the system writes text data.report to standard output
- end while
- the system records a trace event named StreamCompleted
