#!/usr/bin/env python3
"""Independent immutable LRU acceptance; build target/debug/ail first.

Fourteen app and fourteen direct-call routes use the frozen R024 ordered-pair,
opaque-number oracle. All selected Bytes and raw stdin output are compared whole.
Every subprocess uses the existing bounded process harness with a finite deadline
and cleanup. --artifact-dir must initially be empty; no session expectations,
third-party dependencies or Python runtime are needed by generated executables.
"""
import argparse
import collections
import ctypes
import fcntl
import os
import hashlib
import importlib.util
import json
from pathlib import Path
import shutil
import sys
import tempfile
import time

sys.dont_write_bytecode = True
import ail_process_differential as harness

REPO = Path(__file__).resolve().parent.parent
AIL = REPO / 'target/debug/ail'
PACKAGE = REPO / 'examples/lru-cache'
LIBRARY = REPO / 'examples/ail_std_lru.ail'
RUNG = REPO / 'bench/R024-lru-cache'
FINGERPRINT = 'sha256:3a45be77224ac7548a556597dbcc0a5c'
LIMIT = 1_048_576
COUNTS = collections.Counter()
RECORDS = []
ROOT = None
ORACLE = None
FROZEN_ORACLE = {'cases/capacity-minimum.args': 'd14d351a4c536589ea075e4c0fe52c1452bc4b328cd72f9ece5b2aebd4cfbe3f',
 'cases/capacity-negative-bad-operation.args': '4bdb35b7a79641cea5faecab3d013161797490a332a42053b0ffff11a5d78726',
 'cases/capacity-negative-empty.args': '64fd55ff932f0781853c4d7f667fc92524aec53935a2a568fffb7ced0cd413d5',
 'cases/capacity128-256-distinct-puts-three-gets.args': '4b57b41add737365bd7410464e032a96fccf16dae1cd0392e3def0627a2eb1dd',
 'cases/capacity16-fill-then512-mixed-accesses.args': 'c17c9a9af06eff3f37164f03051c68c126b481dbcfcec9d8148fe89a7512db1d',
 'cases/control-pointer-numeric-and-unicode-keys.args': '05805339966a886e0413b15ce245a2c809b0d2df19c510f254f5a0027febadbe',
 'cases/empty-one.args': '8db54134a871d520be0c77974e9f2f81ff42d424672905ff7c2e7f8e898d4132',
 'cases/empty-zero.args': '53b9f292eb6d359f5794c66def2054a0e0be6c596b53ee9bcde51e28f67724b2',
 'cases/full-replacement-no-unrelated-eviction.args': 'a17ac72ba5233786b69dc7e1a49062cbd56ad080bb77226afe2758bac165c72e',
 'cases/full-unicode-fake-state-transport.args': 'eebee83af99ed14949aaca59b35acf6f106a881d6e65b914544ea1a734a15edb',
 'cases/get-escaped-equivalent-unused-values.args': 'e97beff98b0351094323e4a30328b7c09b255916eab94dc668b73a324967cd58',
 'cases/huge-exponent-opaque-put-get-evict.args': 'fd4eba1b3132cf8cc3aed1a366d35a1ace58dd88728b78e2c06574d02b897a6c',
 'cases/ignored-caller-state-duplicates.args': '044de76a7cd28f5613b9d4a61eac51455b8979fdbb178561dabce25b30a16355',
 'cases/ignored-fields-duplicates-and-get-values.args': 'e480b00f2786a3b3e54ecf25cc2fd52e2b6fc1791986305e9350ab7c1b9dc6b8',
 'cases/integer5000-opaque-put-get-evict.args': 'aae106b20371452c67ec5041ffcdc8b4dcb1392de9ad1b30e044d1c670411ac6',
 'cases/long-ascii-key8192.args': '57f36cf5d59f56a91d15744256a7fd86a133bee7894d72b036533af1fb84c28c',
 'cases/long-ascii-value8192.args': 'eaddc2a1382637832c0afad469d039e08acb2aac60a1439db4b4a91999b187df',
 'cases/long-unicode-key8192.args': 'c3dd6954e3185b278b56748cb25a004713ff01f6ba02137bfebc5faf10f0ec40',
 'cases/long-unicode-value8192.args': 'ce68df00ce8ba3963d9fe46fe6a1870e71f49c30bc4e8d5537c28492f8556ab5',
 'cases/max-capacity-no-preallocation.args': '677592113c877fbd953d15795480de5aff32b613e0e3603a91f98cf2095e70a4',
 'cases/negative-zero-normalization.args': 'bf21c6fc915d3ff8d29347a6991662ec31cccab001c9a78349d116d9945d2c80',
 'cases/nested32-arrays-put-get-evict.args': 'e2f65a413f2a97b333ff29657fa24106edf5653397742092498bfd7d37761823',
 'cases/one-null-hit-replace-evict.args': 'd12d6959856f1edb27dcbc51552d28fc2db30e2d7a42877d4fa805ca143001bc',
 'cases/operations-duplicate-key.args': '3b06bb15a64499e35c57b673895afd7d6663e47da86424a04c23dcc57374d609',
 'cases/operations-duplicate-op.args': 'a5e626b43533dfec67a1fb4bdca4ce7c6452fba1d6e3010bdfe1e668155846b3',
 'cases/operations-duplicate-put-value.args': 'ec4230b38031cea42769e99a2231eb492280c6beb30d4e073c97cebe6cce5fa2',
 'cases/operations-escaped-key.args': '793aa6d9875d74221f0c9211b7e4a79c62e47c1b559df1ac22485e75525709ad',
 'cases/operations-escaped-op.args': 'afade4769080d8f446a91a5aa35fb87865a6475a0a583d8bead2f73d8c860196',
 'cases/operations-escaped-put-value.args': '8f48b563ded452d31e5bea30bdd1639ecfacbf9819538349dc6141d83928be82',
 'cases/operations-item-array.args': 'bdc8f0d2df555ba33df336ad3820455b49baa17941f8c450868863f730782aec',
 'cases/operations-item-null.args': '8287f64e2bce10fb1d878491741296edc7a7c5113932f057ba5713057609ba86',
 'cases/operations-item-scalar.args': '2116763d5f6faf11cb40574469877f2021c27daad746f52d373acc798252569e',
 'cases/operations-key-null.args': 'c59beb068ea34346c199e5fb031587ad3faf4fe0cffe1ce829fc2efa02dec82b',
 'cases/operations-key-number.args': 'f35b9c8efbb873b96bfce1bf4d8ea836770c233b94fc74d3b5109922028aa02b',
 'cases/operations-late-malformed-no-partial.args': 'd14c2f1a08c08d3fdd4d283e1f5bf548ad8eb53289eff1aefd9a0f2274f20de1',
 'cases/operations-missing-key.args': 'a85fb79061f10f7b548259cb9d54d36272fdbceed7826990f07653d4b2bd97e0',
 'cases/operations-missing-op.args': 'd22432ced4154eb2560d5854d42327e756d88541936418425ac0378c21ed5878',
 'cases/operations-missing-put-value.args': '61b75205a6cde525419211d36c9944abcfe3bae77a1394e4cd6bebeed6ac209c',
 'cases/operations-op-bool.args': '8408ce0d47db2dc0d45592a4f969eb4837004073abe070bb550f0462757cec8f',
 'cases/operations-op-object.args': 'e9711dd371ce2fe18650cfdfe1ccb570ec3fc398acb0d2e402b95580e605033b',
 'cases/operations-unknown-op.args': '2c944b7acb210aa7d05ea1c910f243d6d87de4a3f2c9dc7d625e6970882315f7',
 'cases/operations-upper-op.args': 'dfa15d3bae729db701a99432b68da53c65bb8a5ad5c76b3efe35b31030d195ee',
 'cases/payload-ordered-duplicates-numbers.args': '3e2c721432b427af79ffc0581a456cae17ecf2781898044ba3fb173589e76f08',
 'cases/promotion-repeated-mru-and-miss.args': '63adc69b4d0a59d541c663a3e6fff669b50c8ec1b5d69c20fd5705e06b39ee9b',
 'cases/request-capacity-5000-digits.args': 'ae2e7c51f31df09fbc6320ce7d1f7297fdf971d23fe8585f7e92127c8f8cccd8',
 'cases/request-capacity-bool.args': '163b8922a713307162eb6d6c5583ec3a44f358ee6fdd3fb8bc8a8da79ae1eb68',
 'cases/request-capacity-exponent.args': '85e2f4939bd437fc636f3935fcefa658f2d07bb1a9f49f67d2e41a5c5d557d02',
 'cases/request-capacity-fraction.args': '47c152d45b995dbbe40108c33fe2f989940beb7361d29599c216b0bdbfe7bd59',
 'cases/request-capacity-high.args': '98de325d4e92094bcd025eb3a5560881df539b6acc95b273f015eb435cd30d19',
 'cases/request-capacity-low.args': 'd6e2ad213a266cfbeb685016dd3a4a01230efe1576d925ef9f357b45aefa138d',
 'cases/request-capacity-null.args': '20c703a70522f7d9e202e139db1488d0c54c43390af6ae918c9cdda9dff6ddaa',
 'cases/request-capacity-string.args': '8f522ef631c88b87d2d178e8762351fc79ec17c6f8c616b49a23d18a77984a06',
 'cases/request-duplicate-capacity.args': '629b3e4035d0dcba5e0b9bb3a1106217664509ec1210701e6cbc6a4ea2afc342',
 'cases/request-duplicate-operations.args': 'd47d04d00cfc3af6871bf896ae5bc9f256f76a0e3702e92cfe285713e57de2e2',
 'cases/request-empty.args': '63663e0c4b42d17147f3f605d36343e3d8039f44e0795a6e370f17bb8caea5e6',
 'cases/request-escaped-capacity.args': '2824a80eaa72f7a973219660f9147b3686b5ddb6730531179c2147e8353a5398',
 'cases/request-escaped-operations.args': 'abaedb4eb8d8404845a0d1092268ffbfa891ee5526c76ff0ab8666035e47019d',
 'cases/request-forbidden-bom-prefix.args': 'f5caaf914f49a1fd8811934448905d5195a01490c1aee6139dc6552de4f5c726',
 'cases/request-forbidden-nbsp-prefix.args': 'c3f8fb74af1e2574e6629c63980cdd0eb463056d2a0fddaf199c51287ec1a51a',
 'cases/request-ignored-infinity.args': '44bba757f6e05b897ace37021b49370b8c4c4c6e30a8c90823d96c8aa6bd9879',
 'cases/request-ignored-invalid-unicode-key.args': 'af9053e40b390f12788de30e1b8a25418c23d7ef40781c9eb83f123237557b54',
 'cases/request-ignored-invalid-unicode-nested.args': '4ceb37aa06056994b496f1ccf842044bed11dd802ae662ed8eda8fbb5c9030ac',
 'cases/request-ignored-invalid-unicode.args': 'c4b7e7aa834c8b7fea6515048f8b83a8dbedb107572581a8ba884b7af92310ce',
 'cases/request-ignored-nan.args': '74345bed22eac86c583640a8ee5ade2a12c1df8477f32e2301e1de1ff0a08c8b',
 'cases/request-leading-zero.args': '7e2f5c401fa6f52c0273d64d6400673081cd9a50576d39ea0fda7fa545ef0aae',
 'cases/request-missing-capacity.args': '2d469478e9f43765626e75407b49baee6f363048b1548607dad9f06f64a95c49',
 'cases/request-missing-operations.args': 'ea53cc10e9c5a40c2e341a993a458e7fa25552bb413128fc12fbdfe4cfcc1c25',
 'cases/request-negative-capacity-nonarray-priority.args': 'a883b675f9489a83ec047fd9ea41de8ed37d9e5093ff6c3e41fcfc18f1a789da',
 'cases/request-operations-null.args': 'a1f16425ae695a63728a8df6a5d0f06ada14d71090380b7b0fff40342cd86a9b',
 'cases/request-operations-object.args': '5da61710662efd6e3b2565d3d5b55704a22fcc971ffe7ea3597a90980ed712f4',
 'cases/request-request-before-negative-and-bad-op.args': '59255232acbaa1f11412bc2c5549d8d86c939f14f8264a91e06b0113b08a6d16',
 'cases/request-root-array.args': '7efde84f2d958bb3f4b9bbf212dce82303ff4873255f292c8fe5c4d8048050a1',
 'cases/request-root-null.args': 'f566a587c8dcd8618972ea0e5eb5f4cc4f0ef563869063c22ead2683f6e1606a',
 'cases/request-root-string.args': '74ed53c5f08506ff5bedf6b28a6328d2daa84803aed4834405605a9470165eea',
 'cases/request-syntax.args': '1d41eb68bd4f76dbbf0b083ce7191061bd6d649cdaeaf1c17ccdba47ee809c2f',
 'cases/request-two-documents.args': 'b86c6d41e7e6bd45cce111611ff942520f9ac6329f37a2119d328a24dc94d98e',
 'cases/spec-six-operation-trace.args': 'dfcdd85dbce1b639217c45e577ef8c60b50b07e74a0293d987636e6d03405db7',
 'cases/zero-stores-nothing.args': '3201db4989aedeb87ff93b8c10089b49b2f776dd1f1a667689456ede0bec8a9f',
 'intent.md': 'dd159124cf3035f64b2b701572e6c32e1e14affc51ba5b45f62d7d8fb962b0be',
 'reference.py': 'e61c37770f75ef1c496c724ef89926f073e7718700fede91beb33be09d773534'}
