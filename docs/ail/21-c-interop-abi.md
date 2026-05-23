# AIL C Interop And ABI Profile

## Purpose

C interop is a first-class AIL profile layered on AIL-System resource,
ownership, layout, and capability semantics. It is not an unchecked escape
hatch. Every imported function, type, pointer, callback, symbol, library, and
failure mapping is represented in AIL-Core and checked before lowering.

## Imported C Declarations

AIL-Spec canonical form:

```text
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

compress2 records trace event named ForeignCallCompress2
```

AIL-Core nodes:

```text
node ExternalBinding zlib.compress2 [binding_kind=CFunction,library=zlib,symbol=compress2]
node Layout zlib.compress2.signature : cdecl
node Input zlib.compress2.dest : Pointer<UInt8> [ownership=borrowed mutable]
node Input zlib.compress2.dest_len : Pointer<UInt64> [ownership=borrowed mutable]
node Input zlib.compress2.source : Pointer<UInt8> [ownership=borrowed]
node Input zlib.compress2.source_len : UInt64
node Input zlib.compress2.level : Int
node Output zlib.compress2.status : CInt
node StatusMap zlib.compress2.Z_OK : success [code=Z_OK]
node Capability call zlib compress2
node Failure OutOfMemory
node Failure OutputBufferTooSmall
edge uses_layout ExternalBinding:zlib.compress2 -> Layout:zlib.compress2.signature
edge has_input ExternalBinding:zlib.compress2 -> Input:zlib.compress2.dest
edge has_output ExternalBinding:zlib.compress2 -> Output:zlib.compress2.status
edge maps_status ExternalBinding:zlib.compress2 -> StatusMap:zlib.compress2.Z_OK [code=Z_OK]
edge requires ExternalBinding:zlib.compress2 -> Capability:call zlib compress2
edge may_fail_with ExternalBinding:zlib.compress2 -> Failure:OutOfMemory [code=Z_MEM_ERROR]
edge records_trace ExternalBinding:zlib.compress2 -> Trace:ForeignCallCompress2
```

Current implementation status: the bootstrap parser accepts imported C function
declarations, typed inputs and outputs, status-code success/failure maps,
required capabilities, and trace events, then lowers them into checked AIL-Core
`ExternalBinding` graphs. Non-failure maps use `StatusMap` nodes and
`maps_status` edges; failure maps use `may_fail_with` edges. AIL-Bytecode
preserves the external binding metadata so the Wasm sandbox contract report can
enumerate declared host imports. The VM and native backends do not yet call or
link foreign symbols; executable FFI calls remain a later backend step.

## Supported C Surface

The profile supports:

- functions
- structs
- unions
- enums
- typedefs
- constants
- function-like macros only when a binding generator expands them into typed
  declarations
- pointers
- arrays with declared length source
- callbacks and function pointers
- dynamic and static libraries
- symbol visibility
- calling conventions

Unsupported or ambiguous macro behavior is rejected unless translated into a
typed AIL declaration by a binding package.

## Layout Rules

`repr(C)` layout declarations reuse AIL-System `Layout` nodes:

```text
Struct: PacketHeader.

PacketHeader uses layout:

- repr(C), align 8

PacketHeader has fields:

- version: UInt8 at offset 0
- flags: UInt8 at offset 1
- length: UInt16 at offset 2
```

The checker verifies:

- field order
- size
- alignment
- padding
- target ABI
- endian policy
- stable hash of the layout declaration

## Pointer And Ownership Rules

Pointer forms:

- `Pointer<T> borrowed`: callee may read during the call
- `Pointer<T> borrowed mutable`: callee may read and write during the call
- `Pointer<T> owned`: ownership transfers to callee
- `Nullable<Pointer<T>>`: null allowed
- `NonNull<Pointer<T>>`: null rejected before call

An owned pointer must declare a release rule:

```text
owned pointer release:

- release function: free
- release responsibility: caller after success
- failure release behavior: caller retains ownership
```

The checker rejects:

- passing secret pointers to unredacted traces
- mutable pointer aliasing
- ownership transfer without release semantics
- nullable pointer passed to `NonNull`
- callback that outlives borrowed data

## Failure Mapping

C return codes, `errno`, null returns, and callback errors map into declared AIL
`Failure` nodes.

Example:

```text
If fopen returns null:

- the system maps errno ENOENT to Failure.FileNotFound
- the system maps errno EACCES to Failure.PermissionDenied
- the trace records ForeignCallFailed
```

Unmapped error codes are rejected for safe profiles and accepted only in
expert-mode unsafe profiles with an explicit catch-all failure.

## Callback Rules

Callbacks declare:

- function pointer type
- captured state
- lifetime
- thread-safety
- reentrancy
- allowed effects
- failure propagation
- trace event

Example:

```text
Callback: for each row.

The callback receives:

- row pointer: Pointer<Row> borrowed
- user data: Pointer<Context> borrowed mutable

The callback may:

- read row pointer
- write user data

The callback must not:

- store row pointer after return
```

## Linking

Bindings declare one of:

- static library archive
- dynamic library name and version
- platform framework
- generated object file
- syscall or OS ABI binding

Each target backend maps binding declarations into its own linker or loader
contract. The Linux ELF backend records dynamic symbol names, static archives,
or direct syscall usage in the native-bytecode report.

## Unsafe Boundaries

Unsafe C interop requires:

- `unsafe c interop` capability
- high-risk safety classification
- expert-mode review
- accepted rejected fixture showing the checker catches a nearby unsafe case
- trace event for every foreign call
- secret redaction rule across the FFI boundary

## Accepted Fixtures

C library import:

```text
Import function strlen from libc.
strlen needs text: Pointer<CChar> borrowed non-null.
strlen produces length: UInt64.
strlen requires capability call libc strlen.
strlen records trace ForeignCallStrlen.
```

Callback:

```text
Import function qsort from libc.
qsort needs comparator: Callback<(Pointer<Void>, Pointer<Void>) -> CInt>.
The comparator must not store borrowed pointers after return.
```

Struct layout:

```text
Struct PacketHeader uses repr(C), align 8.
```

Ownership transfer:

```text
Function strdup returns owned Pointer<CChar>.
Caller releases with free.
```

Rejected unsafe pointer:

```text
Function store_pointer receives Pointer<Row> borrowed.
The function stores row pointer globally.
```

Diagnostic:

```text
AIL-FFI-OWNERSHIP-001 borrowed pointer cannot escape the call boundary
```
