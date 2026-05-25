# Rejected C Interop Fixture: Missing Status Map

C library: libc.

The library imports function open_file.

open_file needs:

- path: Pointer<CChar> borrowed

open_file produces:

- status: CInt

open_file requires capability:

- call libc open_file

open_file records trace event named ForeignOpenFile
