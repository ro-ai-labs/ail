# AIL End-To-End Corpus

This directory will hold the v0.2 prompt-to-artifact release corpus consumed by
`ail-e2e-corpus`.

Each counted example must be replayable without live model access and must
record the full chain from stored prompt transcript to checked AIL artifact,
checked AIL-Core, bytecode, VM trace, and binary or target-contract evidence.
The v0.2 release gate requires at least 100 distinct semantic examples.
