#!/usr/bin/env python3
"""Independent structural edit admission and observable execution oracle.

Python constructs trusted source pairs and computes expected results without any
AIL helper. A generated Rust probe checks public comparison/admission APIs and
in-memory transformations against the exact library artifact emitted by the
current Cargo build. Every fixture, patch and executable is temporary; no old
script, benchmark input, external service or /tmp fixture is required.

Usage: python3 scripts/ail_edit_differential.py [--artifact-dir DIR]
Requires Python 3, Cargo/rustc and Linux x86_64 for native ELF execution.
"""
from __future__ import annotations

import argparse
from collections import Counter
import hashlib
import json
import platform
from pathlib import Path
import random
import re
import subprocess
import tempfile
import time

ROOT = Path(__file__).resolve().parent.parent
SEED = 0xA11ED175
HEAD = 'The application EditOracle manages values.\nA Data has:\n- value: Int\n'
MANIFEST = ('name: edit-oracle\nversion: 0.1.0\nprofile: Application\n'
            'entry: spec.ail-spec.md\nfeatures: things, types, actions, functions, expressions, literals, traces\n'
            'conformance: first-slice\nbase_llm_endpoint: http://127.0.0.1:1\n')

# This probe observes APIs under test; it never computes Python's expected
# arithmetic/literal/function/variant/call results. All assertions remain active
# under python -O because Python's own require() also raises explicitly.
PROBE = r'''
use ail::ail::*;
use ail::core_model::{json_string, Node};
use std::{collections::BTreeMap, env, fs, path::Path};

fn checked(path: &str) -> AilCore {
    let package = load_ail_package_dir(path).unwrap();
    let doc = parse_ail_package_document(&package).unwrap();
    let core = elaborate_ail_core(&package, &doc);
    assert!(check_ail_core(&core).is_empty(), "{}: {:?}", path, check_ail_core(&core));
    core
}
fn compare(base: &AilCore, candidate: &AilCore, policy: &str) {
    let diff = compare_ail_core_semantics(base, candidate);
    let expected = match policy { "equal" => (true, true), "add" => (false, true), _ => (false, false) };
    assert!(diff.invalid.is_empty(), "{}", render_ail_semantic_diff(&diff));
    assert_eq!((diff.is_equivalent(), diff.preserves_baseline()), expected, "{}", render_ail_semantic_diff(&diff));
    assert_eq!(AilEditContext::preserve_core(base).unwrap().check_candidate(candidate).is_empty(), expected.1);
    if policy == "add" {
        assert_eq!(diff.lowering_validation, AilSemanticLoweringValidation::ExistingRecordsPreserved);
    }
}
fn label(node: &Node) -> String { format!("{}:{}", node.kind, node.name) }
fn attrs(attributes: &BTreeMap<String, String>) -> String {
    format!("{{{}}}", attributes.iter().map(|(k,v)| format!("{}:{}", json_string(k), json_string(v))).collect::<Vec<_>>().join(","))
}
// Independent fixture-authority encoder: rebuild a separately authored trusted
// target using the existing patch schema. It does not use the semantic diff to
// decide expected edits. Unsupported repeated edge selectors are rejected here.
fn trusted_patch(base: &AilCore, target: &AilCore) -> String {
    assert_eq!(base.package, target.package, "metadata cannot be patched by v0");
    let base_labels: BTreeMap<_,_> = base.graph.nodes.iter().map(|n| (n.id.clone(), label(n))).collect();
    let target_labels: BTreeMap<_,_> = target.graph.nodes.iter().map(|n| (n.id.clone(), label(n))).collect();
    let mut ops = vec![];
    for edge in &base.graph.edges {
        ops.push(format!(r#"{{"op":"remove_edge","kind":{},"source":{},"target":{}}}"#,
            json_string(&edge.kind), json_string(&base_labels[&edge.source]), json_string(&base_labels[&edge.target])));
    }
    for node in &base.graph.nodes {
        ops.push(format!(r#"{{"op":"remove_node","target":{}}}"#, json_string(&label(node))));
    }
    for node in &target.graph.nodes {
        let ty = node.type_name.as_ref().map(|t| format!(",\"type\":{}", json_string(t))).unwrap_or_default();
        ops.push(format!(r#"{{"op":"add_node","kind":{},"name":{},"attributes":{}{}}}"#,
            json_string(&node.kind), json_string(&node.name), attrs(&node.attributes), ty));
    }
    let mut edges = target.graph.edges.iter().collect::<Vec<_>>();
    edges.sort_by_key(|e| e.ordinal);
    let mut selectors = std::collections::BTreeSet::new();
    for edge in edges {
        assert!(selectors.insert((&edge.kind, &edge.source, &edge.target)), "v0 has no occurrence selector");
        let mut attributes = edge.attributes.clone();
        if edge.kind == "writes" {
            if let Some(reference) = attributes.get_mut("expression") {
                if let Some(mapped) = target_labels.get(reference) { *reference = mapped.clone(); }
            }
        }
        ops.push(format!(r#"{{"op":"add_edge","kind":{},"source":{},"target":{},"attributes":{}}}"#,
            json_string(&edge.kind), json_string(&target_labels[&edge.source]), json_string(&target_labels[&edge.target]), attrs(&attributes)));
    }
    format!(r#"{{"schema":"ail-core.patch.v0","package":{},"base_hash":{},"ops":[{}]}}"#,
        json_string(&base.package.name), json_string(&ail_core_hash(base)), ops.join(","))
}
fn main() {
    let args: Vec<_> = env::args().collect();
    let base = checked(&args[2]);
    let directory = Path::new(&args[4]);
    match args[1].as_str() {
        "pair" => {
            let candidate = checked(&args[3]);
            compare(&base, &candidate, &args[5]);
            if args[6] == "authorize" {
                let patch = trusted_patch(&base, &candidate);
                let context = AilEditContext::expected_core_patch(&base, &patch).unwrap();
                let target = apply_ail_core_patch_text(&base, &patch).unwrap();
                assert!(compare_ail_core_semantics(&candidate, &target).is_equivalent());
                assert!(context.check_candidate(&candidate).is_empty());
                assert!(context.check_candidate(&target).is_empty());
                assert!(!context.check_candidate(&base).is_empty());
                let mut unrelated = target.clone();
                unrelated.graph.nodes.iter_mut().find(|n| n.kind == "Action").unwrap().attributes.insert("unrequested".into(), "extra".into());
                assert!(!context.check_candidate(&unrelated).is_empty());
                let mut addition = target.clone();
                let added = addition.graph.add_node("Type", "Unrequested", None, BTreeMap::new());
                let evidence = addition.graph.add_node("Provenance", "type:Unrequested", None, BTreeMap::new());
                addition.graph.add_edge("has_provenance", &added, &evidence, BTreeMap::new());
                assert!(check_ail_core(&addition).is_empty());
                assert!(!context.check_candidate(&addition).is_empty());
                assert!(AilEditContext::expected_core_patch(&base, &patch.replace(&ail_core_hash(&base), "stale")).is_err());
                fs::write(directory.join("expected.patch.json"), patch).unwrap();
                fs::write(directory.join("expected.core"), render_ail_core(&target)).unwrap();
            }
        }
        "memory" => {
            let mut transformed = base.clone();
            let mut ids = BTreeMap::new();
            for (i, node) in transformed.graph.nodes.iter_mut().enumerate() {
                ids.insert(node.id.clone(), format!("opaque-{i}")); node.id = format!("opaque-{i}");
            }
            for (i, edge) in transformed.graph.edges.iter_mut().enumerate() {
                edge.id = format!("opaque-edge-{i}");
                edge.source = ids[&edge.source].clone(); edge.target = ids[&edge.target].clone();
                edge.ordinal = edge.ordinal * 3 + 11; // legal gaps/global shifts
                if edge.kind == "writes" {
                    if let Some(reference) = edge.attributes.get_mut("expression") {
                        if let Some(mapped) = ids.get(reference) { *reference = mapped.clone(); }
                    }
                }
            }
            transformed.graph.nodes.reverse(); transformed.graph.edges.reverse();
            compare(&base, &transformed, "equal");
            assert_eq!(ail_document_from_core(&base), ail_document_from_core(&transformed));
            assert_eq!(compile_ail_core_ste(&base).unwrap(), compile_ail_core_ste(&transformed).unwrap());
            assert_eq!(compile_ail_core_bytecode(&base).unwrap(), compile_ail_core_bytecode(&transformed).unwrap());
            for node in &mut transformed.graph.nodes {
                if node.kind == "Provenance" { node.name.push_str(" independent review"); }
            }
            compare(&base, &transformed, "equal");
            assert_eq!(compile_ail_core_ste(&base).unwrap(), compile_ail_core_ste(&transformed).unwrap());
            assert_eq!(compile_ail_core_bytecode(&base).unwrap(), compile_ail_core_bytecode(&transformed).unwrap());
            fs::write(directory.join("neutral.core"), render_ail_core(&transformed)).unwrap();
            // Native emission consumes the actual in-memory transformed graph.
            let elf = directory.join("neutral-memory.elf");
            fs::write(&elf, compile_ail_core_native_elf(&transformed, "Demo", "linux-x86_64-elf").unwrap()).unwrap();
            use std::os::unix::fs::PermissionsExt;
            fs::set_permissions(elf, fs::Permissions::from_mode(0o755)).unwrap();
            // Exercise legacy ordinal fallback independently of explicit steps.
            let calls = checked(&args[3]);
            let mut legacy = calls.clone();
            for edge in &mut legacy.graph.edges {
                if edge.kind == "calls" { edge.attributes.remove("step_index"); }
            }
            let mut reversed = legacy.clone(); reversed.graph.edges.reverse();
            compare(&legacy, &reversed, "equal");
            assert_eq!(compile_ail_core_bytecode(&legacy).unwrap(), compile_ail_core_bytecode(&reversed).unwrap());
            fs::write(directory.join("legacy.core"), render_ail_core(&reversed)).unwrap();
            let mut invalids = vec![];
            let mut broken = base.clone(); broken.graph.nodes.push(broken.graph.nodes[0].clone()); invalids.push(broken);
            let mut broken = base.clone(); let mut duplicate = broken.graph.nodes[0].clone(); duplicate.id.push_str("-other"); broken.graph.nodes.push(duplicate); invalids.push(broken);
            let mut broken = base.clone(); broken.graph.edges[0].source = "missing".into(); invalids.push(broken);
            let mut broken = base.clone(); broken.graph.edges[1].ordinal = broken.graph.edges[0].ordinal; invalids.push(broken);
            for broken in invalids {
                let diff = compare_ail_core_semantics(&broken, &broken);
                assert!(!diff.invalid.is_empty()); assert!(!diff.is_equivalent()); assert!(!diff.preserves_baseline());
                assert!(AilEditContext::preserve_core(&broken).is_err());
            }
            for edge_attribute in [false, true] {
                let mut changed = base.clone();
                if edge_attribute { changed.graph.edges.iter_mut().find(|e| e.kind == "returns_field").unwrap().attributes.insert("provenance".into(), "exact policy".into()); }
                else { changed.graph.nodes.iter_mut().find(|n| n.kind == "Action").unwrap().attributes.insert("review_note".into(), "exact policy".into()); }
                compare(&base, &changed, "reject");
            }
            let mut fallback = checked(&args[6]);
            let write = fallback.graph.edges.iter_mut().find(|e| e.kind == "writes").unwrap();
            write.attributes.remove("provenance"); let target_id = write.target.clone();
            fallback.graph.nodes.iter_mut().find(|n| n.id == target_id).unwrap().attributes.remove("provenance");
            fallback.graph.edges.retain(|e| !(e.kind == "has_provenance" && e.source == target_id));
            let owner = fallback.graph.nodes.iter().find(|n| n.kind == "Action").unwrap().clone();
            let target = fallback.graph.nodes.iter().find(|n| n.id == target_id).unwrap().clone();
            let evidence = fallback.graph.add_node("Provenance", format!("action:{}.write:changes the data value to 7", owner.name), None, BTreeMap::new());
            fallback.graph.add_edge("has_provenance", &target, &evidence, BTreeMap::new());
            let mut changed = fallback.clone();
            changed.graph.nodes.iter_mut().find(|n| n.id == evidence.id).unwrap().name = format!("action:{}.write:changes the data value to 8", owner.name);
            compare(&fallback, &changed, "reject");
            fs::write(directory.join("fallback-base.core"), render_ail_core(&fallback)).unwrap();
            fs::write(directory.join("fallback-changed.core"), render_ail_core(&changed)).unwrap();
            let text = render_ail_spec_from_core(&base);
            let package = load_ail_package_dir(&args[2]).unwrap();
            let roundtrip = elaborate_ail_core(&package, &parse_ail_package_spec_text(&package, &text).unwrap());
            assert_eq!(compile_ail_core_bytecode(&base).unwrap(), compile_ail_core_bytecode(&roundtrip).unwrap());
            compare(&base, &roundtrip, "reject"); // parentheses change retained policy prose
        }
        _ => panic!("unknown mode"),
    }
    println!("probe {} passed", args[1]);
}
'''