FROZEN_LIBRARY = {'examples/ail_std_lru.ail/README.md': '01894c83126d2bae671035924dc8fee06460f0f48fb2ffc542ca1e93e5ed1da2',
 'examples/ail_std_lru.ail/ail-package.md': '3c0b254da8551cd722f7f874fc93c6fe780be53dd477163abe28885752cd4450',
 'examples/ail_std_lru.ail/spec.ail-spec.md': '76781d7284f3c329fe489447a396c94d9a0d2efb6f5c0ea670acb3106e6c91c2'}
COMPILER_SHA256 = "22bf54b8ea97847d52e6802d3f167162747137d1d4c92b8651cca303c9ad295d"
PROBE_SOURCE = 'The application LruDirect manages explicit cache calls.\nA Data has:\n- count: Int\n- mode: Text\n- capacity: Int\n- state: Json\n- key: Text\n- value: Json\n- operations: Json\n- request: Text\n- result: Json\n- wrapper: Json\n- path: Text\n- text: Text\n- error: Text\n- first: Json\n- second: Json\n- third: Json\n- next: Json\n- other: Text\nAn Out has:\n- result: Bytes\n\nFunction: repeatGet.\nThe function needs:\n- state: Json\n- key: Text\n- count: Int\nThe function produces:\n- result: Json\nThe function stores:\n- index: Int\n- result: Json\nThe function executes:\n- the function assigns null to local.result\n- the function assigns 0 to local.index\n- while local.index < local.count:\n- the function calls LRU.get with state = local.state; key = local.key into local.result\n- the function computes local.index as local.index + 1\n- end while\n- the function returns local.result\nFunction: framedGet.\nThe function needs:\n- state: Json\n- key: Text\n- count: Int\nThe function produces:\n- result: Json\nThe function stores:\n- result: Json\nThe function executes:\n- if local.count <= 0:\n- the function calls LRU.get with state = local.state; key = local.key into local.result\n- the function returns local.result\n- end if\n- the function calls framedGet with state = local.state; key = local.key; count = local.count - 1 into local.result\n- the function returns local.result\nAction: Probe.\nWhen Probe happens:\n- if boolean text data.mode == "Frames":\n- the system calls framedGet with state = data.state; key = data.key; count = data.count into data.result\n- end if\n- if boolean text data.mode == "Repeat":\n- the system calls repeatGet with state = data.state; key = data.key; count = data.count into data.result\n- end if\n- if boolean text data.mode == "Create":\n- the system assigns {"result":null,"error":""} to data.wrapper\n- try:\n- the system calls LRU.create with capacity = data.capacity into data.result\n- the system assigns "/result" to data.path\n- the system sets JSON value at path data.path in data.wrapper to data.result into data.wrapper\n- recover VALUE into data.error:\n- the system assigns {"result":null,"error":"VALUE"} to data.wrapper\n- end try\n- end if\n- if boolean text data.mode == "Get":\n- the system assigns {"result":null,"error":""} to data.wrapper\n- try:\n- the system calls LRU.get with state = data.state; key = data.key into data.result\n- the system assigns "/result" to data.path\n- the system sets JSON value at path data.path in data.wrapper to data.result into data.wrapper\n- recover VALUE into data.error:\n- the system assigns {"result":null,"error":"VALUE"} to data.wrapper\n- end try\n- the system assigns "/retained_state" to data.path\n- the system sets JSON value at path data.path in data.wrapper to data.state into data.wrapper\n- end if\n- if boolean text data.mode == "Put":\n- the system assigns {"result":null,"error":""} to data.wrapper\n- try:\n- the system calls LRU.put with state = data.state; key = data.key; value = data.value into data.result\n- the system assigns "/result" to data.path\n- the system sets JSON value at path data.path in data.wrapper to data.result into data.wrapper\n- recover VALUE into data.error:\n- the system assigns {"result":null,"error":"VALUE"} to data.wrapper\n- end try\n- the system assigns "/retained_state" to data.path\n- the system sets JSON value at path data.path in data.wrapper to data.state into data.wrapper\n- the system assigns "/retained_value" to data.path\n- the system sets JSON value at path data.path in data.wrapper to data.value into data.wrapper\n- end if\n- if boolean text data.mode == "Run":\n- the system assigns {"result":null,"error":""} to data.wrapper\n- try:\n- the system calls LRU.run with capacity = data.capacity; operations = data.operations into data.result\n- the system assigns "/result" to data.path\n- the system sets JSON value at path data.path in data.wrapper to data.result into data.wrapper\n- recover VALUE into data.error:\n- the system assigns {"result":null,"error":"VALUE"} to data.wrapper\n- end try\n- end if\n- if boolean text data.mode == "Request":\n- the system assigns {"result":null,"error":""} to data.wrapper\n- try:\n- the system calls LRU.runRequest with text = data.request into data.result\n- the system assigns "/result" to data.path\n- the system sets JSON value at path data.path in data.wrapper to data.result into data.wrapper\n- recover VALUE into data.error:\n- the system assigns {"result":null,"error":"VALUE"} to data.wrapper\n- end try\n- end if\n- if boolean text data.mode == "NulGet":\n- the system assigns "\\u0000" to data.key\n- the system assigns {"result":null,"error":""} to data.wrapper\n- try:\n- the system calls LRU.get with state = data.state; key = data.key into data.result\n- the system assigns "/result" to data.path\n- the system sets JSON value at path data.path in data.wrapper to data.result into data.wrapper\n- recover VALUE into data.error:\n- the system assigns {"result":null,"error":"VALUE"} to data.wrapper\n- end try\n- the system assigns "/retained_state" to data.path\n- the system sets JSON value at path data.path in data.wrapper to data.state into data.wrapper\n- end if\n- if boolean text data.mode == "NulPut":\n- the system assigns "\\u0000" to data.key\n- the system assigns {"result":null,"error":""} to data.wrapper\n- try:\n- the system calls LRU.put with state = data.state; key = data.key; value = data.value into data.result\n- the system assigns "/result" to data.path\n- the system sets JSON value at path data.path in data.wrapper to data.result into data.wrapper\n- recover VALUE into data.error:\n- the system assigns {"result":null,"error":"VALUE"} to data.wrapper\n- end try\n- the system assigns "/retained_state" to data.path\n- the system sets JSON value at path data.path in data.wrapper to data.state into data.wrapper\n- the system assigns "/retained_value" to data.path\n- the system sets JSON value at path data.path in data.wrapper to data.value into data.wrapper\n- end if\n- if boolean text data.mode == "Sequence":\n- the system calls LRU.get with state = data.state; key = data.key into data.first\n- the system assigns "/state" to data.path\n- the system gets JSON value at path data.path from data.first into data.next\n- the system calls LRU.put with state = data.next; key = data.other; value = data.value into data.second\n- the system gets JSON value at path data.path from data.second into data.next\n- the system calls LRU.get with state = data.next; key = data.other into data.third\n- the system assigns {"first":null,"second":null,"third":null,"retained_state":null,"retained_value":null} to data.wrapper\n- the system assigns "/first" to data.path\n- the system sets JSON value at path data.path in data.wrapper to data.first into data.wrapper\n- the system assigns "/second" to data.path\n- the system sets JSON value at path data.path in data.wrapper to data.second into data.wrapper\n- the system assigns "/third" to data.path\n- the system sets JSON value at path data.path in data.wrapper to data.third into data.wrapper\n- the system assigns "/retained_state" to data.path\n- the system sets JSON value at path data.path in data.wrapper to data.state into data.wrapper\n- the system assigns "/retained_value" to data.path\n- the system sets JSON value at path data.path in data.wrapper to data.value into data.wrapper\n- end if\n- the system formats data.wrapper as JSON text into data.text\n- the system encodes text data.text as UTF-8 bytes into out.result\n- the system returns out.result\n- the system records a trace event named Probed\n'
def digest(data):
    return hashlib.sha256(data).hexdigest()


