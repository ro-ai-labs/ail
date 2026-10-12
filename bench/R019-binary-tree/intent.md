rung: R019-binary-tree
tier: 1
action: Analyze
targets: vm,native
blocked-on: executable recursive variants and pure AIL binary search trees

# R019 — Insert, traverse and query a binary search tree

Read `data.values: List<Int>` and `data.query: Int`. Insert the input values in
their given order into an initially empty binary search tree. Each distinct key
has one node; repeated keys increase that node's multiplicity. Return exactly
`out.sorted: List<Int>` (ascending, with all duplicates), `out.present: Bool`
(whether the query occurs), and `out.height: Int` (empty tree 0, one node 1).

The AIL implementation must use declared recursive variants, ordinary functions,
and in-order traversal. No sorting opcode, host tree, dedicated tree opcode or
external process may implement the computation. Duplicates affect output length
but do not change tree height. Inputs are valid signed 64-bit integers and valid
List<Int> transports. Frozen shapes have actual tree height at most 62, including
ascending and descending trees at that edge, within the existing call limit.

The independent Python reference builds and traverses a mutable tree iteratively.
It uses neither the AIL evaluator nor a sorting routine to derive expectations.
Fixed cases cover empty, singleton, repeated keys, sorted and reverse insertion,
balanced and irregular shapes, both query outcomes, signed extrema, and seeded
generated inputs. This definition, reference and inputs are frozen before the AIL
implementation. See docs/superpowers/specs/2026-10-11-executable-variants-design.md.
