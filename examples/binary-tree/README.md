# Recursive binary search tree

Build and query a binary search tree using a reusable pure AIL library and
executable variants. Input order determines tree shape; duplicates increase node
multiplicity. In-order traversal returns every input key in ascending order.

```bash
cargo build
target/debug/ail ail-run examples/binary-tree --action Analyze \
  'data.values=[5,2,8,2,7]' data.query=7
```

The selected values are:

```text
out.height=3
out.present=true
out.sorted=[2,2,5,7,8]
```

Compile the same program to standalone native ELF:

```bash
target/debug/ail ail-compile examples/binary-tree --action Analyze \
  --target linux-x86_64-elf --out /tmp/ail-tree
/tmp/ail-tree 'data.values=[5,2,8,2,7]' data.query=7
```

Generate and execute the controlled-English representation:

```bash
target/debug/ail ail-ste examples/binary-tree > /tmp/ail-tree.ail-ste
target/debug/ail ail-run /tmp/ail-tree.ail-ste --action Analyze \
  'data.values=[5,2,8,2,7]' data.query=7
```

The [library](../ail_std_tree.ail/README.md) uses named variant construction,
Boolean variant tests, checked payload extraction, recursion and ordinary list
operations. It uses no sorting opcode or host tree implementation. See the
[language reference](../../docs/ail/59-executable-variants.md) for type checking,
transport, imports and resource limits. The independent
[R019 benchmark](../../bench/R019-binary-tree/intent.md) measures actual VM/native
outputs against a separately implemented Python tree.