def frozen():
    members = {}
    hashed = hashlib.sha256()
    for path in sorted(RUNG.rglob('*')):
        if not path.is_file() or path.relative_to(RUNG).parts[0] == 'program.ail':
            continue
        name = str(path.relative_to(RUNG))
        raw = path.read_bytes()
        members[name] = digest(raw)
        hashed.update(name.encode() + b'\0' + raw + b'\0')
    assert members == FROZEN_ORACLE, 'reviewed oracle members or bytes changed'
    assert len(members) == 80
    assert 'sha256:' + hashed.hexdigest()[:32] == FINGERPRINT
    library = {str(p.relative_to(REPO)): digest(p.read_bytes())
               for p in LIBRARY.rglob('*') if p.is_file()}
    assert library == FROZEN_LIBRARY, 'reviewed library changed'
    return members


def dependencies():
    roots = ['src', 'tests', 'scripts', 'bench', 'examples', 'Cargo.toml', 'Cargo.lock', 'runtime', 'build.rs']
    # Includes every semantic source/manifest, transport/harness and prior oracle.
    # Documentation is not executable input; sample JSON and STE are included.
    paths = set()
    for name in roots:
        p = REPO / name
        paths.update([p] if p.is_file() else (q for q in p.rglob('*') if q.is_file()
                     and '__pycache__' not in q.parts and q.suffix != '.pyc'
                     and (q.suffix != '.md' or q.name in ('ail-package.md', 'spec.ail-spec.md',
                                                        'examples.md', 'support-packages.md'))))
    return {str(p.relative_to(REPO)): digest(p.read_bytes()) for p in sorted(paths)}


