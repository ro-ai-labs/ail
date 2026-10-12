The application CsvAggregate manages category totals from CSV requests.

A Data has:
- request: Text
- raw: Text
- newline: Text
- lines: List<Text>
- line: Text
- cleaned: Text
- report: Text
- path: Text
- ok: Bool
- index: Int
- count: Int

An Out has:
- result: Json

Action: Analyze.
When Analyze happens:
- the system calls Csv.summarizeRequest with text = data.request into out.result
- the system returns out.result
- the system records a trace event named Analyzed

Action: Stream.
When Stream happens:
- the system reads text from standard input into data.raw
- the system assigns "\n" to data.newline
- the system splits text data.raw by data.newline into data.lines
- the system counts items in list data.lines into data.count
- the system assigns 0 to data.index
- while boolean integer data.index < data.count:
- the system gets item data.index from list data.lines into data.line
- the system computes data.index as data.index + 1
- the system trims text data.line into data.cleaned
- if boolean text data.cleaned == "":
- continue
- end if
- the system calls Csv.summarizeRequest with text = data.line into out.result
- the system formats out.result as JSON text into data.report
- the system joins text data.report and data.newline into data.report
- the system writes text data.report to standard output
- end while
- the system records a trace event named Streamed

Action: WriteReport.
When WriteReport happens:
- the system reads text from file resource input into data.raw
- the system calls Csv.summarize with text = data.raw into out.result
- the system assigns "/ok" to data.path
- the system gets JSON value at path data.path from out.result into data.ok
- if boolean not data.ok:
- the system raises error VALUE
- end if
- the system formats out.result as JSON text into data.report
- the system assigns "\n" to data.newline
- the system joins text data.report and data.newline into data.report
- the system writes text data.report to file resource report
- the system records a trace event named ReportWritten
