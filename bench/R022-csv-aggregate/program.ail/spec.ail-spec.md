The application CsvAggregate manages category totals from CSV requests.

A Data has:
- request: Text

An Out has:
- result: Json

Action: Analyze.
When Analyze happens:
- the system calls Csv.summarizeRequest with text = data.request into out.result
- the system returns out.result
- the system records a trace event named Analyzed
