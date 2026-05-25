# Rejected C Interop Fixture: Missing Trace

C library: libc.

The library imports function close_file.

close_file needs:

- fd: CInt

close_file produces:

- status: CInt

close_file maps errno or status codes:

- OK maps to success

close_file requires capability:

- call libc close_file
