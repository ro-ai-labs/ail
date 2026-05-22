# AIL / EIGL Prototype

AIL means Agentic Intent Language. It is the active language direction for this
repository: a semantic programming language and toolchain where humans begin in
English, AI agents help clarify intent, and checked deterministic artifacts
compile into executable behavior.

The current language specification suite starts at
[`docs/ail/README.md`](docs/ail/README.md).

This repository also contains an early Rust prototype from the previous EIGL:
Executable Intent Graph Language direction. The prototype remains useful
bootstrap scaffolding while AIL names, specs, and toolchain contracts are
defined.

The prototype starts from explicit RIF documents, builds an EIG-Core graph, checks safety rules, renders text views, and simulates simple process graphs.

`normalize` renders a parsed `.rif.md` or `.rsl.md` file back into canonical RIF text. That canonical RIF is the stable text intermediary for LLM-assisted authoring and round-tripping.

`patch` reads a structured RIF patch file, applies it to an existing document, and prints the patched canonical RIF. The patch format supports intent-scoped edits and application-model edits, and is meant for LLM-generated changes and human review.

`imports:` sections let one RIF file pull in another by path. Imported declarations are merged before checking and execution, and duplicate intents, things, operations, collections, endpoints, and triggers fail fast with a conflict error. Use `import path as Alias` when you want the imported fragment to be namespaced under a prefix. Imported files may keep their own app name; the root document's app name wins when both are present. Imported files can add an `exports:` section with lines like `export intent ArchiveOrder`; when exports are present, only those top-level declarations are contributed to the importer.

Collections can also declare `unique:` constraints. Each unique field is checked across stored records, so duplicate values fail fast before execution or persistence.

RIF also accepts an optional `module` line near the top of a document. The module name is preserved through rendering and is used as the module name in the EIG-Core program output.

`llm-roundtrip` sends canonical RIF to a local llama.cpp endpoint, asks it to rewrite the document back into RSL, and then verifies that parsing the rewrite returns the same canonical RIF. The default endpoint is `http://inteligentia-pro-1:8080/v1/chat/completions`; legacy `/completion` endpoints are still supported when passed with `--llm-endpoint`.

AIL package commands operate on package directories such as
`examples/support_ticket.ail`. `ail-check` loads the package entry spec and
runs AIL-Core diagnostics, `ail-core` prints deterministic AIL-Core,
`ail-flow` prints a deterministic AIL-Flow JSON projection for no-code
inspection, `ail-lower` compiles checked AIL-Core IR into deterministic
AIL-Bytecode, and `ail-check`, `ail-core`, `ail-flow`, `ail-lower`, and
`ail-run` can use `--spec-file <path>` to read a saved generated AIL-Spec
artifact instead of the package entry spec. `ail-lower --core-file <path>`
reads a saved checked AIL-Core artifact and compiles it directly to bytecode,
without loading the source package spec, including the serialized edge payloads
used by lowering. `ail-patch
<patch-file>` applies a checked AIL patch and prints canonical AIL-Spec,
`ail-run --action <ActionName>` executes through the current AIL bytecode VM,
`ail-vm --action <ActionName>` verifies and executes a saved AIL-Bytecode
artifact directly, `ail-pass <compiler-pass-package-or-bytecode>
<target-package> --action <PassName>` compiles an AIL-Meta compiler pass
package, or reads a saved Compiler-profile AIL-Bytecode artifact, and applies
it to a checked target package's AIL-Core. `ail-pass
<compiler-pass-package-or-bytecode> --core-file <path> --action <PassName>`
applies the pass to a saved checked AIL-Core artifact without loading the target
source package. `ail-conformance` checks accepted and rejected fixtures,
`ail-requirements --prompt <text>` asks the package base
LLM endpoint for a checked AIL-Requirements artifact and gives the base LLM one
diagnostics-guided repair pass if required coverage is missing, `ail-spec
--requirements-file <path> --prompt <text>` turns a checked AIL-Requirements
artifact into an accepted AIL-Spec candidate with one diagnostics-guided repair
pass, `ail-draft --prompt <text>` asks the package base LLM endpoint for an
AIL-Spec candidate before parsing and checking it, and
`ail-build --prompt <text>` asks the base LLM for requirements, asks it to turn
those requirements into an AIL-Spec candidate for the package profile, gives
the base LLM one diagnostics-guided repair pass if requirements coverage or the
checked spec is incomplete, optionally runs `--pass
<compiler-pass-package-or-bytecode>` over the checked AIL-Core IR, then
compiles the resulting IR into verified AIL-Bytecode on success.
`ail-build --requirements-file <path> --prompt <text>` skips requirements
capture and resumes the build from a saved checked AIL-Requirements artifact
before spec drafting. Add `--artifact-dir <dir>` to also write the captured or
loaded requirements, accepted AIL-Spec, checked AIL-Core IR after any build
pass, and final AIL-Bytecode artifact for review. When `ail-build --pass` is
used with `--artifact-dir`, it also writes `pass.ailbc.json` and
`pass-trace.txt`. On `ail-pass`, `--artifact-dir <dir>` writes
`pass.ailbc.json`, `input.ail-core.txt`, `output.ail-core.txt`, and
`trace.txt` while stdout remains the transformed AIL-Core artifact.
The default AIL base LLM endpoint is
`http://inteligentia-pro-1:8080/v1/chat/completions`.

