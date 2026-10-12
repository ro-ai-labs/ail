# Report on discovered files

This AIL program lists an input directory, reads each discovered entry as numeric
JSONL, and writes reports and an ordered manifest under a separate output root.
No filenames are preselected and no external tool performs the computation.

```bash
cargo build
mkdir -p /tmp/ail-directory-reports
target/debug/ail ail-run examples/directory-report --action Generate \
  --read-directory=workspace="$PWD/examples/directory-report/input" \
  --write-directory=reports=/tmp/ail-directory-reports

target/debug/ail ail-compile examples/directory-report --action Generate \
  --target linux-x86_64-elf --out /tmp/ail-directory-report
/tmp/ail-directory-report \
  --read-directory=workspace="$PWD/examples/directory-report/input" \
  --write-directory=reports=/tmp/ail-directory-reports
```

Each nonblank row needs signed 64-bit integer `value` and `divisor` fields.
Division rounds toward zero. The reusable `parseRow` function catches JSON,
JSON-PATH, DIVZERO and OVERFLOW errors. Each report counts accepted and rejected
rows, sorts the accepted quotients, and counts errors by category. Extra fields
are allowed. Blank lines use AIL's fixed Unicode trim rules.

Entries are processed in sorted name order. Successful reports use
`result-<zero-based sorted index>.json`, allowing long and control-containing
input basenames without constructing an invalid output name. A PATH or TEXT
failure produces a manifest error entry and processing continues. Directories,
symlinks and special files appear in the listing but fail the regular-file read.
An invalid UTF-8 basename fails the initial listing before this per-entry handler.

The supplied input produces three reports and two error entries:

- `result-0.json`: two accepted rows, values `[-2,5]`.
- `result-1.json`: one accepted row, five rejected rows, values `[4]`.
- `result-2.json`: an empty report.
- `03-invalid-utf8.txt`: `AIL-RUNTIME-TEXT`.
- `04-nested`: `AIL-RUNTIME-PATH`; its child is not traversed.

`manifest.json` contains `{"entries":[...]}`. Each entry has `name`, `report`,
and `error`; failed entries have an empty report name. The command returns
`batch.count=5`, `batch.written=3`, and `batch.failed=2`. Manifest/report bytes are
canonical compact JSON without an added newline. Output roots must already exist.

Reports are written as each file finishes, and the manifest is written last.
Limits and host failures remain fatal. Earlier files and partial failed writes
can remain; execution does not clean stale reports or promise atomic batch output.
Listing and JSON/Text processing use the existing bounded result budgets, so the
maximum listing count is not a promise that every batch of that size completes.

See [directory resources](../../docs/ail/54-directory-resources.md), the
[STE export](../ste/directory-report.ail-ste), and the independent
`scripts/ail_directory_differential.py` oracle for the complete contract.
