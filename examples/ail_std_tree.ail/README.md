# AIL binary search tree library

## Purpose

This package defines an executable recursive `Tree<T>` type and pure operations
on `Tree<Int>`. Nodes store a key, its multiplicity, and left/right subtrees;
the `Empty` variant terminates each branch. Inserting a duplicate increments
its multiplicity without increasing tree height.

## Concepts Taught

- `fromList(values: List<Int>) -> Tree<Int>` inserts keys in their input order.
- `insert(tree: Tree<Int>, value: Int) -> Tree<Int>` returns a new tree.
- `contains(tree: Tree<Int>, value: Int) -> Bool` searches for a key.
- `height(tree: Tree<Int>) -> Int` returns zero for empty, one for a single node.
- `inorder(tree: Tree<Int>) -> List<Int>` returns ascending keys with duplicates.

Import `../ail_std_tree.ail compatible ^0.1 as Bst` from a neighboring package.
The imported type is `Bst.Tree<Int>`. `repeatValue` is an implementation helper
that builds repeated list values by iterative doubling. Search/insertion assume
the ordering and positive multiplicities maintained by this library; the type
checker validates payload types, not that mathematical ordering invariant.

The type declaration is generic; these library functions specialize Int keys.
Generic executable functions and exhaustive pattern matching remain future work.
Existing resource limits apply, including custom nesting, call depth, instruction
fuel and cumulative allocation. Immutable reconstruction copies values; a large
or skewed workload can reach those limits even when its final output is small.

## Files To Inspect

Read the [source](spec.ail-spec.md), [package manifest](ail-package.md), and
[executable variant reference](../../docs/ail/59-executable-variants.md).
The [example application](../binary-tree/README.md) imports and calls the library.

## Expected Replay Artifacts

After `cargo build`, run `python3 scripts/ail_value_type_differential.py`.
It builds Core, rendered source, STE, bytecode and native artifacts in temporary
directories and compares actual outputs with an independent Python tree.
The [checked-in STE example](../ste/binary-tree.ail-ste) includes the imported
functions and their closed type schema.

## Rejected Fixtures

Variant protocol tests reject unknown fields, mixed nominal types, malformed
schemas and unowned Core operands before execution. Runtime tests reject malformed
unused payloads and wrong-variant extraction, preserving the result destination.
The [frozen tree cases](../../bench/R019-binary-tree/cases) exercise valid programs;
they were fixed before this implementation existed.

## Next Example To Read

Use [binary-tree](../binary-tree/README.md) for a complete caller, or compare the
[binary codec](../ail_std_rle.ail/README.md) with a library over builtin Bytes.

## v0.3 Learning Signal

The new capability is executable user-defined recursive data through both
backends. The package is support-only in the historical replay catalog and does
not increase its 128 counted examples. R019 and the differential suite measure
computation; the broader Python-replacement goal remains active.
