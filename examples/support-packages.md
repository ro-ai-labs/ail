# AIL Support Packages

This manifest declares top-level `examples/*.ail` packages that are intentionally
not counted as direct entries in `examples/examples.md`. Each package remains
part of the end-to-end corpus because it is imported by counted packages or
used by executable toolchain, manual, or regression checks.

## Support Package: examples/ail_std_core.ail
role: support-only
used-by: examples/ail_std_collections.ail, test:cli_ail_stdlib_packages_have_checked_package_artifacts
reason: Standard-library primitive contracts are imported by counted collection examples and covered by stdlib artifact checks.

## Support Package: examples/ail_std_csv.ail
role: support-only
used-by: docs:examples/csv-aggregate/README.md, test:scripts/ail_csv_differential.py, test:bench/R022-csv-aggregate
reason: Pure AIL quoted CSV parsing and checked category aggregation are imported by a runnable JSON-lines/file application and verified by independent VM/native acceptance and capability rung R022.

## Support Package: examples/ail_std_effects.ail
role: support-only
used-by: examples/ail_std_runtime.ail, test:cli_ail_stdlib_import_records_dependency_report, test:cli_ail_std_rejects_missing_capability_grant
reason: Standard-library effect contracts are imported by runtime support fixtures and checked through dependency and conformance tests.

## Support Package: examples/ail_std_expression.ail
role: support-only
used-by: docs:examples/expression-calculator/README.md, test:scripts/ail_calculator_differential.py, test:bench/R026-expression-calculator
reason: Pure AIL tokenization, parsing and expression evaluation are imported by a runnable JSON-lines calculator and verified by independent VM/native comparisons and the capability ladder.

## Support Package: examples/ail_std_rle.ail
role: support-only
used-by: docs:examples/run-length-codec/README.md, test:scripts/ail_rle_differential.py, test:bench/R027-run-length-codec
reason: Pure AIL binary run-length encoding, decoding and size preflight are imported by a file tool and verified through independent VM/native comparisons and capability rung R027.

## Support Package: examples/ail_std_runtime.ail
role: support-only
used-by: test:cli_ail_stdlib_import_records_dependency_report, test:cli_ail_std_rejects_missing_capability_grant
reason: Runtime task and capability fixtures are checked by stdlib regression tests rather than replayed as direct catalog packages.

## Support Package: examples/ail_std_security.ail
role: support-only
used-by: test:cli_ail_stdlib_packages_have_checked_package_artifacts, docs:docs/ail/20-standard-library-and-packages.md
reason: Reusable Secret, permission, and capability semantics are kept as stdlib fixture source with checked artifact coverage.

## Support Package: examples/ail_toolchain_agent.ail
role: support-only
used-by: examples/support_ticket.ail, examples/compiler_pass.ail, toolchain:ail-build, toolchain:ail-story, toolchain:ail-bootstrap
reason: The AIL-authored toolchain agent participates in build, story, bootstrap, target verification, and prompt-portability flows instead of acting as a replay target package.

## Support Package: examples/ail_std_tree.ail
role: support-only
used-by: docs:examples/binary-tree/README.md, test:scripts/ail_value_type_differential.py, test:bench/R019-binary-tree
reason: Pure AIL recursive binary-tree insertion, traversal, search and height demonstrate executable custom variants through independent VM/native comparisons and capability rung R019.

## Support Package: examples/incident_identity.ail
role: support-only
used-by: examples/incident_response.ail
reason: Incident identity users, roles, teams, and contact details are imported by counted incident-response multi-module examples.

## Support Package: examples/incident_policy.ail
role: support-only
used-by: examples/incident_response.ail
reason: Incident severity and escalation-policy definitions are imported by counted incident-response workflow examples.

## Support Package: examples/incident_notifications.ail
role: support-only
used-by: examples/incident_response.ail
reason: AgentTool notification contracts and pager-token boundaries are imported by counted incident-response workflow examples.

## Support Package: examples/support_shared.ail
role: support-only
used-by: examples/support_composed.ail
reason: Shared support-domain user declarations are imported by counted support-composed examples and replayed through package composition evidence.

## Support Package: examples/ail_std_words.ail
role: support-only
used-by: docs:examples/word-frequency/README.md, test:scripts/ail_word_frequency_differential.py, test:bench/R021-word-frequency
reason: Pure AIL exact word tokenization and deterministic frequency summaries are imported by a bounded raw-stdin application and verified by independent VM/native acceptance and capability rung R021.

## Support Package: examples/ail_std_json_pretty.ail
role: support-only
used-by: docs:examples/json-pretty/README.md, test:scripts/ail_json_pretty_differential.py, test:bench/R023-json-pretty
reason: Pure AIL lossless JSON layout preserves ordered duplicate pairs and number tokens through a bounded raw-stdin application and independent VM/native acceptance for capability rung R023.

## Support Package: examples/ail_std_lru.ail
role: support-only
used-by: docs:examples/lru-cache/README.md, test:scripts/ail_lru_differential.py, test:bench/R024-lru-cache
reason: Pure AIL immutable least-recently-used cache APIs are imported by a bounded raw-stdin request tool and independently checked on VM/native for capability rung R024.
