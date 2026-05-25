# Rejected C Interop Fixture: Nullable To NonNull

C library: libc.

The library imports function strlen.

strlen needs:

- text: NonNull<Pointer<CChar>> nullable

strlen produces:

- length: UInt64

strlen requires capability:

- call libc strlen

strlen records trace event named ForeignStrlen