AIL package metadata can declare imports with `imports: <path> as <Alias>`.
Imported fragments are namespace boundaries, so imported things, actions, and
failures are qualified under the alias before checking and rendering.

`unresolved questions:` sections keep ambiguity explicit in RIF. The checker rejects documents that still contain unresolved questions, which makes clarification a first-class part of the authoring loop instead of a hidden guess.

Run commands through Cargo:

```sh
cargo run -- ail-check examples/support_ticket.ail
cargo run -- ail-core examples/support_ticket.ail
cargo run -- ail-flow examples/support_ticket.ail
cargo run -- ail-lower examples/support_ticket.ail
cargo run -- ail-lower examples/refund_tool.ail
cargo run -- ail-lower examples/compiler_pass.ail
cargo run -- ail-lower examples/network_driver.ail
cargo run -- ail-patch examples/support_ticket.ail examples/support_ticket.ail/examples/patches/escalate-ticket.ail-patch.md
cargo run -- ail-core examples/support_composed.ail
cargo run -- ail-run examples/support_ticket.ail --action CloseTicket ticket.id=T-1 ticket.status=Open
cargo run -- ail-vm /tmp/support-ticket.ailbc.json --action CloseTicket ticket.id=T-1 ticket.status=Open
cargo run -- ail-pass examples/compiler_pass.ail examples/support_ticket.ail --action InferReadPermissions
cargo run -- ail-pass examples/compiler_pass.ail examples/support_ticket.ail --action InferReadPermissions --artifact-dir /tmp/support-ticket-ail-pass
cargo run -- ail-pass /tmp/compiler-pass.ailbc.json examples/support_ticket.ail --action InferReadPermissions
cargo run -- ail-pass /tmp/compiler-pass.ailbc.json --core-file /tmp/support-ticket.ail-core.txt --action InferReadPermissions
cargo run -- ail-conformance examples/support_ticket.ail
cargo run -- ail-conformance examples/ail_toolchain_agent.ail
cargo run -- ail-conformance examples/refund_tool.ail
cargo run -- ail-conformance examples/compiler_pass.ail
cargo run -- ail-conformance examples/network_driver.ail
cargo run -- ail-requirements examples/support_ticket.ail --prompt "Capture requirements for a support ticket app" --llm-endpoint http://inteligentia-pro-1:8080/v1/chat/completions
cargo run -- ail-spec examples/support_ticket.ail --prompt "Draft a support ticket app from captured requirements" --requirements-file /tmp/support-ticket.ail-requirements.md --llm-endpoint http://inteligentia-pro-1:8080/v1/chat/completions
cargo run -- ail-lower examples/support_ticket.ail --spec-file /tmp/support-ticket.ail-spec.md
cargo run -- ail-lower examples/support_ticket.ail --core-file /tmp/support-ticket.ail-core.txt
cargo run -- ail-draft examples/support_ticket.ail --prompt "Draft a support ticket app with private internal notes" --llm-endpoint http://inteligentia-pro-1:8080/v1/chat/completions
cargo run -- ail-build examples/support_ticket.ail --prompt "Build a support ticket bytecode artifact" --llm-endpoint http://inteligentia-pro-1:8080/v1/chat/completions
cargo run -- ail-build examples/support_ticket.ail --prompt "Build a support ticket bytecode artifact from saved requirements" --requirements-file /tmp/support-ticket.ail-requirements.md --llm-endpoint http://inteligentia-pro-1:8080/v1/chat/completions
cargo run -- ail-build examples/support_ticket.ail --prompt "Build a support ticket bytecode artifact" --artifact-dir /tmp/support-ticket-ail-build --llm-endpoint http://inteligentia-pro-1:8080/v1/chat/completions
cargo run -- ail-build examples/support_ticket.ail --prompt "Build a support ticket bytecode artifact" --pass examples/compiler_pass.ail --artifact-dir /tmp/support-ticket-ail-build-pass --llm-endpoint http://inteligentia-pro-1:8080/v1/chat/completions
cargo run -- ail-build examples/refund_tool.ail --prompt "Build a refund tool bytecode artifact" --llm-endpoint http://inteligentia-pro-1:8080/v1/chat/completions
cargo run -- check examples/confirm_order.rif.md
cargo run -- graph examples/confirm_order.rif.md
cargo run -- views examples/confirm_order.rif.md
cargo run -- normalize examples/ticket_api_app.rif.md
cargo run -- patch examples/confirm_order.rif.md examples/confirm_order.retry.patch
cargo run -- patch examples/ticket_api_app.rif.md examples/ticket-domain.patch
cargo run -- check examples/ticket_composed_app.rif.md
cargo run -- llm-roundtrip examples/domain_sentence_app.rsl.md --llm-endpoint http://inteligentia-pro-1:8080/v1/chat/completions
cargo run -- simulate examples/confirm_order.rif.md order.status=Draft order.items.count=2
cargo run -- lower examples/confirm_order.rif.md
cargo run -- run examples/issue_tracker.rif.md issue.status=Open
cargo run -- check examples/issue_tracker_app.rif.md
cargo run -- graph examples/issue_tracker_app.rif.md
cargo run -- run examples/issue_tracker_app.rif.md --intent ReopenIssue issue.status=Resolved
cargo run -- run examples/ticket_routing_app.rif.md ticket.route=Unrouted ticket.priority=Critical
cargo run -- run examples/invoice_app.rif.md invoice.status=Draft invoice.subtotal=40 invoice.tax=2
cargo run -- run examples/compound_invoice_app.rif.md invoice.status=Draft invoice.subtotal=10 invoice.tax=0 invoice.discount=1
cargo run -- run examples/triage_app.rif.md ticket.priority=Normal ticket.status=Open
cargo run -- run examples/feature_flag_app.rif.md feature.enabled=false
cargo run -- run examples/pricing_app.rif.md product.price=19.99 product.discount_rate=0.15
cargo run -- run examples/payment_app.rif.md invoice.total=USD:12.50
cargo run -- run examples/scheduling_app.rif.md job.starts_at=2026-05-20T09:30:00Z job.timeout=PT30M
cargo run -- run examples/profile_app.rif.md user.profile.age=21
cargo run -- run examples/bulk_import_app.rif.md batch.counts=[1,2,3]
cargo run -- run examples/dashboard_app.rif.md 'dashboard.counts={"open":1,"closed":2}'
cargo run -- run examples/optional_profile_app.rif.md user.nickname=None 'user.age=Some(42)'
cargo run -- run examples/result_payment_app.rif.md 'payment.confirmation=Success(200)'
cargo run -- run examples/queue_drain_app.rif.md queue.count=3
cargo run -- run examples/branch_invoice_app.rif.md invoice.total=10
cargo run -- run examples/invoice_workflow_app.rif.md invoice.total=10
cargo run -- run examples/parallel_invoice_app.rif.md invoice.total=10
cargo run -- run examples/order_review_app.rif.md approved=true
cargo run -- run examples/invoice_app.rif.md --state-in examples/invoice.state --state-out /tmp/invoice.state invoice.tax=2
cargo run -- dispatch examples/ticket_api_app.rif.md POST /tickets/INC-99 --state-in examples/ticket.state --data-in examples/ticket.data --state-out /tmp/ticket.state --data-out /tmp/ticket.data id=INC-99 title="Printer is jammed" assignee=Sam auth.bearer_token=demo-token
cargo run -- emit examples/ticket_event_app.rif.md ticket.created --state-in examples/ticket.state --data-in examples/ticket.data --state-out /tmp/ticket.state --data-out /tmp/ticket.data id=INC-99 title="Printer is jammed" assignee=Sam tags[0].name=printer
cargo run -- schedule examples/daily_cleanup_app.rif.md nightly.cleanup --state-in examples/invoice.state --state-out /tmp/cleanup.state job_id=cleanup-1 run_at=2026-05-20T09:30:00Z run_index=1
cargo run -- dequeue examples/queue_inbox_app.rif.md support.inbox --state-in examples/queue.state --state-out /tmp/queue.state message_id=msg-1 message="hello from queue"
cargo run -- serve examples/session_inbox_app.rif.md --listen 127.0.0.1:3000 --state-in examples/session.state
cargo run -- serve examples/ticket_api_app.rif.md --listen 127.0.0.1:3000 --state-in examples/ticket.state --data-in examples/ticket.data
cargo run -- run examples/ticket_maintenance_app.rif.md --data-in examples/ticket.data --data-out /tmp/ticket.data report.closed_count=0
cargo run -- run examples/profile_registry_app.rif.md --state-in examples/profile_registry.state
cargo run -- run examples/confirm_order.rsl.md order.status=Draft order.items.count=2
```

