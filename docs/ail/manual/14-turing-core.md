# AIL Manual: Turing Core

## Purpose

The Turing Core chapter checks recursive function evidence directly. It proves
that AIL can represent recursive functions with typed inputs and outputs,
checker-visible base cases, explicit stack bounds, and well-founded
termination measures before those functions are accepted as deterministic
conformance fixtures.

Run the chapter:

```sh
python3 scripts/run_ail_interactive_manual.py --chapter turing-core --run-checks
```

## Workflow

Run the recursive factorial conformance bundle:

```sh
cargo run -- ail-conformance examples/recursive_factorial.ail \
  --artifact-dir /tmp/ail-manual-turing-core-conformance
```

The command writes `conformance-report.txt`, `manifest.ail-conformance.txt`,
and fingerprints for the recursive package and its package-local fixtures.

The accepted fixtures must include:

```text
accepted: recursive-with-stack-bound.ail-spec.md
accepted: recursive-with-well-founded-measure.ail-spec.md
```

The rejected fixtures must include:

```text
rejected: recursive-without-base-case.ail-spec.md AIL-CONTROL-003
rejected: recursive-without-decreasing-argument.ail-spec.md AIL-CONTROL-003
```

The well-founded measure fixture uses:

```text
the function has a well-founded termination measure n that decreases to 0 on every recursive call
```

Lowering records that proof as a `TerminationMeasure` node connected to the
function by `has_termination_measure`. The checker accepts it only when the
measure is explicitly decreasing and has a visible lower bound or
well-foundedness statement.
