# Accepted C Interop Fixture: Owned Pointer Release

C library: libc.

The library imports function strdup.

strdup needs:

- source: Pointer<CChar> borrowed

strdup produces:

- duplicate: Pointer<CChar> owned release free

strdup requires capability:

- call libc strdup

strdup records trace event named ForeignStrdup

