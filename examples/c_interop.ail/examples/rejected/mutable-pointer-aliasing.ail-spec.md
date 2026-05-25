# Rejected C Interop Fixture: Mutable Pointer Aliasing

C library: libc.

The library imports function swap_buffers.

swap_buffers needs:

- left: Pointer<UInt8> borrowed mutable alias buffer
- right: Pointer<UInt8> borrowed mutable alias buffer

swap_buffers produces:

- status: CInt

swap_buffers maps errno or status codes:

- OK maps to success

swap_buffers requires capability:

- call libc swap_buffers

swap_buffers records trace event named ForeignSwapBuffers