def require(condition: bool, detail: object) -> None:
    if not condition:
        raise RuntimeError(str(detail))


class Oracle:
    def __init__(self, workspace: Path):
        self.workspace = workspace
        self.commands: list[dict] = []
        self.counts: Counter = Counter()
        self.records: list[dict] = []

    def run(self, arguments, success=True, timeout=60):
        command = list(map(str, arguments))
        result = subprocess.run(command, cwd=ROOT, capture_output=True, text=True, timeout=timeout)
        self.commands.append({'command': command, 'returncode': result.returncode})
        require((result.returncode == 0) == success,
                (command, result.returncode, result.stdout[-2000:], result.stderr[-3000:]))
        return result

    def package(self, directory: Path, source: str, metadata='') -> Path:
        directory.mkdir(parents=True)
        (directory / 'ail-package.md').write_text(MANIFEST + metadata)
        (directory / 'spec.ail-spec.md').write_text(source)
        return directory

    def execute(self, frontend, directory, expected, seeds=(), trace=None, failed=False):
        directory.mkdir(parents=True, exist_ok=True)
        bytecode = directory / 'program.ailbc.json'
        bytecode.write_text(self.run([self.ail, 'ail-lower', *frontend]).stdout)
        native = directory / 'program.elf'
        self.run([self.ail, 'ail-compile', *frontend, '--action', 'Demo',
                  '--target', 'linux-x86_64-elf', '--out', native])
        for backend, command in [('VM', [self.ail, 'ail-vm', bytecode, '--action', 'Demo']),
                                 ('native', [native])]:
            result = self.run([*command, *seeds], success=not failed)
            output = result.stdout + result.stderr
            # Native legacy unrolled calls print intermediates; the last value
            # is the final state. Expected values are authored by Python.
            state = dict(line.split('=', 1) for line in output.split('\n')
                         if line.startswith('data.value='))
            if failed:
                require(re.search(r'^failure[= ]RequirementFailed$', output, re.MULTILINE)
                        and state == {}, (backend, output))
            else:
                require(state == {'data.value': expected}, (backend, expected, state, output))
                if trace:
                    require(re.search(r'\btrace ' + re.escape(trace) + r'(?= -> |\n|$)', output),
                            (backend, trace, output))
            self.counts['runtime_checks'] += 1
            self.records.append({'route': str(directory.relative_to(self.workspace)), 'backend': backend,
                                 'expected': expected, 'failed': failed, 'trace': trace})

    def routes(self, package, directory, expected, **kwargs):
        core = directory / 'saved.core'
        core.write_text(self.run([self.ail, 'ail-core', package]).stdout)
        ste = directory / 'saved.ail-ste'
        ste.write_text(self.run([self.ail, 'ail-ste', package]).stdout)
        for form, args in [('source', [package]), ('core', ['--core-file', core]), ('ste', [ste])]:
            self.execute(args, directory / form, expected, **kwargs)
        return core

    def build(self):
        result = self.run(['cargo', 'build', '--lib', '--bin', 'ail', '--message-format=json'], timeout=180)
        artifacts = [json.loads(line) for line in result.stdout.splitlines() if line.startswith('{')]
        artifacts = [a for a in artifacts if a.get('reason') == 'compiler-artifact']
        lib = [Path(f) for a in artifacts if a['target']['name'] == 'ail' and 'lib' in a['target']['kind']
               for f in a['filenames'] if f.endswith('.rlib')]
        bins = [Path(a['executable']) for a in artifacts if a['target']['name'] == 'ail' and a.get('executable')]
        require(len(lib) == len(bins) == 1, ('exact Cargo artifact selection', lib, bins))
        self.ail = bins[0]
        self.library = lib[0]
        source = self.workspace / 'probe.rs'
        source.write_text(PROBE)
        self.probe = self.workspace / 'probe'
        self.run(['rustc', '--edition=2024', source, '--extern', f'ail={self.library}',
                  '-L', f'dependency={self.library.parent}', '-o', self.probe])

    def pair(self, name, base, candidate, before, after, policy='reject', authorize=True,
             seeds=(), trace_before='Done', trace_after='Done', before_failed=False,
             base_metadata='', candidate_metadata=''):
        directory = self.workspace / name
        directory.mkdir()
        original = self.package(directory / 'baseline', base, base_metadata)
        changed = self.package(directory / 'candidate', candidate, candidate_metadata)
        self.run([self.probe, 'pair', original, changed, directory, policy,
                  'authorize' if authorize else 'skip'])
        self.counts[policy + '_pairs'] += 1
        for preservation in [False, True]:
            result = self.run([self.ail, 'ail-equivalence', changed, '--baseline', original,
                               *(['--preserve-baseline'] if preservation else [])],
                              success=(policy == 'equal' or policy == 'add' and preservation))
            require(f'semantically-equivalent {str(policy == "equal").lower()}' in result.stdout
                    and f'preserves-baseline {str(policy in ("equal", "add")).lower()}' in result.stdout,
                    ('equivalence/preservation report', result.stdout))
            self.counts['cli_comparisons'] += 1
        baseline_core = self.routes(original, directory, before, seeds=seeds,
                                    trace=trace_before, failed=before_failed)
        candidate_dir = directory / 'candidate-routes'
        candidate_dir.mkdir()
        candidate_core = self.routes(changed, candidate_dir, after, seeds=seeds, trace=trace_after)
        # Prove advertised saved-Core candidate dispatch too.
        self.run([self.ail, 'ail-equivalence', '--core-file', candidate_core,
                  '--baseline', baseline_core, '--preserve-baseline'], success=policy in ('equal', 'add'))
        self.counts['cli_comparisons'] += 1
        if authorize:
            self.execute(['--core-file', directory / 'expected.core'], directory / 'approved-target',
                         after, seeds=seeds, trace=trace_after)
            self.counts['authorized_targets'] += 1
        print(f'{name}: {policy}, independent runtime outcomes verified', flush=True)
        return original, changed, baseline_core, candidate_core


