The application WordFrequency manages exact frequencies from text requests.

A Data has:
- request: Text

An Out has:
- result: Json

Action: Analyze.
When Analyze happens:
- the system calls Words.summarizeRequest with text = data.request into out.result
- the system returns out.result
- the system records a trace event named Analyzed
