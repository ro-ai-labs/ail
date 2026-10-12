# Reusable finite Conway Life

The [pure Life library](../../examples/ail_std_life.ail/README.md) and
[stdin application](../../examples/conway-life/README.md) implement simultaneous
B3/S23 evolution on a finite rectangle with dead exterior. Package `ail.std.life`
0.1.0 uses the Application profile and alias `Life`. Import paths are relative
to the caller; the example imports `../ail_std_life.ail compatible ^0.1 as Life`.

| API | Behavior |
| --- | --- |
| `step(width: Int, height: Int, cells: List<Int>) -> List<Int>` | Validate, then advance once. |
| `evolve(width: Int, height: Int, cells: List<Int>, generations: Int) -> List<Int>` | Validate first, require a nonnegative count and advance exactly that many times. Zero returns canonical input. |
| `population(cells: List<Int>) -> Int` | Validate bits and sum; empty is 0. |
| `runRequest(text: Text) -> Json` | Validate a complete request and return the exact success/domain response. |

Width/height are positive signed-i64 integers. Cells has exactly width × height
items in row-major order, each 0 or 1. Overflow-free validation rejects oversized
mathematical products with VALUE directly or GRID through runRequest. Integer -0
is zero. Eight distinct neighboring coordinates exclude self; out-of-grid
coordinates contribute zero. A live cell survives with two or three neighbors;
a dead cell is born with three. All updates read one prior generation.

Retain inputs and returned lists for independent composition. For example,
`evolve(w,h,step(w,h,c),4)` equals `evolve(w,h,c,5)` while `c` and the first result
retain their original contents. step and evolve share one generation function.
Only current/next grids are retained by the algorithm; immutable runtime arenas
still account for allocated intermediate values cumulatively.

Request is one complete JSON object with exactly one decoded occurrence of each
selected name width, height, cells and generations. Extra members and unrelated
duplicates are ignored after full syntax and Unicode validation. Selected numbers
must be strict signed-i64 JSON integers; cells must be a flat integer array.
Fractions, exponent notation, booleans, nested cells, null and out-of-range selected
integers fail REQUEST. Escaped aliases count as duplicate selected names. Large
numbers in ignored fields stay opaque. Reject BOM, non-JSON whitespace, invalid
Unicode anywhere, trailing documents, NaN/Infinity and malformed JSON. Literal
leading/trailing space, tab, CR and LF are accepted.

Error priority is REQUEST, then GRID, then GENERATIONS. Zero generations cannot
bypass grid validation. Success is exactly
`{"ok":true,"cells":C,"population":P,"error":""}`. Failure is exactly
`{"ok":false,"cells":[],"population":0,"error":CATEGORY}`. Field order and
canonical decimal bits are fixed. Direct invalid domains raise catchable VALUE;
malformed typed transport can fail before entry. JSON/JSON-PATH recovery encloses
only request extraction and VALUE recovery only grid validation. Execution,
counting the final population, construction and output propagate fatal failures.

Analyze accepts data.request Text, selects only out.result Bytes and records
Simulated. Run reads raw UTF-8 stdin to EOF, calls the same adapter, serializes,
appends one LF, then writes once. `--stdio` grants both streams and suppresses
runtime reports. Success/domain responses exit zero with empty stderr. Missing
grants, malformed UTF-8 and runtime bounds fail without result stdout or private
operand markers. An OS write failure cannot make the single write transactional.

This is ordinary AIL with no host Life primitive, dependencies, cycle shortcuts
or budget increase. Existing bounds include 65,536 list items, 1 MiB value
transport, cumulative 64 MiB list/Json and Text arenas, 64 value-function frames
and invocation fuel. Immutable append copies and reparses lists, so dense larger
or longer simulations cost substantial time and allocation.

Required accepted workloads include dead/all-live/blinker/glider/mixed 20×20×50,
32×32×10, 1×128/128×1×40 and 1×1×257. A valid small request padded to exactly
1 MiB is the stdin input control; 1 MiB+1 reaches TEXT-LIMIT. A 65,537-cell request
reaches JSON-LIMIT before typed-list extraction. Depth65 reaches JSON-LIMIT.
1×1×10000 reaches fatal instruction fuel, paired with 257 generations. 32×32×70
reaches the real shared immutable-result LIST-LIMIT, paired with one generation.
These observations describe the measured constructions, not portable maximum
simulation sizes. A canonical cells list is at most 131,073 bytes; the response adds small fixed fields,
so a separate 1 MiB canonical response boundary is unreachable and is not claimed.

The [independent acceptance harness](../../scripts/ail_life_differential.py) freezes
all 172 oracle requests and their membership/fingerprint. Source, saved Core and
rendered Spec execute on VM/native directly and through their own exact STE,
plus source saved-bytecode VM/native: fourteen app and fourteen direct routes.
Every request receives Analyze and actual stdin comparisons; direct calls cover
all APIs, canonical zero, composition, retained input, repeated calls and VALUE.
Full source VM/native reports for all APIs and domain/composition cases precede
the broad matrix. Successful sequential-update and wrapped-edge mutants compile,
exit zero and produce independently rejected incorrect responses on both backends.

Artifacts reserve exclusive directory ownership before children or writes; occupied
and concurrent attempts reject without changing existing evidence. Immutable
attempt drivers are hashed before and throughout bounded execution. Startup/final
dependency snapshots, exact argv/status/streams, failures and generated artifacts
remain available for review. The source STE route executes the checked export's
exact bytes. Support registration preserves the historical 128 replay entries.
