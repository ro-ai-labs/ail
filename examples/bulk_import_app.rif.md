app BulkImports

things:
  thing Batch
    field id: Id<Batch>
    field counts: List<Int>

operations:
  operation Bulk.add(counts: List<Int>) -> Unit

intent ImportBatch

subject:
  batch: Batch

steps:
  1. Set counts
     set: batch.counts = [1,2,3]
     changes: batch.counts

  2. Add counts
     call: Bulk.add([1,2,3])

guarantees:
  if this intent succeeds:
    batch.counts == [1,2,3]
