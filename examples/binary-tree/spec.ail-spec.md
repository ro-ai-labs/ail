The application BinaryTree manages insertion, traversal and search.

A Data has:
- values: List<Int>
- query: Int
- tree: Bst.Tree<Int>

An Out has:
- sorted: List<Int>
- present: Bool
- height: Int

Action: Analyze.
When Analyze happens:
- the system calls Bst.fromList with values = data.values into data.tree
- the system calls Bst.inorder with tree = data.tree into out.sorted
- the system calls Bst.contains with tree = data.tree; value = data.query into out.present
- the system calls Bst.height with tree = data.tree into out.height
- the system returns out.sorted
- the system returns out.present
- the system returns out.height
- the system records a trace event named Analyzed
