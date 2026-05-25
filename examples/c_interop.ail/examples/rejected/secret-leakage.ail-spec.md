# Rejected C Interop Fixture: Secret Leakage

C library: libc.

The library imports function send_secret.

send_secret needs:

- payload: Secret<Pointer<UInt8>> borrowed
- length: UInt64

send_secret produces:

- status: CInt

send_secret maps errno or status codes:

- OK maps to success

send_secret requires capability:

- call libc send_secret

send_secret records trace event named ForeignSendSecret