def invoke(command, input=b''):
    command = list(map(str, command))
    index = len(RECORDS)
    start = time.monotonic()
    record = {'command': command, 'timeout_seconds': 90,
              'stdin_bytes': len(input), 'stdin_sha256': digest(input)}
    RECORDS.append(record)
    (ROOT / 'io' / f'{index:05d}.stdin').write_bytes(input)
    try:
        result = harness.invoke(command, input, cwd=REPO, timeout=90)
    except BaseException as error:
        record.update(status=None, exception=repr(error), seconds=round(time.monotonic() - start, 4))
        raise
    record.update(status=result.returncode, seconds=round(time.monotonic() - start, 4),
                  stdout_bytes=len(result.stdout), stderr_bytes=len(result.stderr),
                  stdout_sha256=digest(result.stdout), stderr_sha256=digest(result.stderr))
    (ROOT / 'io' / f'{index:05d}.stdout').write_bytes(result.stdout)
    (ROOT / 'io' / f'{index:05d}.stderr').write_bytes(result.stderr)
    (ROOT / "commands.json").write_text(json.dumps(RECORDS, ensure_ascii=False, indent=2) + "\n")
    return result


def checked(command):
    r = invoke(command)
    assert r.returncode == 0 and r.stderr == b'', (command, r.returncode, r.stderr[-1500:])
    return r.stdout


def inspect(program):
    assert program['external_bindings'] == [] and program['capability_grants'] == []
    helpers = set()
    for action in program['actions']:
        pure = action['action'].startswith('LRU.')
        if pure:
            helpers.add(action['action'])
            assert action['instructions'][0]['opcode'] == 'VALUE_FUNCTION'
        for instruction in action['instructions']:
            opcode = instruction['opcode']
            assert not opcode.startswith(('PROCESS_', 'HOST_', 'FILE_', 'DIRECTORY_'))
            assert opcode not in ('CALL_EXTERNAL', 'EVAL_EXPRESSION')
            if opcode.startswith(('STDIN_', 'STDOUT_', 'STDERR_')):
                assert action['action'] == 'Run' and opcode in ('STDIN_READ_TEXT', 'STDOUT_WRITE_TEXT')
            if pure:
                assert opcode not in ('ACTION_BEGIN', 'EMIT_TRACE', 'OUTPUT_FIELD', 'RETURN_SUCCESS')
                destination = instruction['operands'].get('key')
                assert destination is None or destination.startswith('local.'), instruction
                if opcode == 'CALL_VALUE':
                    assert instruction['operands']['target'].startswith('LRU.'), instruction
    assert helpers == {'LRU.' + n for n in ['stateOf','pointer','contains','normalize','create','lookup','get','insert','put','validateOperations','executeValidated','run','failure','runRequest']}
    for action in program['actions']:
        if action['action']=='Run':
            ins=action['instructions']
            assert [i['opcode'] for i in ins]==['ACTION_BEGIN','STDIN_READ_TEXT','CALL_VALUE','JSON_FORMAT','ASSIGN_LITERAL','TEXT_JOIN','STDOUT_WRITE_TEXT','EMIT_TRACE','RETURN_SUCCESS']
            assert ins[-2]['operands']=={'event':'Completed'}
        if action['action']=='Analyze':
            assert [i['operands'] for i in action['instructions'] if i['opcode']=='OUTPUT_FIELD']==[{'key':'out.result'}]
    COUNTS['ir_inspections'] += 1


def build(root, package, stem, actions, full=True):
    frontends = [('source', [package])]
    if full:
        core = root / (stem + '.core')
        core.write_bytes(checked([AIL, 'ail-core', package]))
        rendered = root / (stem + '-rendered')
        rendered.mkdir()
        (rendered / 'ail-package.md').write_text(
            '# AIL Package\nname: lru-rendered\nversion: 0.1.0\nprofile: Application\n'
            'entry: spec.ail-spec.md\nfeatures: things, actions, functions, text, bytes, json, stdio, expressions, control-flow, traces\n'
            'conformance: first-slice\n')
        (rendered / 'spec.ail-spec.md').write_bytes(checked([AIL, 'ail-spec', '--core-file', core]))
        frontends += [('core', ['--core-file', core]), ('rendered', [rendered])]
    routes = []
    for name, arguments in frontends:
        bytecode = root / f'{stem}-{name}.ailbc.json'
        bytecode.write_bytes(checked([AIL, 'ail-lower', *arguments]))
        inspect(json.loads(bytecode.read_bytes()))
        forms = [('direct', arguments, [AIL, 'ail-run', *arguments])]
        if full:
            ste = root / f'{stem}-{name}.ail-ste'
            ste.write_bytes(checked([AIL, 'ail-ste', *arguments]))
            forms.append(('ste', [ste], [AIL, 'ail-vm', ste]))
        if name == 'source' and full:
            forms.append(('bytecode', [bytecode], [AIL, 'ail-vm', bytecode]))
        for form, compile_arguments, vm in forms:
            binaries = {}
            for action in actions:
                binary = root / f'{stem}-{name}-{form}-{action}.elf'
                checked([AIL, 'ail-compile', *compile_arguments, '--action', action,
                         '--target', 'linux-x86_64-elf', '--out', binary])
                binaries[action] = binary
            routes += [(f'{stem}/{name}/{form}/VM', vm, binaries, False),
                       (f'{stem}/{name}/{form}/native', [], binaries, True)]
            print('built', stem, name, form, flush=True)
    return routes