`.rsl.md` files use the compact RSL surface and are elaborated into the same internal RIF model before checking or execution.
RSL also accepts sentence forms in `things:` sections, such as `A Customer has an email address.` and `An Order can be Draft, Confirmed, Cancelled.`

Application documents can declare reusable domain enum types:

```rif
types:
  enum Priority
    value Low
    value Normal
    value Critical

things:
  thing Ticket
    field priority: Priority
```

Enum values can be assigned with `set:`, passed to operation calls, checked by the compiler, and validated in runtime state arguments.

The compiler also treats `true` and `false` as `Bool` literals. Boolean fields accept those values in `set:`, operation calls, predicates, guarantees, and runtime state arguments.

Decimal literals such as `12.50` and `0.25` are typed as `Decimal`. `Int` values can be passed where `Decimal` is expected, decimal values can participate in `compute:` arithmetic, and runtime state values for `Decimal` fields must parse as finite numbers.

Money literals use `CURRENCY:amount`, for example `USD:12.50`. Currency codes must be three uppercase ASCII letters, the amount must parse as a finite number, and money values can participate in `compute:` arithmetic for same-currency addition/subtraction and scalar multiplication/division.

Time literals use UTC timestamp tokens such as `2026-05-20T09:30:00Z`. Duration literals use ISO-8601-style tokens such as `PT30M` or `P1D`.

