# C Interop AIL-Spec Example

The application C Interop manages ABI-safe host bindings.

C library: zlib.

The library imports function compress2.

compress2 needs:

- dest: Pointer<UInt8> borrowed mutable
- dest_len: Pointer<UInt64> borrowed mutable
- source: Pointer<UInt8> borrowed
- source_len: UInt64
- level: Int

compress2 produces:

- status: CInt

compress2 maps errno or status codes:

- Z_OK maps to success
- Z_MEM_ERROR maps to Failure.OutOfMemory
- Z_BUF_ERROR maps to Failure.OutputBufferTooSmall

compress2 requires capability:

- call zlib compress2

compress2 records trace event named ForeignCallCompress2Scenario027

C library: libc.

The library imports function qsort.

qsort needs:

- base: Pointer<Void> borrowed mutable
- count: UInt64
- width: UInt64
- comparator: Callback<Pointer<Void>,Pointer<Void>,CInt> borrowed callback noescape

qsort produces:

- status: CInt

qsort maps errno or status codes:

- OK maps to success
- EINVAL maps to Failure.InvalidComparator

qsort requires capability:

- call libc qsort

qsort records trace event named ForeignCallbackComparedScenario027

System component: Packet header layout.

The component uses:

- packet header: Buffer

The component lays out:

- packet header: repr(C), size 4, align 2, offsets version=0 flags=1 length=2, target wasm32-unknown-sandbox-wasm

The component records:

- PacketHeaderLayoutCheckedScenario027

Action: Compress payload.

When compress payload happens:

- the system records a trace event named PayloadCompressedScenario027

Failure OutOfMemory happens when zlib reports memory exhaustion:

- the caller sees "Out of memory"
- the trace records ForeignOutOfMemoryScenario027

Failure OutputBufferTooSmall happens when zlib reports the output buffer is too small:

- the caller sees "Output buffer too small"
- the trace records ForeignOutputBufferTooSmallScenario027

Failure InvalidComparator happens when libc rejects the callback comparator:

- the caller sees "Invalid comparator"
- the trace records ForeignInvalidComparatorScenario027