def run(route, action, arguments=(), input=b'', stdio=False):
    _, vm, binaries, native = route
    command = [binaries[action]] if native else [*vm, '--action', action]
    return invoke(command + (['--stdio'] if stdio else []) + list(arguments), input)


def encoded(value):
    return ORACLE.encode(value).encode('utf-8')


def direct_vectors():
    ref = ORACLE
    cases=[]
    def fields(o):return dict(o.pairs)
    def add(name,action,args,result,retained_state=None,retained_value=None,error=''):
     wrapper=ref.Object([('result',result),('error',error)]+([('retained_state',retained_state)] if action in ['Get','Put','NulGet','NulPut'] else [])+([('retained_value',retained_value)] if action in ['Put','NulPut'] else []))
     cases.append((name,action,args,wrapper))
    def direct(name,action,*,capacity=0,state=None,key='',value=None,operations=None,text=''):
     args=[];base=action.replace('Nul','')
     if base in ['Create','Run']:args.append('data.capacity='+str(capacity))
     if base in ['Get','Put']:args.append('data.state='+ref.encode(state));args+=[] if action.startswith('Nul') else ['data.key='+key]
     if base=='Put':args.append('data.value='+ref.encode(value))
     if base=='Run':args.append('data.operations='+ref.encode(operations))
     if base=='Request':args.append('data.request='+text)
     try:
      if base=='Create':result=ref.create(capacity)
      elif base=='Get':result=ref.get(state,'\0' if action.startswith('Nul') else key)
      elif base=='Put':result=ref.put(state,'\0' if action.startswith('Nul') else key,value)
      elif base=='Run':result=ref.run(capacity,operations)
      else:result=ref.evaluate_request(text)
      error=''
     except ref.DomainError as e:result=None;error=e.category
     add(name,action,args,result,state,value,error)
    for cap in [0,1,128,ref.I64_MAX,-1,ref.I64_MIN]:direct('create-'+str(cap),'Create',capacity=cap)
    invalid=['null','[]','1','{}','{"capacity":1}','{"capacity":1,"entries":{},"x":0}','{"capacity":1,"capacity":1}','{"entries":{},"x":0}','{"capacity":true,"entries":{}}','{"capacity":1.0,"entries":{}}','{"capacity":1e0,"entries":{}}','{"capacity":9223372036854775808,"entries":{}}','{"capacity":-1,"entries":{}}','{"capacity":1,"entries":[]}','{"capacity":0,"entries":{"a":1}}','{"capacity":1,"entries":{"a":1,"b":2}}','{"capacity":2,"entries":{"a":1,"\\u0061":2}}','{"capacity":2,"entries":{"🙂":1,"\\ud83d\\ude42":2}}','{"capacity":1,"entries":{},"entries":{}}']
    for i,text in enumerate(invalid):
     for action in ['Get','Put']:direct(f'invalid-{i}-{action}',action,state=ref.parse(text),key='absent',value=ref.parse('{"p":1,"p":-0}'))
    for text in ['{"entries":{},"capacity":-0}','{"entries":{"a":null,"b":2},"capacity":2}','{"capacity":9223372036854775807,"entries":{"a":1,"b":2}}']:
     state=ref.parse(text)
     for key in ['a','b','absent']:
      for action in ['Get','Put']:direct('valid-'+str(len(cases))+'-'+action,action,state=state,key=key,value=ref.parse('{"d":1,"d":null}'))
    state=ref.parse('{"capacity":2,"entries":{"a":null,"b":2}}')
    direct('full-replace','Put',state=state,key='a',value=ref.parse('1.2500e-2'));direct('full-evict','Put',state=state,key='c',value=ref.parse('-0'))
    for key in ['', '/', '~', '~1/', '-', '0', '01', 'é', 'e\u0301', '\u2028fake\ntrace=', 'ASCII'*2048, '界'*8192]:
     state=ref.state_object(1,[(key,ref.parse('{"x":[-0,1E+0003],"x":true}'))]);direct('key-get-'+str(len(cases)),'Get',state=state,key=key);direct('key-put-'+str(len(cases)),'Put',state=state,key=key,value=ref.parse('null'))
    state=ref.state_object(1,[('\0',ref.parse('{"x":1,"x":2}'))]);direct('nul-get','NulGet',state=state);direct('nul-put','NulPut',state=state,value=ref.parse('-0'))
    for text in ['7'*5000,'1e+'+'8'*5000,'['*32+'null'+']'*32,'{"x":1,"x":-0,"a":[1.2500e-2,1E+0003]}',ref.quote('v'*8192),ref.quote('界'*8192)]:
     value=ref.parse(text);state=ref.state_object(1,[('a',value)]);direct('payload-get-'+str(len(cases)),'Get',state=state,key='a');direct('payload-replace-'+str(len(cases)),'Put',state=state,key='a',value=value);direct('payload-evict-'+str(len(cases)),'Put',state=state,key='b',value=None)
    for path in sorted((RUNG/'cases').glob('*.args')):
     args=[line.strip() for line in path.read_text().splitlines() if line.strip() and not line.strip().startswith('#')];text=args[0][len('data.request='):];result=ref.evaluate_request(text)
     if fields(result)['ok']:
      request=ref.parse(text);cap=ref.strict_i64(ref.selected(request,'capacity','VALUE'));ops=ref.selected(request,'operations','VALUE');direct('frozen-run-'+path.stem,'Run',capacity=cap,operations=ops)
    for text in ['{}','{"capacity":-1,"operations":[null]}','{"capacity":1,"operations":[{"op":"put","key":"a","value":1},null]}','{"capacity":1,"operations":[{"op":"get","key":"a","value":1,"value":2}]}']:
     direct('request-'+str(len(cases)),'Request',text=text)
    for cap,ops in [(-1,'null'),(1,'null'),(1,'[null]'),(1,'[{"op":"put","key":"a","value":1},{"op":"put","key":"b"}]'),(1,'[{"op":"get","key":"a","op":"get"}]')]:direct('run-error-'+str(len(cases)),'Run',capacity=cap,operations=ref.parse(ops))
    state=ref.parse('{"entries":{"a":null,"b":{"x":1,"x":2}},"capacity":2}');value=ref.parse('{"v":[-0,1E+0003],"v":null}');first=ref.get(state,'a');second=ref.put(fields(first)['state'],'b',value);third=ref.get(fields(second)['state'],'b');wrapper=ref.Object([('first',first),('second',second),('third',third),('retained_state',state),('retained_value',value)]);cases.append(('sequence','Sequence',['data.state='+ref.encode(state),'data.key=a','data.other=b','data.value='+ref.encode(value)],wrapper))
    return [(name, ["data.mode="+action]+args, encoded(expected)) for name,action,args,expected in cases]