`thing` fields can reference other `thing` types. Runtime state validation follows nested paths such as `user.profile.age`, so nested scalar values and field typos are checked against the declared domain model.

Applications can declare durable collections in a `collections:` section. Each collection names a prefix and a record type, and the runtime treats keys like `tickets[ticket.id].status` as durable collection paths when they are loaded from `--data-in` or written out through `--data-out`. Whole-record writes are typed object copies, so `set: tickets[ticket.id] = ticket` upserts every declared leaf field for that record.

Collections can also declare `unique:` constraints. Each unique field is checked across stored records, so duplicate values fail fast before execution or persistence.

Steps can delete durable records with `delete:` lines. A delete path removes the exact key and any nested collection fields under that path, so `delete: tickets[ticket.id]` clears the full record for that id.

Collection paths also support simple queries. `tickets.count` returns the number of stored ticket records, `tickets.keys` returns the record ids as comma-delimited text, `tickets.keys_json` returns the record ids as a JSON string array for response bodies, `tickets.records` returns stored records as `List<Ticket>`, and `tickets[id].record` returns the first matching `Ticket` object. Compatibility projections `tickets.records_json` and `tickets[id].record_json` expose the same values as `List<Map<Text, Text>>` and `Map<Text, Text>`; record projections preserve boolean, numeric, `None`/`null`, list, and map JSON values. `tickets[status=Closed].count` filters by a field value before counting or deleting. Field filters can use literals, live values, typed arithmetic/text expressions, or `contains` membership checks with `=`, `==`, `!=`, `>`, `<`, `>=`, `<=`, and `contains`, so application rules can express selectors such as `tickets[priority>=2].count`, `tickets[priority>=report.minimum_priority + 1].count`, and `tickets[tags contains "urgent"].count`. Filters can combine clauses with `and`, such as `tickets[status=Open and priority>=2]`, `tickets[title contains "printer" and tags contains "urgent"]`, or the same selectors with `.records`, `.record`, `.records_json`, and `.record_json` suffixes. The checker validates selector field types and rejects invalid combinations such as ordering a state field against a number.

