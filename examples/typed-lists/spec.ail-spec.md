The application Lists manages batches.

An Input has:
- values: List<Int>
- names: List<Text>
- index: Int
- replacement: Text

An Output has:
- values: List<Int>
- names: List<Text>
- count: Int
- total: Int
- item: Int
- name: Text

Function: square.
The function needs:
- n: Int
The function produces:
- result: Int
The function executes:
- the function returns local.n * local.n

Action: Numbers.
When Numbers happens:
- the system creates an empty list into output.values
- the system counts items in list input.values into output.count
- the system computes input.index as 0
- the system computes output.total as 0
- while input.index < output.count:
- the system gets item input.index from list input.values into output.item
- the system calls square with n = output.item into output.item
- the system appends integer output.item to list output.values into output.values
- the system computes output.total as output.total + output.item
- the system computes input.index as input.index + 1
- end while
- the system returns output.values
- the system returns output.total
- the system returns output.count
- the system records a trace event named Calculated

Action: Names.
When Names happens:
- the system counts items in list input.names into output.count
- the system gets item input.index from list input.names into output.name
- the system appends text input.replacement to list input.names into output.names
- the system sets item -1 in list output.names to text output.name into output.names
- the system returns output.names
- the system returns output.name
- the system returns output.count
- the system records a trace event named Edited
