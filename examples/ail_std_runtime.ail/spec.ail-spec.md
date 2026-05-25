# AIL Standard Runtime Package

Package: ail.std.runtime.

The application AIL Standard Runtime manages task execution and runtime failure
classification.

A RuntimeTask has:

- id: Text
- status: State<Ready, Running, Failed, Complete>

Action: Run task.

When the runtime runs a task:

- the system requires the task status to be Ready
- if RuntimeUnavailable
- the system changes the task status to Running
- the system records a trace event named TaskRun

Failure RuntimeUnavailable happens when the runtime cannot schedule the task:

- the system changes the task status to Failed
- the caller sees "Runtime unavailable"
- the trace records RuntimeUnavailableRecorded
