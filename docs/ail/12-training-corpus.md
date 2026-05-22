# AIL Training Corpus And Conformance

## Purpose

AIL should be developed with a training and conformance corpus from the start.
The corpus teaches humans, LLMs, and toolchain components the same semantic
contract.

## Example Artifact Set

Each accepted language feature should include paired artifacts:

- vague human request
- agent interview transcript
- structured English spec
- semantic IR
- no-code view model
- valid examples
- invalid examples
- diagnostics
- runtime traces
- debugging conversations
- patch examples
- round-trip examples
- conformance expectations

## Fine-Tuning Data

fine-tuning data may include interviews, specs, IR, patches, diagnostics,
traces, and explanations. Sensitive examples must be redacted or synthetic.

## Prompt Calibration Data

Prompt Calibration data is the small high-quality set used in system prompts,
few-shot examples, and agent evaluation. It should prefer regular forms,
explicit failures, explicit permissions, and corrections that ask questions
instead of guessing.

## Invalid Examples

Invalid Examples are first-class. They show missing permissions, missing
capabilities, unresolved questions, secret leaks, ambiguous writes, unhandled
failures, invalid types, bad round trips, and unsafe lowering.

## Diagnostics Dataset

The Diagnostics Dataset pairs invalid artifacts with expected checker messages,
source provenance, no-code highlights, repair suggestions, and agent follow-up
questions.

## Trace Dataset

The Trace Dataset pairs accepted programs and inputs with expected semantic
trace events and debugging explanations.

## Human Review Dataset

The Human Review Dataset measures whether non-engineers can understand what a
program does, what data it uses, what can fail, what secrets are protected, and
why behavior is allowed.

## Conformance Suite

The Conformance Suite is the authoritative executable corpus. It checks
parsing, elaboration, graph normalization, projection round trips, checker
diagnostics, runtime behavior, traces, no-code patches, and lowering
equivalence.
