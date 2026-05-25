# AIL Standard Core Package

Package: ail.std.core.

The application AIL Standard Core manages identity values and primitive
contracts used by standard library packages.

Function: Identity.copy.

The function needs:

- value: T

The function produces:

- result: T

When Identity.copy runs:

- the function returns value unchanged
- the function records a trace event named IdentityCopied