Steps can also iterate over collection matches with `for each: tickets[status=Closed] as ticket_id`. Inside the step body, the current record id is available under the item name, so the step can update or delete each matching record in turn. Collection record projections iterate as typed objects, so `for each: tickets[status=Open].records as ticket` lets the step read fields such as `ticket.id` and `ticket.title`. The same `for each:` form can iterate over list values, such as `for each: profile.events as event`, where the item name holds each element value, or maps, such as `for each: dashboard.counts as status`, where the item name holds each key.

Predicates in `requires:`, `when:`, and `guarantees:` can combine simple checks with `and`, `or`, `not`, and parentheses. `not` binds tighter than `and`, and `and` binds tighter than `or`. Predicates also support `value exists`, including collection paths such as `tickets[id].id exists`, plus `contains` checks for text substrings, list elements, and map keys such as `account.email contains "@"`, `account.roles contains "admin"`, or `account.flags contains "beta"`. The checker validates reference-shaped operands on both sides of a predicate, so `invoice.status is invoice.expected_status` is checked as a field-to-field comparison while scalar values such as `Draft`, `true`, `12.50`, and quoted strings remain literals. It also rejects invalid typed predicate combinations before runtime, such as ordering a `State` or comparing `Money` with `Decimal`. Runtime comparisons order normalized `Int`/`Decimal` values, same-currency `Money` values, UTC `Time` values, and exact-unit `Duration` values using weeks, days, hours, minutes, or seconds, so guards can express thresholds such as `product.discount_rate < 0.50`, `invoice.total >= USD:20.00`, or `job.timeout <= PT1H`.

Steps can assign typed values with `set:` lines, including literals, references, collection projections, declared `thing` objects, and typed arithmetic/text expressions on the right-hand side. A whole-object assignment such as `set: draft = source` copies the declared leaf fields from one compatible object path to the other, including nested `thing` fields. Steps can also compute integer, decimal, money, and text values with `compute:` lines. A dotted or indexed compute target writes state, such as `compute: invoice.total = invoice.subtotal + invoice.tax`; a bare compute target creates a non-persisted workflow value, such as `compute: line_total = invoice.subtotal + invoice.tax`, that later steps, guards, assignments, appends, and returns can reference. Arithmetic supports `+`, `-`, `*`, `/`, normal multiplication/division precedence, parentheses, and compact operator spelling such as `invoice.subtotal+invoice.tax*invoice.multiplier`; the checker validates every referenced value and accepts numeric arithmetic over `Int` and `Decimal`, same-currency `Money + Money` and `Money - Money`, `Money * Int/Decimal` or `Money / Int/Decimal`, and `Text + Text` concatenation.