def demo(effect: str, head=HEAD) -> str:
    return head + ('Action: Demo.\nWhen Demo happens:\n- the system ' + effect + '\n'
                   '- the system returns data.value\n- the system records a trace event named Done\n')


def generated_cases(oracle: Oracle):
    rng = random.Random(SEED)
    arithmetic = []
    for index in range(6):
        left, right = rng.randrange(11, 61), rng.randrange(2, 10)
        arithmetic.append(oracle.pair(f'arithmetic-{index}', demo(f'computes data.value as {left} + {right}'),
                                      demo(f'computes data.value as {left} - {right}'),
                                      str(left + right), str(left - right)))
    left, right = rng.sample(range(70, 150), 2)
    oracle.pair('integer-literal', demo(f'assigns {left} to data.value'),
                demo(f'assigns {right} to data.value'), str(left), str(right))
    for family, before, after in [('text-literal', '"old λ"', '"new Ω"'),
                                  ('list-literal', '[2,5,8]', '[2,6,8]')]:
        ty = 'Text' if family.startswith('text') else 'List<Int>'
        head = HEAD.replace('Int', ty)
        oracle.pair(family, demo(f'assigns {before} to data.value', head),
                    demo(f'assigns {after} to data.value', head), (json.loads(before) if ty == 'Text' else before),
                    (json.loads(after) if ty == 'Text' else after))
    function = ('Function: shift.\nThe function needs:\n- n: Int\nThe function produces:\n'
                '- result: Int\nThe function executes:\n- the function returns local.n + 6\n')
    base = HEAD + function + demo('calls shift with n = 13 into data.value', '')
    oracle.pair('function-body', base, base.replace('local.n + 6', 'local.n - 6'), '19', '7')
    variants = ('The application EditOracle manages choices.\nType: Choice.\nChoice has variants:\n'
                '- First\n- Second\nA Data has:\n- value: Choice\n')
    base = demo('constructs Choice variant First with none into data.value', variants)
    oracle.pair('variant-constructor', base, base.replace('variant First', 'variant Second'),
                '["First"]',
                '["Second"]')
    calls = HEAD + ('Action: Set.\nWhen Set happens:\n- the system assigns 17 to data.value\n'
                    '- the system records a trace event named SetDone\nAction: Add.\nWhen Add happens:\n'
                    '- the system computes data.value as data.value + 4\n'
                    '- the system records a trace event named AddDone\n') + demo('calls Set\n- the system calls Add', '')
    call_pair = oracle.pair('call-order', calls,
                            calls.replace('calls Set\n- the system calls Add', 'calls Add\n- the system calls Set'),
                            '21', '17', seeds=('data.value=0',))
    # v0 patch selectors cannot address repeated identical edge occurrences.
    oracle.pair('call-multiplicity', calls, calls.replace('calls Add\n', 'calls Add\n- the system calls Add\n'),
                '21', '25', seeds=('data.value=0',), authorize=False)
    permission = demo('computes data.value as 12 + 5').replace(
        'When Demo happens:\n', 'When Demo happens:\n- the system requires the actor has role Admin\n')
    oracle.pair('permission-removal', permission,
                permission.replace('- the system requires the actor has role Admin\n', ''),
                '', '17', seeds=('actor.role=Guest',), before_failed=True)
    source = demo('computes data.value as 12 + 5')
    oracle.pair('trace-replacement', source, source.replace('named Done', 'named Changed'),
                '17', '17', trace_after='Changed')
    state_type = demo('changes the data value to Ready', HEAD.replace('Int', 'State<Ready, Closed>'))
    oracle.pair('case-sensitive-type', state_type,
                state_type.replace('Ready', 'ready'), 'Ready', 'ready')
    for name, extra in [('version', 'version: 2.0.0\n'), ('endpoint', 'base_llm_endpoint: http://127.0.0.1:2\n')]:
        # Different metadata is tested against a saved Core to avoid duplicate
        # manifest keys or relying on the manifest parser's precedence rules.
        directory = oracle.workspace / ('metadata-' + name)
        directory.mkdir()
        package = oracle.package(directory / 'baseline', source)
        text = oracle.run([oracle.ail, 'ail-core', package]).stdout
        candidate = directory / 'changed.core'
        old = 'version: 0.1.0\n' if name == 'version' else 'base_llm_endpoint: http://127.0.0.1:1\n'
        require(old in text, ('metadata source', old))
        candidate.write_text(text.replace(old, extra))
        for preserve in (False, True):
            rejected = oracle.run([oracle.ail, 'ail-equivalence', '--core-file', candidate, '--baseline', package,
                                   *(['--preserve-baseline'] if preserve else [])], success=False)
            require('semantically-equivalent false' in rejected.stdout and 'metadata' in rejected.stdout,
                    ('metadata rejection diagnostic', rejected.stdout))
            oracle.counts['metadata_rejections'] += 1
    oracle.pair('formatting', source, '\n' + source.replace('\n', '\n\n') + '\n',
                '17', '17', policy='equal', authorize=False)
    additions = [
        ('action', 'Action: Extra.\nWhen Extra happens:\n- the system computes data.value as 8 + 1\n- the system returns data.value\n- the system records a trace event named ExtraDone\n'),
        ('function', function.replace('shift', 'independent')),
        ('type', 'Type: Independent.\nIndependent has variants:\n- New\n')]
    for name, addition in additions:
        oracle.pair('addition-' + name, source, source + addition, '17', '17', policy='add', authorize=False)
    directory = oracle.workspace / 'memory'
    directory.mkdir()
    fallback = oracle.package(directory / 'fallback', demo('changes the data value to 7'))
    oracle.run([oracle.probe, 'memory', arithmetic[0][0], call_pair[0], directory, 'unused', fallback])
    # Execute the actual opaque-ID/permuted graph emitted natively by the probe.
    expected = oracle.records[0]['expected']
    native = oracle.run([directory / 'neutral-memory.elf'])
    require(f'data.value={expected}\n' in native.stdout, native.stdout)
    oracle.records.append({'route': 'memory/opaque-in-memory', 'backend': 'native',
                           'expected': expected, 'failed': False, 'trace': None})
    oracle.counts['runtime_checks'] += 1
    for filename, value, seeds in [('neutral.core', expected, ()), ('legacy.core', '21', ('data.value=0',)),
                                    ('fallback-base.core', '7', ()), ('fallback-changed.core', '8', ())]:
        oracle.execute(['--core-file', directory / filename], directory / filename.replace('.core', ''),
                       value, seeds=seeds)
    oracle.counts['memory_probe_groups'] += 1
    return arithmetic[0]


