# Rejected C Interop Fixture: Borrowed Pointer Escape

C library: libc.

The library imports function store_pointer.

store_pointer needs:

- row: Pointer<Void> borrowed escaping

store_pointer produces:

- status: CInt

store_pointer maps errno or status codes:

- OK maps to success

store_pointer requires capability:

- call libc store_pointer

store_pointer records trace event named ForeignStorePointer