def probe_package(root):
    p = root / 'direct-probe'; p.mkdir()
    (p/'ail-package.md').write_text('# AIL Package\nname: lru-direct-probe\nversion: 0.1.0\nprofile: Application\nentry: spec.ail-spec.md\nfeatures: things, actions, functions, text, bytes, json, expressions, control-flow, traces\nimports: '+str(LIBRARY)+' compatible ^0.1 as LRU\nconformance: first-slice\n')
    (p/'spec.ail-spec.md').write_text(PROBE_SOURCE)
    return p


def selected(result, wanted, route, action='Analyze', recoveries=None, complete=False):
    state = b'out.result=hex:'+wanted.hex().encode()+b'\n'
    assert result.returncode == 0, (route[0], result.returncode, result.stderr[-1500:])
    if complete:
        traces = ['action '+action+' started'] + ['recovered '+c+' into local.error' for c in (recoveries or [])] + ['formatted JSON data.report','computed bytes out.result','trace Simulated']
        if route[3]:
            expected = state
            errors = b'action Analyze started\nformatted JSON data.report\ncomputed bytes out.result\ntrace Simulated\n'
        else:
            prefix = b'ail-vm succeeded\n' if 'ail-vm' in list(map(str,route[1])) else b'ail-run succeeded\n'
            expected = prefix+state+b'trace='+(' -> '.join(traces)).encode()+b'\n'; errors=b''
        assert result.stdout == expected and result.stderr == errors, (route[0],result.stdout[:800],expected[:800],result.stderr,errors)
    else:
        assert [l for l in result.stdout.splitlines(keepends=True) if l.startswith(b'out.result=')] == [state], route[0]
        if route[3]: assert result.stdout==state and result.stderr.endswith(('trace '+('Probed' if action=='Probe' else 'Simulated')+'\n').encode())
        else: assert result.stderr==b'' and result.stdout.endswith(('trace '+('Probed' if action=='Probe' else 'Simulated')+'\n').encode())
    COUNTS['exact_results'] += 1


def exact(result, wanted, context):
    assert result.returncode==0 and result.stdout==wanted and result.stderr==b'', (context,result.returncode,result.stdout[:500],result.stderr[-1500:])
    COUNTS['exact_stdin'] += 1


def fatal(result, category, context, private=()):
    assert result.returncode>0 and result.stdout==b'', (context,result.returncode,result.stdout[:500])
    token=b'AIL bytecode execution step limit exceeded' if category=='FUEL' else ('AIL-RUNTIME-'+category+':').encode()
    assert token in result.stderr and all(s not in result.stdout+result.stderr for s in private), (context,result.stderr[-1500:])
    COUNTS['fatal_checks'] += 1


def fixed_cases():
    cases=[]
    for p in sorted((RUNG/'cases').glob('*.args')):
        lines=[l for l in p.read_text().splitlines() if l.strip() and not l.lstrip().startswith('#')]
        assert len(lines)==1 and lines[0].startswith('data.request=')
        text=lines[0][len('data.request='):]; cases.append((p.stem,text,encoded(ORACLE.evaluate_request(text))))
    assert len(cases)==78
    return cases


def io(routes, cases):
    small=b'{"capacity":0,"operations":[]}'
    padded=small+b' '*(LIMIT-len(small)); private=b'PRIVATE-LRU-INPUT'
    ops=[ORACLE.Object([('op','put'),('key','a'),('value','v'*8192)])]+[ORACLE.Object([('op','get'),('key','a')]) for _ in range(140)]
    alloc=ORACLE.encode(ORACLE.Object([('private',private.decode()),('capacity',ORACLE.Number('1')),('operations',ops)])).encode()
    fuel=b'{"private":"PRIVATE-LRU-INPUT","capacity":1,"operations":['+b','.join([b'{"op":"get","key":"a"}']*3000)+b']}'
    bad=[('invalid-UTF8',b'\xff'+private,'TEXT'),('one-MiB-plus-one',padded+b' ','TEXT-LIMIT'),('request-depth67',b'{"private":"PRIVATE-LRU-INPUT","capacity":1,"operations":[{"op":"put","key":"a","value":'+b'['*64+b'null'+b']'*64+b'}]}','JSON-LIMIT'),('input-depth66',b'{"private":"PRIVATE-LRU-INPUT","capacity":1,"operations":[{"op":"put","key":"a","value":'+b'['*63+b'null'+b']'*63+b'}]}','JSON-LIMIT'),('history-allocation',alloc,'JSON-LIMIT'),('validator-fuel',fuel,'FUEL')]
    zero_ops=[ORACLE.Object([('op','put'),('key',str(i)),('value',ORACLE.Number(str(i)))]) for i in range(512)]
    zero_text=ORACLE.encode(ORACLE.Object([('capacity',ORACLE.Number('0')),('operations',zero_ops)]))
    output_control='{"private":"PRIVATE-LRU-INPUT","capacity":1,"operations":[{"op":"put","key":"a","value":'+'['*60+'null'+']'*60+'},{"op":"put","key":"b","value":0}]}'
    output_fatal='{"private":"PRIVATE-LRU-INPUT","capacity":1,"operations":[{"op":"put","key":"a","value":'+'['*61+'null'+']'*61+'},{"op":"put","key":"b","value":0}]}'
    bad.append(('valid-input64-result65',output_fatal.encode(),'JSON-LIMIT'))
    assert (PACKAGE/'expected.json').read_bytes()==encoded(ORACLE.evaluate_request((PACKAGE/'input.json').read_text()))+b'\n'
    values=[(label,text.encode(),wanted+b'\n') for label,text,wanted in cases]+[('exact-one-MiB',padded,encoded(ORACLE.evaluate_request(small.decode()))+b'\n'),('sample',(PACKAGE/'input.json').read_bytes(),(PACKAGE/'expected.json').read_bytes()),('zero512',zero_text.encode(),encoded(ORACLE.evaluate_request(zero_text))+b'\n'),('result-depth64-control',output_control.encode(),encoded(ORACLE.evaluate_request(output_control))+b'\n')]
    for route in routes:
        for label,text,wanted in values: exact(run(route,'Run',input=text,stdio=True),wanted,(route[0],label))
        fatal(run(route,'Run',input=private),'CAPABILITY',(route[0],'missing-grant'),[private])
        for label,text,category in bad: fatal(run(route,'Run',input=text,stdio=True),category,(route[0],label),[private])
        print('stdin',route[0],len(values),len(bad)+1,flush=True)


