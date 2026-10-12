The application RunLengthLibrary manages binary run-length encoding.

Function: readCount.
The function needs:
- data: Bytes
- offset: Int
The function produces:
- result: Int
The function stores:
- a: Int
- b: Int
- c: Int
- d: Int
The function executes:
- the function gets byte local.offset from bytes local.data into local.a
- the function gets byte local.offset + 1 from bytes local.data into local.b
- the function gets byte local.offset + 2 from bytes local.data into local.c
- the function gets byte local.offset + 3 from bytes local.data into local.d
- the function returns local.a * 16777216 + local.b * 65536 + local.c * 256 + local.d

Function: packet.
The function needs:
- count: Int
- value: Int
The function produces:
- result: Bytes
The function stores:
- result: Bytes
The function executes:
- if boolean integer local.count <= 0 or integer local.count > 4294967295:
- the function raises error BYTES
- end if
- the function assigns "hex:0000000000" to local.result
- the function sets byte 0 in bytes local.result to local.count / 16777216 into local.result
- the function sets byte 1 in bytes local.result to (local.count / 65536) % 256 into local.result
- the function sets byte 2 in bytes local.result to (local.count / 256) % 256 into local.result
- the function sets byte 3 in bytes local.result to local.count % 256 into local.result
- the function sets byte 4 in bytes local.result to local.value into local.result
- the function returns local.result

Function: encodedSize.
The function needs:
- data: Bytes
The function produces:
- result: Int
The function stores:
- left: Bytes
- right: Bytes
- length: Int
- middle: Int
- leftSize: Int
- rightSize: Int
- previous: Int
- value: Int
- index: Int
- size: Int
The function executes:
- the function counts bytes in local.data into local.length
- if local.length == 0:
- the function returns 0
- end if
- if local.length > 64:
- the function computes local.middle as local.length / 2
- the function slices bytes local.data from 0 to local.middle into local.left
- the function slices bytes local.data from local.middle to local.length into local.right
- the function calls encodedSize with data = local.left into local.leftSize
- the function calls encodedSize with data = local.right into local.rightSize
- the function gets byte -1 from bytes local.left into local.previous
- the function gets byte 0 from bytes local.right into local.value
- if local.previous == local.value:
- the function returns local.leftSize + local.rightSize - 5
- end if
- the function returns local.leftSize + local.rightSize
- end if
- the function gets byte 0 from bytes local.data into local.previous
- the function assigns 5 to local.size
- the function assigns 1 to local.index
- while local.index < local.length:
- the function gets byte local.index from bytes local.data into local.value
- if local.value != local.previous:
- the function computes local.size as local.size + 5
- the function computes local.previous as local.value
- end if
- the function computes local.index as local.index + 1
- end while
- the function returns local.size

Function: encodeBlock.
The function needs:
- data: Bytes
The function produces:
- result: Bytes
The function stores:
- left: Bytes
- right: Bytes
- result: Bytes
- run: Bytes
- length: Int
- middle: Int
- leftLength: Int
- rightLength: Int
- leftCount: Int
- rightCount: Int
- value: Int
- next: Int
- index: Int
- end: Int
The function executes:
- the function counts bytes in local.data into local.length
- if local.length > 64:
- the function computes local.middle as local.length / 2
- the function slices bytes local.data from 0 to local.middle into local.left
- the function slices bytes local.data from local.middle to local.length into local.right
- the function calls encodeBlock with data = local.left into local.left
- the function calls encodeBlock with data = local.right into local.right
- the function counts bytes in local.left into local.leftLength
- the function counts bytes in local.right into local.rightLength
- the function gets byte -1 from bytes local.left into local.value
- the function gets byte 4 from bytes local.right into local.next
- if local.value == local.next:
- the function calls readCount with data = local.left; offset = local.leftLength - 5 into local.leftCount
- the function calls readCount with data = local.right; offset = 0 into local.rightCount
- the function calls packet with count = local.leftCount + local.rightCount; value = local.value into local.run
- the function slices bytes local.left from 0 to local.leftLength - 5 into local.left
- the function slices bytes local.right from 5 to local.rightLength into local.right
- the function joins bytes local.left and local.run into local.left
- end if
- the function joins bytes local.left and local.right into local.result
- the function returns local.result
- end if
- the function assigns "hex:" to local.result
- the function assigns 0 to local.index
- while local.index < local.length:
- the function gets byte local.index from bytes local.data into local.value
- the function computes local.end as local.index + 1
- while local.end < local.length:
- the function gets byte local.end from bytes local.data into local.next
- if local.value != local.next:
- break
- end if
- the function computes local.end as local.end + 1
- end while
- the function calls packet with count = local.end - local.index; value = local.value into local.run
- the function joins bytes local.result and local.run into local.result
- the function computes local.index as local.end
- end while
- the function returns local.result

