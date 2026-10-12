# Conway's Life
action: Analyze
targets: vm,native
blocked-on: reusable pure AIL Life package

Simulate finite rectangular Conway Life with simultaneous B3/S23 updates,
dead exterior and row-major canonical 0/1 cells. Parse one complete request
with strict selected i64 fields, duplicate-preserving member validation and
complete Unicode validation. Return one complete UTF-8 JSON response as Bytes.

The independent dense Python oracle returns immutable direct grids, accepts
opaque ignored numeric lexemes, and uses no AIL code or other rung oracle.
Success field order is ok,cells,population,error; failure categories are
REQUEST, GRID and GENERATIONS in that priority. There is no simulation shortcut.

Required workloads: dead, all-live, blinker, glider and seeded mixed 20x20/50;
seeded mixed 32x32/10; 1x128 and 128x1/40; and 1x1/257.
Cases also freeze anchors, narrow edges, zero/odd/even counts, every selected
member's missing/duplicate/escaped-duplicate/type failures, syntax/Unicode,
i64 boundaries, negative zero, grid-domain failures and error priorities.

Ignored deep-container positive cases use 63 nested containers plus the request
root (64 open containers). Oversized fatal-depth checks belong outside this rung.
