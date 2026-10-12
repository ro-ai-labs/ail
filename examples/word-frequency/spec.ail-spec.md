The application WordFrequency manages exact frequencies from text requests.

A Data has:
- request: Text
- raw: Text
- report: Text
- newline: Text

An Out has:
- result: Json

Action: Analyze.
When Analyze happens:
- the system calls Words.summarizeRequest with text = data.request into out.result
- the system returns out.result
- the system records a trace event named Analyzed

Action: Count.
When Count happens:
- the system reads text from standard input into data.raw
- the system calls Words.summarize with text = data.raw into out.result
- the system formats out.result as JSON text into data.report
- the system assigns "\n" to data.newline
- the system joins text data.report and data.newline into data.report
- the system writes text data.report to standard output
- the system records a trace event named Counted
