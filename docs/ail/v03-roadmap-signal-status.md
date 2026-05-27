# AIL v0.3 Roadmap Signal Status

This file is the release-audit registry for high-count `ail-v03-roadmap`
signals. A signal with count `5` or higher must be classified here as either
`promoted` or `deferred`; otherwise `scripts/run_v03_signal_status_audit.py`
rejects the release audit bundle. `promoted` means the signal has been turned
into checked language, prompt, checker, runtime, target, documentation, or
agent-contract evidence. `deferred` means the signal remains an intentional
v0.3 follow-up with an explicit rationale.

signal: AgentTool authoring needs human-approved multi-agent policy handoff imports after deterministic policy reviews are replayed.
status: promoted
rationale: The v0.3 release audit now bundles the AgentTool capture plan, human-approved policy import demo, and deterministic multi-agent handoff report for example-40-policy.
evidence: scripts/run_v03_agent_policy_import_audit.py

signal: Application examples need more repaired incident promotion variants and richer stateful application walkthroughs after the first package-local repair proof is promoted.
status: deferred
rationale: The private-notes and commander-review incident repair proofs are promoted as example-122 and example-123, but richer stateful application walkthroughs and broader repaired incident coverage remain intentionally open.
evidence: docs/ail/31-v03-learning-and-authoring-gate.md

signal: Complex systems need richer story graphs that span imported modules, UI surfaces, workflows, target contracts, and regenerated story views.
status: deferred
rationale: Incident-response examples now span imported modules and target evidence, but richer regenerated multi-surface story graphs remain a next-version work item.
evidence: docs/ail/31-v03-learning-and-authoring-gate.md

signal: Generic runtime behavior needs clearer type-inference explanations.
status: promoted
rationale: Runtime Generic examples now emit deterministic type-inference review artifacts for example-35 through example-39, tying inferred state variants, preconditions, state transitions, trace coverage, and replay fingerprints into reviewer-facing evidence.
evidence: cargo run -- ail-examples examples --release-evidence

signal: Generics need reusable conformance fixtures and teachable stdlib walkthroughs.
status: promoted
rationale: Standard Collections examples now emit deterministic stdlib walkthrough artifacts for example-0 through example-9, tying generic type declarations, Option.map behavior, accepted and rejected fixtures, story anchors, and replay fingerprints into reviewer-facing evidence.
evidence: cargo run -- ail-examples examples --release-evidence

signal: Interop needs deeper unsafe-boundary tutorials and more ABI fixture diversity.
status: deferred
rationale: Current interop fixtures cover layouts, callbacks, ownership, nullability, aliasing, status maps, and secret leakage, but deeper unsafe-boundary tutorials and ABI diversity remain open.
evidence: docs/ail/31-v03-learning-and-authoring-gate.md

signal: Package graphs need clearer authoring guidance and dependency review views.
status: promoted
rationale: Support Composed package-import examples now emit deterministic dependency-review artifacts for example-10 through example-19, tying local package identity, Shared alias ownership, imported type use, capability grants, story anchors, and replay fingerprints into reviewer-facing evidence.
evidence: cargo run -- ail-examples examples --release-evidence

signal: Rejected examples need repair tutorials that convert diagnostics into corrected specs.
status: deferred
rationale: Rejected entries now emit repair tutorials, repair proofs, diffs, and promotion reviews, but multi-diagnostic promotion coverage and reviewer-produced decisions are still the next bar.
evidence: docs/ail/31-v03-learning-and-authoring-gate.md

signal: Security examples need threat-model annotations and audit trails.
status: promoted
rationale: Secret Access examples now emit deterministic threat-model audit artifacts for entries example-75 through example-79, tying support-role checks, redaction, denied-access traces, diagnostic links, and replay fingerprints into reviewer-facing evidence.
evidence: cargo run -- ail-examples examples --release-evidence

signal: Self-hosting needs multiple composed compiler-pass variants and reviewer-visible pass-order conflict diagnostics.
status: deferred
rationale: The bootstrap chapter proves one fixed-point compiler-pass composition, but multiple composed variants and pass-order conflict diagnostics remain open.
evidence: docs/ail/31-v03-learning-and-authoring-gate.md

signal: State examples need clearer persistence and concurrency boundaries.
status: promoted
rationale: Stateful Counter examples now emit deterministic state-boundary review artifacts for example-95 through example-98, example-100, and example-110, tying persistence, idempotency, concurrency, failure replay, diagnostics, and replay fingerprints into reviewer-facing evidence.
evidence: cargo run -- ail-examples examples --release-evidence

signal: Systems profile needs unsupported-target migration guidance and broader transmit/interrupt runtime variants.
status: deferred
rationale: Systems manual evidence covers scheduler, interrupt, native receive, and invalid-contract diagnostics, but transmit and interrupt-handler variants plus unsupported-target migration guidance remain deferred.
evidence: docs/ail/31-v03-learning-and-authoring-gate.md

signal: Turing Core examples need richer termination proofs beyond base-case, decreasing-argument, and numeric stack-bound patterns.
status: promoted
rationale: Recursive factorial conformance now includes an accepted well-founded termination-measure fixture in addition to base-case, decreasing-argument, numeric stack-bound, and rejected AIL-CONTROL-003 fixtures; the v0.3 manual and release audit preserve that conformance evidence.
evidence: docs/ail/manual/14-turing-core.md

signal: UI examples need richer package-local walkthroughs and stricter semantic tagging.
status: deferred
rationale: UI examples now include deterministic review, accessibility, patch import, and runtime state evidence, but more package-local walkthroughs and stricter semantic tagging are still needed.
evidence: docs/ail/31-v03-learning-and-authoring-gate.md

signal: Workflow examples need retry/backoff semantics and richer scheduler policies beyond temporal-policy diagnostics.
status: deferred
rationale: Repeated-task examples prove current scheduled-workflow behavior, but retry/backoff semantics and richer scheduler policies remain intentionally deferred.
evidence: docs/ail/31-v03-learning-and-authoring-gate.md