def mutation(root):
    library=root/'mutated-library.ail'; shutil.copytree(LIBRARY,library)
    source=library/'spec.ail-spec.md';original=source.read_text();start=original.index('Function: lookup.');end=original.index('Function: get.',start)
    rule='- the function removes JSON value at path local.path from local.entries into local.entries\n- the function sets JSON value at path local.path in local.entries to local.value into local.entries\n'
    assert original[start:end].count(rule)==1
    source.write_text(original[:start]+original[start:end].replace(rule,'')+original[end:])
    package=root/'mutated-app';shutil.copytree(PACKAGE,package);f=package/'ail-package.md';f.write_text(f.read_text().replace('../ail_std_lru.ail',str(library)))
    text=(RUNG/'cases/spec-six-operation-trace.args').read_text().strip().split('=',1)[1]; wanted=encoded(ORACLE.evaluate_request(text))
    for route in build(root,package,'mutation',['Analyze'],full=False):
        result=run(route,'Analyze',['data.request='+text]);line=next(l for l in result.stdout.splitlines() if l.startswith(b'out.result=hex:'));actual=bytes.fromhex(line[len(b'out.result=hex:'):].decode())
        selected(result,actual,route,complete=True)
        assert dict(ORACLE.parse(actual.decode()).pairs)['ok'] is True and actual != wanted
        (root/('mutation-'+('native' if route[3] else 'VM')+'.json')).write_text(json.dumps({'successful_report':True,'expected_hex':wanted.hex(),'actual_hex':actual.hex(),'rejected':True},indent=2)+'\n')
        COUNTS['semantic_mutations_detected']+=1


def ordinary_direct(routes):
    ref=ORACLE
    smokes=[('Create',['data.capacity=2'],ref.create(2),None,None,[]),('Get',['data.state={"entries":{"a":null,"b":2},"capacity":2}','data.key=a'],ref.get(ref.parse('{"entries":{"a":null,"b":2},"capacity":2}'),'a'),ref.parse('{"entries":{"a":null,"b":2},"capacity":2}'),None,[]),('Put',['data.state={"capacity":2,"entries":{"a":1,"b":2}}','data.key=a','data.value={"x":1,"x":-0}'],ref.put(ref.parse('{"capacity":2,"entries":{"a":1,"b":2}}'),'a',ref.parse('{"x":1,"x":-0}')),ref.parse('{"capacity":2,"entries":{"a":1,"b":2}}'),ref.parse('{"x":1,"x":-0}'),[]),('Run',['data.capacity=1','data.operations=[{"op":"get","key":"missing"}]'],ref.run(1,ref.parse('[{"op":"get","key":"missing"}]')),None,None,[]),('Request',['data.request={}'],ref.evaluate_request('{}'),None,None,['recovered JSON-PATH into local.error']),('Create',['data.capacity=-1'],None,None,None,['recovered VALUE into data.error']),('Get',['data.state={}','data.key=absent'],None,ref.parse('{}'),None,['recovered VALUE into data.error'])]
    for number,(mode,args,value,state,payload,recovery) in enumerate(smokes):
     wrapper=ref.Object([('result',value),('error','VALUE' if value is None else '')]+([('retained_state',state)] if mode in ['Get','Put'] else [])+([('retained_value',payload)] if mode=='Put' else []));wanted=encoded(wrapper);index=['Frames','Repeat','Create','Get','Put','Run','Request','NulGet','NulPut','Sequence'].index(mode)
     for route in routes:
      trace=['action Probe started']+[f'jump __control_{i}_otherwise' for i in range(index)]+['assigned literal data.wrapper']+([] if route[3] else recovery)
      trace+=['assigned literal data.wrapper'] if value is None else ['assigned literal data.path','set JSON data.wrapper']
      if mode in ['Get','Put']:trace+=['assigned literal data.path','set JSON data.wrapper']
      if mode=='Put':trace+=['assigned literal data.path','set JSON data.wrapper']
      trace += [f'jump __control_{i}_otherwise' for i in range(index+1,10)]+['formatted JSON data.text','computed bytes out.result','trace Probed']
      line=b'out.result=hex:'+wanted.hex().encode()+b'\n'
      stdout=line if route[3] else b'ail-run succeeded\n'+line+b'trace='+(' -> '.join(trace)).encode()+b'\n';stderr=('\n'.join(trace)+'\n').encode() if route[3] else b''
      result=run(route,'Probe',['data.mode='+mode]+args);assert result.returncode==0 and result.stdout==stdout and result.stderr==stderr,(mode,route[0],result.stdout[:500],stdout[:500],result.stderr,stderr)
      tag=str(number)+'-'+('native' if route[3] else 'VM');(ROOT/(tag+'.expected.stdout')).write_bytes(stdout);(ROOT/(tag+'.expected.stderr')).write_bytes(stderr)
     COUNTS['complete_direct_reports'] += 2

def direct_fatals(routes):
    secret=b'PRIVATE-LRU-INPUT'
    for route in routes:
     fatal(run(route,'Probe',['data.mode=Frames','data.state={"capacity":1,"entries":{"secret":"PRIVATE-LRU-INPUT"}}','data.key=secret','data.count=70']),'DEPTH',(route[0],'frames70'),[secret])
     fatal(run(route,'Probe',['data.mode=Repeat','data.state={"capacity":1,"entries":{}}','data.key='+'k'*16384+secret.decode(),'data.count=1300']),'TEXT-LIMIT',(route[0],'text-arena'),[secret])
     fatal(run(route,'Probe',['data.mode=Get','data.state={"PRIVATE-LRU-INPUT":','data.key=absent']),'JSON',(route[0],'typed-malformed'),[secret])
     print('direct fatals',route[0],flush=True)

