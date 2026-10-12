The application LruCache manages cache simulation requests.

A Data has:
- request: Text
- document: Json
- report: Text

An Out has:
- result: Bytes

Action: Analyze.
When Analyze happens:
- the system calls LRU.runRequest with text = data.request into data.document
- the system formats data.document as JSON text into data.report
- the system encodes text data.report as UTF-8 bytes into out.result
- the system returns out.result
- the system records a trace event named Simulated
