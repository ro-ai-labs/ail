# Reusable Conway's Life in AIL

## Intent and chosen approach

Continue the authorized Specs → checked Core → ASD-STE100-based STE → VM/native
language-development goal after the completed 23/23 LRU milestone. Implement
planned rung R025, Conway's Life on a 20×20 grid for 50 generations, as reusable
pure AIL functions and a bounded stdin tool. This is an architectural addition
with a written contract, independent oracle and task reviews. Standing continuous
reversible-work authority covers design and implementation without repeated
permission pauses, git mutations, installation or publication. The broader
Python-replacement and AI-agent goal remains active.

Choose a flat `List<Int>` with explicit width and height. It composes with the
existing typed-list and integer APIs and makes dimensions, indexing and result
transport explicit. A text picture would add parsing conventions to the core API;
sparse coordinates would add duplicate/order policy and favor sparse workloads.
Neither is needed for this milestone. Finite dead boundaries avoid the ambiguous
wrapped-neighbor multiplicity of one-cell-wide toroidal grids.

All simulation code is ordinary pure AIL. Existing compiler/runtime, previous
rung inputs/programs, scripts and tests stay unchanged. No host Life primitive,
new opcode, dependency, budget increase, instruction padding, cycle shortcut or
arbitrary grid/generation cap is allowed. If execution exposes a real blocker,
preserve the reproducer and review the smallest scope change before proceeding.

References: [typed lists](../../ail/38-typed-lists.md),
[value functions](../../ail/44-value-functions.md),
[JSON lookup](../../ail/42-recursive-json.md) and
[runtime recovery](../../ail/51-runtime-recovery.md).

## Grid and evolution semantics

Width and height are positive signed 64-bit integers. Cells is a flat integer
list with exactly width × height items; every item is 0 (dead) or 1 (alive).
Index `y * width + x` selects column x of row y, both zero based. No empty or
zero-dimensional grid is valid. Dimension/product mismatch and mathematically
oversized products are domain errors; validation must avoid integer overflow.
In particular, maximum-i64 dimensions with a short list report VALUE directly
or GRID through the request adapter, rather than INT-OVERFLOW.

For each cell, count the eight neighboring coordinates with offsets -1, 0 or 1
on each axis, excluding (0, 0). Coordinates outside the rectangle are dead.
There is no wrapping, reflected boundary or duplicated neighbor. An alive cell
survives with two or three live neighbors. A dead cell becomes alive with exactly
three. Every other cell is dead in the next generation. All cells read the same
previous generation: updates are simultaneous, never visible within a generation.

Each call returns immutable canonical integer-list state. It cannot modify a
retained input or earlier returned grid. Integer -0 is accepted as zero and
normalizes to 0. There is no implicit crop, resize, padding or reordering.

Hand-calculated anchors include:

- A live 1×1 cell dies after one step; a dead 1×1 cell stays dead.
- A 2×2 all-live block remains `[1,1,1,1]`.
- A 3×3 horizontal blinker `[0,0,0,1,1,1,0,0,0]` becomes
  `[0,1,0,0,1,0,0,1,0]`, then returns to its original state.
- A 3×3 grid `[1,1,0,1,0,0,0,0,0]` becomes
  `[1,1,0,1,1,0,0,0,0]`, demonstrating corner birth and dead boundaries.

## Public pure API

Package `ail.std.life` version 0.1.0, Application profile, directory
`examples/ail_std_life.ail`, imported as `Life`. Exact signatures:

- `step(width: Int, height: Int, cells: List<Int>) -> List<Int>`:
  validate the full grid and return its next generation.
- `evolve(width: Int, height: Int, cells: List<Int>, generations: Int) -> List<Int>`:
  validate the full grid first, then require nonnegative generations. Zero returns
  the validated canonical input. Otherwise perform exactly that many transitions,
  reusing the same generation implementation as step. Retain only the current
  and next grids, not all historical generations.
- `population(cells: List<Int>) -> Int`: require only 0/1 items, then return their
  sum. An empty list is valid here and returns 0; this helper has no dimensions.
- `runRequest(text: Text) -> Json`: apply the request contract below.

Invalid well-typed domain arguments raise existing catchable VALUE. Malformed
typed List/Text transport and argument-boundary errors may occur before the
function body. Existing fatal fuel, depth and allocation errors propagate.
No broad recovery turns a fatal runtime failure into a successful domain result.

Share meaningful helpers for validation, bounded neighbor access and a single
generation. Reusing an unchecked internal generation after initial validation
is allowed. Public step and evolve must use the same transition algorithm.
Function decomposition must serve these responsibilities; helpers whose only
purpose is to manufacture fuel are forbidden. Public results and behavior do
not depend on how helpers are named.

## JSON request and response

A request is exactly one complete JSON object containing one unambiguous member
for each selected name: `width`, `height`, `cells`, `generations`.
Width, height and generations must be strict signed-i64 JSON integers. Cells
must be a flat array of strict signed-i64 JSON integers. Fractions, exponents,
Booleans, nulls, strings, nested cells and integers outside i64 are wrong types.
The integer spelling -0 is valid. Decoded-equivalent selected member names are
duplicates and invalid, including an escaped spelling of a selected name.