def main():
    global ROOT,ORACLE
    parser=argparse.ArgumentParser(description=__doc__);parser.add_argument('--artifact-dir',required=True,type=Path);parser.add_argument('--preflight-only',action='store_true');parser.add_argument('--worker',action='store_true',help=argparse.SUPPRESS);parser.add_argument('--reservation-fd',type=int,help=argparse.SUPPRESS);args=parser.parse_args()
    if args.reservation_fd is None:
        raise SystemExit('worker requires a supervisor reservation')
    ROOT=args.artifact_dir.resolve()
    reserved=os.fstat(args.reservation_fd);actual=ROOT.stat()
    assert (reserved.st_dev,reserved.st_ino)==(actual.st_dev,actual.st_ino), 'artifact destination changed after reservation'
    ROOT.mkdir(parents=True,exist_ok=True);assert not any(ROOT.iterdir());(ROOT/'io').mkdir();assert ctypes.CDLL(None).prctl(36,1,0,0,0)==0
    before=frozen();deps=dependencies();compiler=digest(AIL.read_bytes());assert compiler==COMPILER_SHA256
    (ROOT/'identity-before.json').write_text(json.dumps({'oracle':before,'dependencies':deps,'compiler':compiler},indent=2)+'\n')
    snapshot=ROOT/'sources'
    for name in deps:
        p=REPO/name;d=snapshot/name;d.parent.mkdir(parents=True,exist_ok=True);d.write_bytes(p.read_bytes())
    spec=importlib.util.spec_from_file_location('lru_independent_oracle',RUNG/'reference.py');ORACLE=importlib.util.module_from_spec(spec);sys.modules[spec.name]=ORACLE;spec.loader.exec_module(ORACLE)
    start=time.monotonic()
    try:
        probe=probe_package(ROOT);early=build(ROOT,PACKAGE,'smoke-app',['Analyze','Run'],False);early_probe=build(ROOT,probe,'smoke-probe',['Probe'],False)
        smoke=[('{"capacity":2,"operations":[{"op":"put","key":"a","value":1},{"op":"get","key":"a"}]}',[]),('{}',['JSON-PATH']),('{"capacity":-1,"operations":[null]}',[]),('{"capacity":1,"operations":[{"op":"bad","key":"a"}]}',['VALUE'])]
        for route in early:
            for text,recoveries in smoke:selected(run(route,'Analyze',['data.request='+text]),encoded(ORACLE.evaluate_request(text)),route,recoveries=recoveries,complete=True)
        vectors=direct_vectors()
        for route in early_probe:
            for label,arguments,wanted in [vectors[0],vectors[6],vectors[-1]]:selected(run(route,'Probe',arguments),wanted,route,'Probe')
        ordinary_direct(early_probe)
        io(early,[(str(i),text,encoded(ORACLE.evaluate_request(text))) for i,(text,_) in enumerate(smoke)])

        for route in early_probe:
            fatal(run(route,'Probe',['data.mode=Frames','data.state={"capacity":1,"entries":{}}','data.key=a','data.count=70']),'DEPTH',(route[0],'frames70'))
            fatal(run(route,'Probe',['data.mode=Repeat','data.state={"capacity":1,"entries":{}}','data.key='+'k'*16384,'data.count=1300']),'TEXT-LIMIT',(route[0],'text-allocation'))
            fatal(run(route,'Probe',['data.mode=Get','data.state={','data.key=absent']),'JSON',(route[0],'malformed-typed-state'))
        mutation(ROOT)
        (ROOT/'preflight.json').write_text(json.dumps({'counts':dict(COUNTS),'commands':len(RECORDS)},indent=2)+'\n')
        routes=[];probes=[]
        if not args.preflight_only:
            routes=build(ROOT,PACKAGE,'app',['Analyze','Run']);probes=build(ROOT,probe,'probe',['Probe']);assert len(routes)==len(probes)==14
            cases=fixed_cases();(ROOT/'expectations.json').write_text(json.dumps({'requests':[(n,w.hex()) for n,_,w in cases],'direct':[(n,a,w.hex()) for n,a,w in vectors]},ensure_ascii=False,indent=2)+'\n')
            for route in routes:
                for label,text,wanted in cases:selected(run(route,'Analyze',['data.request='+text]),wanted,route)
                print('Analyze',route[0],len(cases),flush=True)
            for route in probes:
                for label,arguments,wanted in vectors:selected(run(route,'Probe',arguments),wanted,route,'Probe')
                print('direct',route[0],len(vectors),flush=True)
            direct_fatals(probes)
            io(routes,cases)
        assert frozen()==before and dependencies()==deps and digest(AIL.read_bytes())==compiler
        summary={'status':'PASS','counts':dict(COUNTS),'app_routes':len(routes),'direct_routes':len(probes),'direct_vectors':len(vectors),'commands':len(RECORDS),'seconds':time.monotonic()-start,'fingerprint':FINGERPRINT}
        (ROOT/'summary.json').write_text(json.dumps(summary,indent=2)+'\n');print(json.dumps(summary),flush=True)
    except BaseException as error:
        (ROOT/'failure.json').write_text(json.dumps({'exception':repr(error),'counts':dict(COUNTS),'commands':len(RECORDS)},indent=2)+'\n');raise
    finally:(ROOT/'commands.json').write_text(json.dumps(RECORDS,ensure_ascii=False,indent=2)+'\n')


def reserve_destination(destination):
    """Lock the directory inode before inspecting or accepting its contents."""
    destination.mkdir(parents=True,exist_ok=True)
    descriptor=os.open(destination,os.O_RDONLY|os.O_DIRECTORY|os.O_NOFOLLOW)
    try:
        try:
            fcntl.flock(descriptor,fcntl.LOCK_EX|fcntl.LOCK_NB)
        except BlockingIOError:
            raise SystemExit('artifact directory is already reserved')
        if any(destination.iterdir()):
            raise SystemExit('artifact directory must initially be empty')
        return descriptor
    except BaseException:
        os.close(descriptor)
        raise


def write_supervisor_records(descriptor,stdout,stderr,record):
    # Relative exclusive creation binds writes to the reserved inode, including
    # exception handling; an existing file is never replaced.
    for name,data in [('outer.stdout',stdout),('outer.stderr',stderr),
                      ('outer.json',(json.dumps(record,indent=2)+'\n').encode())]:
        flags=os.O_WRONLY|os.O_CREAT|os.O_EXCL|os.O_NOFOLLOW
        with os.fdopen(os.open(name,flags,0o600,dir_fd=descriptor),'wb') as stream:
            stream.write(data)


if __name__=='__main__':
    if '--worker' in sys.argv or '--help' in sys.argv or '-h' in sys.argv:
        main()
    else:
        supervisor=argparse.ArgumentParser(description=__doc__)
        supervisor.add_argument('--artifact-dir',required=True,type=Path)
        supervisor.add_argument('--preflight-only',action='store_true')
        outer_args=supervisor.parse_args()
        destination=outer_args.artifact_dir.resolve()
        descriptor=reserve_destination(destination)
        try:
            assert ctypes.CDLL(None).prctl(36,1,0,0,0)==0
            command=[sys.executable,str(Path(__file__).resolve()),'--worker',
                     '--reservation-fd',str(descriptor),*sys.argv[1:]]
            started=time.monotonic()
            try:
                result=harness.invoke(command,cwd=REPO,timeout=7200,pass_fds=(descriptor,))
            except BaseException as error:
                stdout=getattr(error,'output',b'') or b'';stderr=getattr(error,'stderr',b'') or b''
                write_supervisor_records(descriptor,stdout,stderr,{
                    'command':command,'timeout_seconds':7200,'status':None,
                    'exception':repr(error),'seconds':time.monotonic()-started,
                    'stdout_sha256':digest(stdout),'stderr_sha256':digest(stderr)})
                raise
            write_supervisor_records(descriptor,result.stdout,result.stderr,{
                'command':command,'timeout_seconds':7200,'status':result.returncode,
                'seconds':time.monotonic()-started,'stdout_sha256':digest(result.stdout),
                'stderr_sha256':digest(result.stderr)})
            sys.stdout.buffer.write(result.stdout);sys.stderr.buffer.write(result.stderr)
            sys.exit(result.returncode)
        finally:
            os.close(descriptor)
