# Interop Fixture: strlen Binding

status: accepted
profile: C interop

Canonical declaration:

```text
Import function strlen from libc.

strlen needs:

- text: Pointer<CChar> borrowed non-null

strlen produces:

- length: UInt64

strlen requires capability:

- call libc strlen

strlen records trace:

- ForeignCallStrlen
```

Expected:

- no pointer ownership transfer
- no secret disclosure
- trace event is present
