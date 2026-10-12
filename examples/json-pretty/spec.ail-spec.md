The application JsonPretty manages JSON rendering requests.

A Data has:
- request: Text
- document: Json
- report: Text
- raw: Text
- newline: Text

An Out has:
- result: Bytes

Action: Analyze.
When Analyze happens:
- the system calls Pretty.renderRequest with text = data.request into data.document
- the system formats data.document as JSON text into data.report
- the system encodes text data.report as UTF-8 bytes into out.result
- the system returns out.result
- the system records a trace event named Rendered

Action: Format.
When Format happens:
- the system reads text from standard input into data.raw
- the system calls Pretty.renderText with text = data.raw into data.report
- the system assigns "\n" to data.newline
- the system joins text data.report and data.newline into data.report
- the system writes text data.report to standard output
- the system records a trace event named Formatted
