The application TextFiles manages documents.

A Data has:
- body: Text
- heading: Text
- result: Text

An Output has:
- characters: Int

Action: Process.
When Process happens:
- the system reads text from file resource input into data.body
- the system assigns "Processed by AIL:\n" to data.heading
- the system joins text data.heading and data.body into data.result
- the system counts characters in data.result into output.characters
- the system calls Save
- the system returns output.characters
- the system records a trace event named Processed

Action: Save.
When Save happens:
- the system writes text data.result to file resource output
- the system records a trace event named Saved
