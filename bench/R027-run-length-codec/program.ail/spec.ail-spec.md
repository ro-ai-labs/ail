The application RunLengthBenchmark manages binary run-length transformations.

A Data has:
- mode: Text
- input: Bytes
- caught: Text
- operation: Int

An Out has:
- value: Bytes
- error: Text

Action: Transform.
When Transform happens:
- the system assigns "hex:" to out.value
- the system assigns "" to out.error
- the system assigns 0 to data.operation
- try:
- if boolean text data.mode == "encode":
- the system assigns 1 to data.operation
- otherwise:
- if boolean text data.mode == "decode":
- the system assigns 2 to data.operation
- end if
- end if
- recover TEXT into data.caught:
- the system assigns 0 to data.operation
- end try
- if data.operation == 1:
- try:
- the system calls Rle.encode with data = data.input into out.value
- recover BYTES into data.caught:
- the system assigns "BYTES" to out.error
- end try
- otherwise:
- if data.operation == 2:
- try:
- the system calls Rle.decode with data = data.input into out.value
- recover BYTES into data.caught:
- the system assigns "BYTES" to out.error
- end try
- otherwise:
- the system assigns "MODE" to out.error
- end if
- end if
- the system returns out.value
- the system returns out.error
- the system records a trace event named Transformed
