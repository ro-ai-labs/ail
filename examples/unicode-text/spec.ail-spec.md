The application TextTools manages text.

An Input has:
- first: Text
- second: Text
- start: Int
- end: Int

An Output has:
- joined: Text
- part: Text
- count: Int
- position: Int

Action: Process.

When Process happens:
- the system joins text input.first and input.second into output.joined
- the system counts characters in output.joined into output.count
- the system slices text output.joined from input.start to input.end into output.part
- the system finds text input.second in output.joined into output.position
- the system returns output.joined
- the system returns output.part
- the system returns output.count
- the system returns output.position
- the system records a trace event named Processed
