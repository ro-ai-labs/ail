# Rejected C Interop Fixture: Owned Pointer Without Release

C library: libc.

The library imports function strdup.

strdup needs:

- source: Pointer<CChar> borrowed

strdup produces:

- duplicate: Pointer<CChar> owned

strdup requires capability:

- call libc strdup

strdup records trace event named ForeignStrdup