List fields use `List<T>` and list literals such as `[1,2,3]`. List literal items can be live expressions, so `set: profile.names = [profile.first_name,profile.last_name]` builds a runtime list from current state. The parser treats commas inside list literals as part of the value, and runtime state validation checks each element against `T`. Application logic can read, replace, or delete one element by zero-based index, such as `profile.events[0]` or `profile.events[profile.index]`; the checker requires an `Int` index and treats the result as `T`, so a step can use `set: profile.events[0] = profile.next_event` or `delete: profile.events[profile.index]`. `profile.events.count` returns the current list length as an `Int`. Steps can append one typed element with `append: profile.events += profile.next_event`, which lets applications grow audit trails, tags, comments, and inbox history without replacing the whole list value. Lists can also hold declared objects, so `set: thread.comments = [comment]` or `append: thread.comments += comment` serializes the current `Comment` fields into a `List<Comment>` value; indexed object fields such as `thread.comments[0].text` can be read in guards, assignments, returns, and endpoint mappings, and `set: thread.comments[0].text = "Reviewed"` updates the object inside the list value.

Generic type arguments can be nested, such as `Option<List<Comment>>`, `Result<Map<Text, Comment>, Text>`, `List<Option<Text>>`, or `Map<Text, Result<Int, Text>>`. Wrapper projections compose with container lookups in either direction, so applications can read and update `profile.tags.value[0]` when an option holds a list, or `profile.aliases[0].value` and `profile.checks["open"].success` when a list or map holds wrappers. Declared object fields compose through the same paths, including updates such as `thread.comments[0].value.text = "Reviewed"` or `thread.directory["primary"].success.text = "Published"`. Container mutations also compose through nested wrappers, so `append: profile.tag_groups[0].value += profile.next_tag` grows a `List<Option<List<Text>>>` entry, and `delete: profile.counts_by_status["current"].success["closed"]` removes a map entry inside a `Map<Text, Result<Map<Text, Int>, Text>>`.

Map fields use `Map<K, V>` and map literals such as `{"open":1,"closed":2}`. Map literal keys and values can be live expressions, so `set: dashboard.counts = {"open":dashboard.open_count,"closed":dashboard.closed_count}` builds a runtime map from current state. The parser treats commas inside map literals as part of the value, and runtime state validation checks each key against `K` and each value against `V`. Application logic can read, update, or delete one value by key with a literal or live key expression, such as `dashboard.counts["open"]` or `dashboard.counts[dashboard.status]`; the checker validates the key type and treats the lookup result as `V`, so a step can use `set: dashboard.counts[dashboard.status] = dashboard.next_count` or `delete: dashboard.counts["closed"]`. `dashboard.counts.count` returns the number of entries as an `Int`. Maps can also hold declared objects, so indexed object fields such as `directory.comments["primary"].text` can be read in guards, assignments, returns, and endpoint mappings, and `set: directory.comments["primary"].text = "Reviewed"` updates the object stored at that map key.

Optional fields use `Option<T>` with `Some(value)` and `None` literals. `Some(...)` validates its inner value against `T`, while `None` is accepted for any `Option<T>`. Application logic can read a present optional value through `.value`, such as `user.nickname.value`, and the checker treats it as `T` for guards, assignments, returns, and endpoint mappings. Setting `.value`, such as `set: user.nickname.value = "Grace"`, rewrites the option as `Some("Grace")`. If the option payload is a declared object, projected fields such as `article.review.value.text` and nested fields such as `article.review.value.details.note` can also be read and updated while rewriting the enclosing option value. If the payload is a list or map, indexed entries such as `profile.tags.value[0]` can be updated or deleted, and wrapper-held lists can append with `append: profile.tags.value += profile.next_tag`; lists or maps of declared objects also allow indexed fields such as `thread.comments.value[0].text`.

Secret values use `Secret<T>` for request inputs, operation arguments, and runtime state that must keep sensitive values explicit in the type system. Runtime validation still checks the wrapped `T`, so `Secret<Int>` accepts `123` and rejects `abc`, while the checker keeps `Secret<T>` distinct from plain `T`. Runtime state output redacts secret-typed paths as `<secret>`, and endpoint responses may not map secret values.

Fallible values use `Result<T, E>` with `Success(value)` and `Failure(value)` literals. Runtime state validation checks `Success(...)` against `T` and `Failure(...)` against `E`. Application logic can read a successful payload through `.success`, such as `payment.confirmation.success`, or a failed payload through `.failure`, such as `payment.rejection.failure`; the checker treats those projections as `T` and `E` for guards, assignments, returns, and endpoint mappings. Setting those projections rewrites the result as `Success(...)` or `Failure(...)`. If either payload is a declared object, projected fields such as `payment.confirmation.success.status` and nested fields such as `payment.confirmation.success.details.note` can also be read and updated while rewriting the enclosing result value. If a payload is a list or map, indexed entries such as `profile.counts.success["open"]` can be updated or deleted, and wrapper-held lists can append with `append: profile.audit.success += profile.next_event`; lists or maps of declared objects also allow indexed fields such as `thread.directory.success["primary"].text`.