Function: encode.
The function needs:
- data: Bytes
The function produces:
- result: Bytes
The function stores:
- size: Int
- result: Bytes
The function executes:
- the function calls encodedSize with data = local.data into local.size
- if local.size > 1048576:
- the function raises error BYTES
- end if
- the function calls encodeBlock with data = local.data into local.result
- the function returns local.result

Function: decodedSize.
The function needs:
- data: Bytes
The function produces:
- result: Int
The function stores:
- left: Bytes
- right: Bytes
- length: Int
- middle: Int
- leftSize: Int
- rightSize: Int
- count: Int
- index: Int
- size: Int
The function executes:
- the function counts bytes in local.data into local.length
- if local.length % 5 != 0:
- the function raises error BYTES
- end if
- if local.length > 160:
- the function computes local.middle as (local.length / 5 / 2) * 5
- the function slices bytes local.data from 0 to local.middle into local.left
- the function slices bytes local.data from local.middle to local.length into local.right
- the function calls decodedSize with data = local.left into local.leftSize
- the function calls decodedSize with data = local.right into local.rightSize
- the function returns local.leftSize + local.rightSize
- end if
- the function assigns 0 to local.size
- the function assigns 0 to local.index
- while local.index < local.length:
- the function calls readCount with data = local.data; offset = local.index into local.count
- if local.count == 0:
- the function raises error BYTES
- end if
- the function computes local.size as local.size + local.count
- the function computes local.index as local.index + 5
- end while
- the function returns local.size

Function: repeatByte.
The function needs:
- count: Int
- value: Int
The function produces:
- result: Bytes
The function stores:
- block: Bytes
- result: Bytes
- remaining: Int
The function executes:
- if boolean integer local.count < 0 or integer local.count > 1048576:
- the function raises error BYTES
- end if
- the function assigns "hex:00" to local.block
- the function sets byte 0 in bytes local.block to local.value into local.block
- the function assigns "hex:" to local.result
- the function computes local.remaining as local.count
- while local.remaining > 0:
- if local.remaining % 2 == 1:
- the function joins bytes local.result and local.block into local.result
- end if
- the function computes local.remaining as local.remaining / 2
- if local.remaining > 0:
- the function joins bytes local.block and local.block into local.block
- end if
- end while
- the function returns local.result

Function: decodeBlock.
The function needs:
- data: Bytes
The function produces:
- result: Bytes
The function stores:
- left: Bytes
- right: Bytes
- result: Bytes
- length: Int
- middle: Int
- count: Int
- value: Int
The function executes:
- the function counts bytes in local.data into local.length
- if local.length == 0:
- the function assigns "hex:" to local.result
- the function returns local.result
- end if
- if local.length == 5:
- the function calls readCount with data = local.data; offset = 0 into local.count
- the function gets byte 4 from bytes local.data into local.value
- the function calls repeatByte with count = local.count; value = local.value into local.result
- the function returns local.result
- end if
- the function computes local.middle as (local.length / 5 / 2) * 5
- the function slices bytes local.data from 0 to local.middle into local.left
- the function slices bytes local.data from local.middle to local.length into local.right
- the function calls decodeBlock with data = local.left into local.left
- the function calls decodeBlock with data = local.right into local.right
- the function joins bytes local.left and local.right into local.result
- the function returns local.result

Function: decode.
The function needs:
- data: Bytes
The function produces:
- result: Bytes
The function stores:
- size: Int
- result: Bytes
The function executes:
- the function calls decodedSize with data = local.data into local.size
- if local.size > 1048576:
- the function raises error BYTES
- end if
- the function calls decodeBlock with data = local.data into local.result
- the function returns local.result
