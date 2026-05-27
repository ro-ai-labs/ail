# AIL Support Packages

This manifest declares top-level `examples/*.ail` packages that are intentionally
not counted as direct entries in `examples/examples.md`. Each package remains
part of the end-to-end corpus because it is imported by counted packages or
used by executable toolchain, manual, or regression checks.

## Support Package: examples/ail_std_core.ail
role: support-only
used-by: examples/ail_std_collections.ail, test:cli_ail_stdlib_packages_have_checked_package_artifacts
reason: Standard-library primitive contracts are imported by counted collection examples and covered by stdlib artifact checks.

## Support Package: examples/ail_std_effects.ail
role: support-only
used-by: examples/ail_std_runtime.ail, test:cli_ail_stdlib_import_records_dependency_report, test:cli_ail_std_rejects_missing_capability_grant
reason: Standard-library effect contracts are imported by runtime support fixtures and checked through dependency and conformance tests.

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

## Support Package: examples/recursive_factorial.ail
role: support-only
used-by: test:ail_spec_parses_function_surface_into_core_and_round_trips, test:ail_spec_lowers_function_surface_into_runnable_bytecode, docs:docs/ail/README.md
reason: Recursive function semantics are currently verified by focused parser, Core round-trip, bytecode, and VM tests before being promoted into catalog replay entries.

## Support Package: examples/support_shared.ail
role: support-only
used-by: examples/support_composed.ail
reason: Shared support-domain user declarations are imported by counted support-composed examples and replayed through package composition evidence.