Steps can also repeat with `repeat while:` or `repeat until:`. The loop condition is re-evaluated after each execution of the step, which makes draining and retry workflows possible without inventing a separate runtime model.

Steps can branch with `otherwise:`. When a `when:` guard fails, the step's `otherwise` effect runs instead of the primary effect.

Steps can also invoke other intents in the same application with `invoke:` and `otherwise invoke:`. Invocation bindings use `name = expression` inside the call, so one workflow can pass values into another without matching field names exactly. A child workflow subject or input must either be bound explicitly or be available in the caller under the same name with a compatible type. When a binding maps one subject object to another, nested state fields are remapped into the child workflow and projected back to the caller when the child succeeds.

Steps can fan out to multiple intents with `parallel invoke:` and `otherwise parallel invoke:`. Each branch runs from the same state snapshot, and the runtime rejects conflicting writes during the join.

Operations can accept typed expression arguments in `call:` lines. They can return one anonymous value with `operation Name(...) -> Type`, which a step may store under any local `output:` name. Operations can also declare named output contracts with indented `output: name: Type` lines; callers must store those returned names with matching types, and the checker reports duplicate, missing, unknown, or mismatched output declarations before runtime. Step output names must be unique within an intent so later references and supplied `--operation-output name=value` values are unambiguous. Local execution can supply deterministic external operation results with repeated `--operation-output name=value` flags; supplied values are validated against the step output type and are used wherever that output name is read, while omitted outputs keep the existing placeholder value.

Intent names must be unique in an application so invocations, endpoints, triggers, and CLI selection resolve one workflow. Intents can publish explicit result aliases with `returns:`. Each alias names a source value from the final state, a step output, a typed literal, a declared `thing` object, or a typed arithmetic/text expression, so reusable workflows can expose a stable public result without leaking every internal field. Return alias names must be unique within the intent. A whole-object alias such as `result: ticket` renders the declared fields as a JSON object, which endpoint response contracts can return as a typed object.

The CLI can load initial runtime state from `--state-in <file>` and write the final state back with `--state-out <file>`. State files use the same `key=value` line format as command-line runtime assignments.

Durable collection data uses the same line format, but is loaded with `--data-in <file>` and written back with `--data-out <file>`.

Applications can define request routes in an `endpoints:` section. A route names an HTTP method, a path, and a target intent, can declare typed request fields with a `request:` block, can add `requires:` guards for request-level access control, and maps incoming request values into typed state paths with `bind:` lines. Endpoint method/path pairs must be unique in the application, with methods compared case-insensitively. When a `request:` block is present, binding sources must come from declared request fields, path parameters, framework request values such as `auth.*`, `headers.*`, `cookies.*`, and `query.*`, known runtime state expressions, or typed arithmetic/text expressions composed from those sources; the checker reports undeclared request fields and type mismatches before runtime. Query parameters can be declared and validated as typed request fields such as `query.status: State<Open, Closed>`, then used in guards, bindings, and responses as `query.status`. Request fields can also use declared `thing` types, so a JSON object request value such as `ticket: Ticket` is validated against the `Ticket` fields and `bind: ticket = ticket` expands it into `ticket.id`, `ticket.title`, and the other declared state fields; `dispatch` and form-style callers can pass the same object as one aggregate request value such as `ticket={"id":"T-1","title":"Printer"}`. Durable collection records can be loaded the same way, so `bind: ticket = tickets[id]` copies the selected record into the target intent subject. The `dispatch` command resolves a route, validates declared request fields, applies the bindings, and runs the target intent against the current state. Missing or invalid declared request fields fail with `BadRequest`, which can be shaped with an `error BadRequest:` response block.

