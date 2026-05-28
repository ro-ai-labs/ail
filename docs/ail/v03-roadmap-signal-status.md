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
status: promoted
rationale: Support-ticket and incident repair-promotion Application entries now emit deterministic application-walkthrough artifacts for example-30 through example-34, example-90 through example-94, example-122, and example-123, tying user story, requirements, spec, checked Core, bytecode, runtime or target evidence, stateful boundary, trace event, repair provenance, semantic anchors, and replay fingerprints into reviewer-facing evidence.
evidence: cargo run -- ail-examples examples --release-evidence

signal: Complex systems need richer story graphs that span imported modules, UI surfaces, workflows, target contracts, and regenerated story views.
status: promoted
rationale: Incident-response complex-system examples now emit deterministic complex-story-graph artifacts for example-111 through example-115, tying imported identity, policy, and notification modules, the incident_response root workflow, command-center and service-owner UI surfaces, lifecycle transitions, target contracts, regenerated story views, semantic anchors, and replay fingerprints into reviewer-facing evidence.
evidence: cargo run -- ail-examples examples --release-evidence

signal: Generic runtime behavior needs clearer type-inference explanations.
status: promoted
rationale: Runtime Generic examples now emit deterministic type-inference review artifacts for example-35 through example-39, tying inferred state variants, preconditions, state transitions, trace coverage, and replay fingerprints into reviewer-facing evidence.
evidence: cargo run -- ail-examples examples --release-evidence

signal: Generics need reusable conformance fixtures and teachable stdlib walkthroughs.
status: promoted
rationale: Standard Collections examples now emit deterministic stdlib walkthrough artifacts for example-0 through example-9, tying generic type declarations, Option.map behavior, accepted and rejected fixtures, story anchors, and replay fingerprints into reviewer-facing evidence.
evidence: cargo run -- ail-examples examples --release-evidence

signal: Interop needs deeper unsafe-boundary tutorials and more ABI fixture diversity.
status: promoted
rationale: C interop examples now emit deterministic unsafe-boundary review artifacts for the ten accepted C interop entries, tying zlib/libc host calls, owned pointer release, borrowed mutable pointers, noescape callbacks, repr(C) layout, C status maps, nullable pointer rejection, accepted and rejected ABI fixtures, stable FFI diagnostics, trace coverage, and replay fingerprints into reviewer-facing evidence.
evidence: cargo run -- ail-examples examples --release-evidence

signal: Package graphs need clearer authoring guidance and dependency review views.
status: promoted
rationale: Support Composed package-import examples now emit deterministic dependency-review artifacts for example-10 through example-19, tying local package identity, Shared alias ownership, imported type use, capability grants, story anchors, and replay fingerprints into reviewer-facing evidence.
evidence: cargo run -- ail-examples examples --release-evidence

signal: Rejected examples need repair tutorials that convert diagnostics into corrected specs.
status: promoted
rationale: The rejected repair audit now checks all eight rejected entries behind this roadmap signal, verifies diagnostics, repair tutorials, corrected repair candidates, checked Core, bytecode, VM or target repair evidence, repair diffs, promotion reviews, fingerprint entries in the examples report and manifest, expected-diagnostic removal, and zero missing semantic anchors. Broader reviewer-produced live repair decisions remain a post-promotion bar.
evidence: scripts/run_v03_rejected_repair_audit.py

signal: Security examples need threat-model annotations and audit trails.
status: promoted
rationale: Secret Access examples now emit deterministic threat-model audit artifacts for entries example-75 through example-79, tying support-role checks, redaction, denied-access traces, diagnostic links, and replay fingerprints into reviewer-facing evidence.
evidence: cargo run -- ail-examples examples --release-evidence

signal: Self-hosting needs multiple composed compiler-pass variants and reviewer-visible pass-order conflict diagnostics.
status: promoted
rationale: The bootstrap chapter now records a second compiler-pass self-check composition variant, preserves ordered user-supplied --pass sequencing, and rejects a duplicate pass before the fixed-point gate as a fingerprinted AIL-BOOTSTRAP-PASS-ORDER-001 conflicting-order fixture. Broader accepted multi-pass compiler families remain a post-promotion bar.
evidence: docs/ail/31-v03-learning-and-authoring-gate.md

signal: State examples need clearer persistence and concurrency boundaries.
status: promoted
rationale: Stateful Counter examples now emit deterministic state-boundary review artifacts for example-95 through example-98, example-100, and example-110, tying persistence, idempotency, concurrency, failure replay, diagnostics, and replay fingerprints into reviewer-facing evidence.
evidence: cargo run -- ail-examples examples --release-evidence

signal: Systems profile needs unsupported-target migration guidance and broader transmit/interrupt runtime variants.
status: promoted
rationale: The Systems profile audit now compiles and runs receive, transmit, and interrupt-handler variants, verifies the packet-transmit fixture through conformance, fingerprints each runtime trace, and ties unsupported-target migration guidance to rejected catalog entry example-104 with AIL-BACKEND-001 evidence. Broader driver families and hardware target diversity remain a post-promotion bar.
evidence: scripts/run_v03_systems_profile_audit.py

signal: Turing Core examples need richer termination proofs beyond base-case, decreasing-argument, and numeric stack-bound patterns.
status: promoted
rationale: Recursive factorial conformance now includes an accepted well-founded termination-measure fixture in addition to base-case, decreasing-argument, numeric stack-bound, and rejected AIL-CONTROL-003 fixtures; the v0.3 manual and release audit preserve that conformance evidence.
evidence: docs/ail/manual/14-turing-core.md

signal: UI examples need richer package-local walkthroughs and stricter semantic tagging.
status: promoted
rationale: Option Map UI-surface examples now emit deterministic ui-semantic-tags artifacts for example-20 through example-24, tying package-local Option.map behavior, ui.form/ui.route/ui.state semantic tags, story anchors, checked Core, bytecode, and replay fingerprints into reviewer-facing evidence.
evidence: cargo run -- ail-examples examples --release-evidence

signal: Workflow examples need retry/backoff semantics and richer scheduler policies beyond temporal-policy diagnostics.
status: promoted
rationale: Repeated-task examples now emit deterministic workflow-scheduler review artifacts for example-80 through example-84, tying temporal policy, bounded retry policy, exponential backoff policy, accepted and rejected retry/backoff fixtures, stable AIL-WORKFLOW-001 and AIL-WORKFLOW-002 diagnostics, trace coverage, and replay fingerprints into reviewer-facing evidence.
evidence: cargo run -- ail-examples examples --release-evidence