def publication(oracle: Oracle, arithmetic):
    baseline, candidate, baseline_core, candidate_core = arithmetic
    directory = oracle.workspace / 'publication'
    directory.mkdir()
    target_patch = baseline.parent / 'expected.patch.json'
    bundle, native = directory / 'bundle', directory / 'accepted.elf'
    options = ['--core-file', candidate_core, '--baseline', baseline_core,
               '--target', 'linux-x86_64-elf', '--action', 'Demo', '--artifact-dir', bundle, '--out', native]
    rejected = oracle.run([oracle.ail, 'ail-build', *options], success=False)
    require('AIL-EDIT-REJECTED' in rejected.stderr + rejected.stdout and not bundle.exists() and not native.exists(), rejected)
    oracle.run([oracle.ail, 'ail-build', *options, '--expected-patch', target_patch])
    admission = (bundle / 'admission.ail-edit.txt').read_text()
    require('policy ExpectedPatchTarget' in admission and 'admitted true' in admission
            and 'exact-expected-target-matching true' in admission, admission)
    before = {str(p): hashlib.sha256(p.read_bytes()).hexdigest() for p in directory.rglob('*') if p.is_file()}
    rejected = oracle.run([oracle.ail, 'ail-build', *options], success=False)
    after = {str(p): hashlib.sha256(p.read_bytes()).hexdigest() for p in directory.rglob('*') if p.is_file()}
    require(before == after and 'AIL-EDIT-REJECTED' in rejected.stderr + rejected.stdout,
            ('accepted publication changed on rejection', before, after))
    # Positional package must never silently disappear into standalone Core mode.
    selector = oracle.run([oracle.ail, 'ail-build', baseline, *options], success=False)
    require('cannot combine a positional package with --core-file' in selector.stderr + selector.stdout,
            ('selector-specific rejection', selector.stdout, selector.stderr))
    require(before == {str(p): hashlib.sha256(p.read_bytes()).hexdigest() for p in directory.rglob('*') if p.is_file()},
            'selector rejection changed accepted artifacts')
    result = oracle.run([native])
    expected = next(r['expected'] for r in oracle.records if r['route'] == 'arithmetic-0/approved-target')
    require(f'data.value={expected}\n' in result.stdout, ('authorized build runtime', expected, result.stdout))
    oracle.records.append({'route': 'publication/approved-build', 'backend': 'native',
                           'expected': expected, 'failed': False, 'trace': None})
    oracle.counts['publication_checks'] += 5
    oracle.counts['runtime_checks'] += 1


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--artifact-dir', type=Path, help='retain generated evidence in a new or empty directory')
    options = parser.parse_args()
    require(platform.system() == 'Linux' and platform.machine() in ('x86_64', 'AMD64'),
            'native oracle requires Linux x86_64')
    workspace = options.artifact_dir.resolve() if options.artifact_dir else Path(tempfile.mkdtemp(prefix='ail-edit-oracle-'))
    if options.artifact_dir:
        workspace.mkdir(parents=True, exist_ok=True)
        require(not any(workspace.iterdir()), 'artifact directory must be empty')
    oracle = Oracle(workspace)
    started = time.monotonic()
    try:
        oracle.build()
        arithmetic = generated_cases(oracle)
        publication(oracle, arithmetic)
        require(len(oracle.records) == oracle.counts['runtime_checks'], 'runtime evidence count mismatch')
        report = {'status': 'passed', 'seed': SEED, 'counts': dict(oracle.counts), 'commands': oracle.commands,
                  'runtime_records': oracle.records, 'seconds': round(time.monotonic() - started, 2),
                  'binary': str(oracle.ail), 'binary_sha256': hashlib.sha256(oracle.ail.read_bytes()).hexdigest(),
                  'library': str(oracle.library), 'library_sha256': hashlib.sha256(oracle.library.read_bytes()).hexdigest(),
                  'script_sha256': hashlib.sha256(Path(__file__).read_bytes()).hexdigest()}
        (workspace / 'report.json').write_text(json.dumps(report, indent=2) + '\n')
        print(json.dumps({'status': 'passed', 'counts': dict(oracle.counts), 'evidence': str(workspace)}))
    except Exception as error:
        (workspace / 'failure.json').write_text(json.dumps({'status': 'failed', 'error': str(error),
                                                          'counts': dict(oracle.counts), 'commands': oracle.commands}, indent=2) + '\n')
        raise


if __name__ == '__main__':
    main()