Route paths can use `{name}` segments. Those segments bind to the matching request path parts, so `POST /tickets/{id}` can capture `id` from `/tickets/INC-99` and feed it into the target intent like any other request value.

Endpoints can also declare typed `respond:` contracts. A `name: Type` line declares a response field, and a `name = expression` line maps it to a final state value, intent return alias, literal, path parameter, collection projection, typed object, or typed arithmetic/text expression; the checker reports unknown response types, unmapped declared fields, unknown sources, and type mismatches. `dispatch` prints `response.<name>=...` lines and `serve` returns a JSON object for successful requests with an explicit response contract. Boolean, numeric, `None`/`null`, list, map, and declared `thing` response values are emitted as JSON values rather than forced strings, so a response field such as `ticket: Ticket` with `ticket = ticket` renders the declared `Ticket` fields as a JSON object. Secret-flow checks follow declared object fields, so an object that contains a `Secret<T>` field cannot be mapped into an endpoint response. Collection endpoints can expose collection ids with `ids = tickets.keys_json`, typed stored objects with `items: List<Ticket>` and `items = tickets.records`, and detail endpoints with `item: Ticket` and `item = tickets[id].record`; the `_json` projections remain available for raw map-shaped contracts. A `status: 201 Created` line inside `respond:` sets the successful HTTP response status; endpoints without one still return `200 OK` on success. Endpoints can add typed default `error:` and named `error FailureName:` contracts with their own `status:`, `name: Type` fields, and `name = expression` mappings; the same checker rules catch unknown error field types, unmapped fields, unknown sources, and type mismatches. The special `failure` source is typed as `Text` and exposes the runtime failure name for structured error payloads. A route requirement can use `else FailureName`, and a matching `error FailureName:` block can provide a failure-specific status and body, for example `tickets[id].id exists else NotFound` with `error NotFound:` returning `404 Not Found`.

The `serve` command starts a small HTTP server that uses the same endpoint definitions and dispatch logic as the CLI route path. It accepts form-encoded bodies as `key=value` pairs and also accepts JSON request bodies for simple API clients. JSON objects flatten into dotted request paths, JSON arrays flatten into indexed paths such as `tags[0]` or `tags[0].name`, and aggregate JSON arrays/objects are also available as typed request values such as `tags: List<Text>` or `metadata: Map<Text, Text>`. Query parameters are exposed as `query.<name>` request values, HTTP headers are exposed as `headers.<name>` bindings like `headers.x_request_id`, and `Authorization: Bearer ...` is also exposed as `auth.bearer_token` for endpoint guards. Cookie headers are also parsed, so `Cookie: session=...` becomes `cookies.session` and `auth.session_id`.

This lets the same endpoint model support both API tokens and browser-style session cookies without changing the application logic model.

Applications can also define non-HTTP ingress in a `triggers:` section. Triggers name an external event, can declare typed payload fields with a `payload:` block, can add `requires:` guards, and map payload values into typed state paths with `bind:` lines. Trigger names must be unique in the application. Binding sources can be payload fields, `event.*` metadata, known runtime state expressions, or typed arithmetic/text expressions composed from those sources. The checker validates declared payload types, rejects undeclared payload binding sources, and reports payload-to-target type mismatches. The `emit` command resolves a trigger, validates declared payload fields, applies the bindings, and runs the target intent against the current state. Missing or invalid declared payload fields fail with `BadEvent`.

Trigger payload values are exposed as `event.<name>` bindings, and the trigger name is always available as `event.name`. This keeps queue messages, pub/sub events, and other external inputs in the same guard and binding model as endpoints without forcing them through HTTP.

Triggers can also carry `schedule:` metadata for periodic jobs. The `schedule` command resolves the same trigger path, exposes `event.schedule`, and runs the target intent against the current state. This is the first slice of a time-based job surface rather than an external scheduler integration. Use a single-token schedule label such as `PT24H` so it can be compared in guards like `event.schedule is PT24H`.

Triggers can also carry `queue:` metadata for queue-driven workers. The `dequeue` command resolves the same trigger path, exposes `event.queue`, `event.message_id`, and `event.message`, and runs the target intent against the current state. This is the queue-backed worker slice of the ingress model.
