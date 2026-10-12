The application BinaryTreeLibrary manages recursive binary search trees.

Type: Tree<T>.
Tree has variants:
- Empty
- Node(left: Tree<T>, multiplicity: Int, right: Tree<T>, value: T)

Function: insert.
The function needs:
- tree: Tree<Int>
- value: Int
The function produces:
- result: Tree<Int>
The function stores:
- empty: Bool
- left: Tree<Int>
- right: Tree<Int>
- result: Tree<Int>
- count: Int
- current: Int
The function executes:
- the function tests Tree<Int> variant Empty of local.tree into local.empty
- if boolean local.empty:
- the function constructs Tree<Int> variant Empty with none into local.left
- the function constructs Tree<Int> variant Node with left = local.left; multiplicity = 1; right = local.left; value = local.value into local.result
- the function returns local.result
- end if
- the function gets field left of Tree<Int> variant Node from local.tree into local.left
- the function gets field right of Tree<Int> variant Node from local.tree into local.right
- the function gets field multiplicity of Tree<Int> variant Node from local.tree into local.count
- the function gets field value of Tree<Int> variant Node from local.tree into local.current
- if local.value < local.current:
- the function calls insert with tree = local.left; value = local.value into local.left
- otherwise:
- if local.value > local.current:
- the function calls insert with tree = local.right; value = local.value into local.right
- otherwise:
- the function computes local.count as local.count + 1
- end if
- end if
- the function constructs Tree<Int> variant Node with left = local.left; multiplicity = local.count; right = local.right; value = local.current into local.result
- the function returns local.result

Function: fromList.
The function needs:
- values: List<Int>
The function produces:
- result: Tree<Int>
The function stores:
- tree: Tree<Int>
- length: Int
- index: Int
- value: Int
The function executes:
- the function constructs Tree<Int> variant Empty with none into local.tree
- the function counts items in list local.values into local.length
- the function assigns 0 to local.index
- while local.index < local.length:
- the function gets item local.index from list local.values into local.value
- the function calls insert with tree = local.tree; value = local.value into local.tree
- the function computes local.index as local.index + 1
- end while
- the function returns local.tree

Function: contains.
The function needs:
- tree: Tree<Int>
- value: Int
The function produces:
- result: Bool
The function stores:
- empty: Bool
- found: Bool
- current: Int
The function executes:
- the function assigns false to local.found
- the function tests Tree<Int> variant Empty of local.tree into local.empty
- while boolean not local.empty:
- the function gets field value of Tree<Int> variant Node from local.tree into local.current
- if local.value == local.current:
- the function assigns true to local.found
- the function returns local.found
- end if
- if local.value < local.current:
- the function gets field left of Tree<Int> variant Node from local.tree into local.tree
- otherwise:
- the function gets field right of Tree<Int> variant Node from local.tree into local.tree
- end if
- the function tests Tree<Int> variant Empty of local.tree into local.empty
- end while
- the function returns local.found

Function: height.
The function needs:
- tree: Tree<Int>
The function produces:
- result: Int
The function stores:
- empty: Bool
- left: Tree<Int>
- right: Tree<Int>
- leftHeight: Int
- rightHeight: Int
The function executes:
- the function tests Tree<Int> variant Empty of local.tree into local.empty
- if boolean local.empty:
- the function returns 0
- end if
- the function gets field left of Tree<Int> variant Node from local.tree into local.left
- the function gets field right of Tree<Int> variant Node from local.tree into local.right
- the function calls height with tree = local.left into local.leftHeight
- the function calls height with tree = local.right into local.rightHeight
- if local.leftHeight > local.rightHeight:
- the function returns local.leftHeight + 1
- end if
- the function returns local.rightHeight + 1

Function: repeatValue.
The function needs:
- count: Int
- value: Int
The function produces:
- result: List<Int>
The function stores:
- block: List<Int>
- result: List<Int>
- remaining: Int
The function executes:
- if local.count < 1:
- the function raises error VALUE
- end if
- the function assigns [] to local.block
- the function appends integer local.value to list local.block into local.block
- the function assigns [] to local.result
- the function computes local.remaining as local.count
- while local.remaining > 0:
- if local.remaining % 2 == 1:
- the function concatenates list local.result and list local.block into local.result
- end if
- the function computes local.remaining as local.remaining / 2
- if local.remaining > 0:
- the function concatenates list local.block and list local.block into local.block
- end if
- end while
- the function returns local.result

Function: inorder.
The function needs:
- tree: Tree<Int>
The function produces:
- result: List<Int>
The function stores:
- empty: Bool
- left: Tree<Int>
- right: Tree<Int>
- leftValues: List<Int>
- rightValues: List<Int>
- middleValues: List<Int>
- count: Int
- value: Int
- result: List<Int>
The function executes:
- the function tests Tree<Int> variant Empty of local.tree into local.empty
- if boolean local.empty:
- the function assigns [] to local.result
- the function returns local.result
- end if
- the function gets field left of Tree<Int> variant Node from local.tree into local.left
- the function gets field right of Tree<Int> variant Node from local.tree into local.right
- the function gets field multiplicity of Tree<Int> variant Node from local.tree into local.count
- the function gets field value of Tree<Int> variant Node from local.tree into local.value
- the function calls inorder with tree = local.left into local.leftValues
- the function calls repeatValue with count = local.count; value = local.value into local.middleValues
- the function calls inorder with tree = local.right into local.rightValues
- the function concatenates list local.leftValues and list local.middleValues into local.result
- the function concatenates list local.result and list local.rightValues into local.result
- the function returns local.result