Extra members and unrelated duplicate names are ignored only after whole-document
JSON syntax and Unicode validation. Invalid Unicode in an ignored string or key
still fails. Large valid numbers in ignored fields are opaque and do not need
i64 conversion. Reject empty input, trailing documents, BOM, non-JSON whitespace,
NaN/Infinity and malformed JSON. Ordinary leading/trailing JSON whitespace is valid.

Error priority is explicit:

1. Complete parse, root and selected-member/type validation: REQUEST.
2. Full grid dimensions, length and 0/1 domain validation: GRID.
3. Negative generations: GENERATIONS.
4. Execute validated evolution and count the result population.

Thus a negative generation with a nonarray cells member is REQUEST; with a
well-typed but invalid grid it is GRID; with a valid grid it is GENERATIONS.
A nonbit integer cell is GRID, whereas a Boolean/fractional cell is REQUEST.
Input validation is required even for zero generations. Fatal runtime bounds may
interrupt parsing/validation and do not promise a domain response.

Success is exactly `{"ok":true,"cells":C,"population":P,"error":""}`.
Failure is exactly `{"ok":false,"cells":[],"population":0,"error":CATEGORY}`.
Field order is exact; CATEGORY is REQUEST, GRID or GENERATIONS. These are JSON
response strings, not new runtime categories. Response lists contain canonical
decimal 0/1 values. Catch existing JSON/JSON-PATH only around request extraction;
catch VALUE only around domain validation. Simulation, population computation
after validation, result construction and output remain outside those catches.

## Applications and pipeline

Create `examples/conway-life` and `bench/R025-conway-life/program.ail` importing
the library. Their Analyze bodies match exactly: accept `data.request: Text`,
call Life.runRequest into `data.document: Json`, format `data.report: Text`,
encode UTF-8 to `out.result: Bytes`, select only out.result and record Simulated.
Use complete Bytes transport; benchmark parsing must not truncate or manufacture
state lines from ignored input strings.

The example also has Run: read one raw UTF-8 JSON request from stdin through EOF,
call the same adapter, format compact JSON, append one LF and write once to
stdout. Use existing stdin/stdout grants and `--stdio` to suppress runtime reports.
Structured successes/errors exit zero, emit exactly that line and have empty
stderr. Missing grants, invalid UTF-8 and fatal failures exit nonzero without
result stdout or leaked operand/private-marker contents. All fallible work
precedes the only write. No JSON-lines protocol, GUI, history animation or service.

Export `examples/ste/conway-life.ail-ste` from final example source. Source,
saved Core and rendered Spec must agree through the normal and STE paths;
direct STE, saved bytecode VM and native compiled from saved bytecode must
actually execute. Generated executables use no Python interpreter or host model.

## Independent measurement and limits

Freeze `bench/R025-conway-life/{intent.md,reference.py,cases/*.args}` before any
AIL implementation. The oracle uses a dense Python grid. Cross-check it against
a distinct set-of-live-coordinates/neighbor-count model, hand anchors and seeded
cases. It must preserve duplicate-name/type distinctions when parsing requests,
validate Unicode throughout and never read AIL-produced expectations.

Required successful cases include dead, all-live, blinker, glider and deterministic
mixed 20×20 grids evolved 50 times; a 32×32 mixed grid evolved 10 times; 1×128 and
128×1 grids evolved 40 times; and a valid 1×1 grid evolved 257 times. Both zero
and odd/even generation counts, small rectangular grids, corners and narrow
dimensions are required. Direct API tests cover composition, retained input,
independent repeated calls and population. Required-size cases must pass the
unchanged benchmark deadline; a timeout is not a semantic pass.

Existing limits remain: 65,536 list items, 1 MiB value transport, 64 MiB list/Json
and text arenas, 64 active function frames and per-invocation instruction fuel.
Immutable append currently copies and reparses lists; this is not an efficient
mutable array or unbounded simulator. Full 20×20×50 success is a requirement to
measure, not a claim inferred from source inspection.

Acceptance compares complete oracle results, full representative VM/native
reports and exact raw stdin bytes. Include successful wrong-output mutants for
sequential in-place updates and wrapped edges; each must compile, exit zero and
be rejected by an independently chosen distinguishing case. Prove meaningful
fatal fuel/list-allocation and input/result limit behavior with positive controls;
preserve the actual reached category and avoid claiming an untested limit.

Every subprocess and outer driver has a finite deadline and descendant cleanup.
Keep original commands, streams, statuses, tested input/dependency/artifact hashes
and all failed/interrupted attempts in unique directories. An occupied artifact
directory must reject without modifying existing evidence; concurrent writers
must not share it. Review saved evidence without repeating passing matrices.
Record the new score only after independent task/whole-milestone review and one
unchanged full ladder run, preserving all 23 prior fingerprints and pass entries.
