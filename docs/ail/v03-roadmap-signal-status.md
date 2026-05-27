# AIL v0.3 Roadmap Signal Status

This file is the release-audit registry for high-count `ail-v03-roadmap`
signals. A signal with count `5` or higher must be classified here as either
`promoted` or `deferred`; otherwise `scripts/run_v03_signal_status_audit.py`
rejects the release audit bundle. `promoted` means the signal has been turned
into checked language, prompt, checker, runtime, target, documentation, or
agent-contract evidence. `deferred` means the signal remains an intentional
v0.3 follow-up with an explicit rationale.

signal: AgentTool authoring needs human-approved multi-agent policy handoff imports after deterministic policy reviews are replayed.
status: deferred
rationale: Deterministic policy import and role-separated handoff evidence exists, but broader live reviewer coverage with both accepted and rejected handoffs is still the next bar.
evidence: docs/ail/31-v03-learning-and-authoring-gate.md

signal: Application examples need more repaired incident promotion variants and richer stateful application walkthroughs after the first package-local repair proof is promoted.
status: deferred
rationale: The first incident private-notes repair proof is promoted as example-122, but additional repaired incident variants and richer stateful application walkthroughs remain intentionally open.
evidence: docs/ail/31-v03-learning-and-authoring-gate.md

signal: Complex systems need richer story graphs that span imported modules, UI surfaces, workflows, target contracts, and regenerated story views.
status: deferred
rationale: Incident-response examples now span imported modules and target evidence, but richer regenerated multi-surface story graphs remain a next-version work item.
evidence: docs/ail/31-v03-learning-and-authoring-gate.md

signal: Generic runtime behavior needs clearer type-inference explanations.
status: deferred
rationale: Runtime generic examples are replayable, but the teaching surface still needs clearer type-inference explanations for reviewers.
evidence: docs/ail/31-v03-learning-and-authoring-gate.md

signal: Generics need reusable conformance fixtures and teachable stdlib walkthroughs.
status: deferred
rationale: Standard-library package and Option.map fixtures exist, but more reusable generic conformance walkthroughs remain an intentional v0.3 learning task.
evidence: docs/ail/31-v03-learning-and-authoring-gate.md

signal: Interop needs deeper unsafe-boundary tutorials and more ABI fixture diversity.
status: deferred
rationale: Current interop fixtures cover layouts, callbacks, ownership, nullability, aliasing, status maps, and secret leakage, but deeper unsafe-boundary tutorials and ABI diversity remain open.
evidence: docs/ail/31-v03-learning-and-authoring-gate.md

signal: Package graphs need clearer authoring guidance and dependency review views.
status: deferred
rationale: Package resolution, dependency reports, and learning guides are checked, but reviewer-facing dependency review views still need more authoring guidance.
evidence: docs/ail/31-v03-learning-and-authoring-gate.md

signal: Rejected examples need repair tutorials that convert diagnostics into corrected specs.
status: deferred
rationale: Rejected entries now emit repair tutorials, repair proofs, diffs, and promotion reviews, but multi-diagnostic promotion coverage and reviewer-produced decisions are still the next bar.
evidence: docs/ail/31-v03-learning-and-authoring-gate.md

signal: Security examples need threat-model annotations and audit trails.
status: deferred
rationale: Secret Access conformance and redaction fixtures exist, but threat-model annotations and audit-trail artifacts are intentionally deferred.
evidence: docs/ail/31-v03-learning-and-authoring-gate.md

signal: Self-hosting needs multiple composed compiler-pass variants and reviewer-visible pass-order conflict diagnostics.
status: deferred
rationale: The bootstrap chapter proves one fixed-point compiler-pass composition, but multiple composed variants and pass-order conflict diagnostics remain open.
evidence: docs/ail/31-v03-learning-and-authoring-gate.md

signal: State examples need clearer persistence and concurrency boundaries.
status: deferred
rationale: Stateful runtime fixtures now cover persistence, idempotency, locking, and replay recovery, but migration, stale-state, transaction, and durable runtime evidence remain next-version work.
evidence: docs/ail/31-v03-learning-and-authoring-gate.md

signal: Systems profile needs unsupported-target migration guidance and broader transmit/interrupt runtime variants.
status: deferred
rationale: Systems manual evidence covers scheduler, interrupt, native receive, and invalid-contract diagnostics, but transmit and interrupt-handler variants plus unsupported-target migration guidance remain deferred.
evidence: docs/ail/31-v03-learning-and-authoring-gate.md

signal: Turing Core examples need richer termination proofs beyond base-case, decreasing-argument, and numeric stack-bound patterns.
status: deferred
rationale: Recursive factorial and stack-depth fixtures prove the first termination checks, but richer proof patterns remain an explicit v0.3 follow-up.
evidence: docs/ail/31-v03-learning-and-authoring-gate.md

signal: UI examples need richer package-local walkthroughs and stricter semantic tagging.
status: deferred
rationale: UI examples now include deterministic review, accessibility, patch import, and runtime state evidence, but more package-local walkthroughs and stricter semantic tagging are still needed.
evidence: docs/ail/31-v03-learning-and-authoring-gate.md

signal: Workflow examples need retry/backoff semantics and richer scheduler policies beyond temporal-policy diagnostics.
status: deferred
rationale: Repeated-task examples prove current scheduled-workflow behavior, but retry/backoff semantics and richer scheduler policies remain intentionally deferred.
evidence: docs/ail/31-v03-learning-and-authoring-gate.md
