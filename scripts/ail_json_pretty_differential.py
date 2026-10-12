#!/usr/bin/env python3
"""Independent lossless JSON acceptance; build target/debug/ail first.

Fourteen app and fourteen direct-render routes use the frozen R023 ordered-pair,
opaque-number oracle. All selected Bytes and raw stdin output are compared whole.
Every subprocess uses the existing bounded process harness with a finite deadline
and cleanup. --artifact-dir must initially be empty; no session expectations,
third-party dependencies or Python runtime are needed by generated executables.
"""
import argparse
import collections
import ctypes
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
PACKAGE = REPO / 'examples/json-pretty'
LIBRARY = REPO / 'examples/ail_std_json_pretty.ail'
RUNG = REPO / 'bench/R023-json-pretty'
FINGERPRINT = 'sha256:611860d22426e9cba9d97d4ba0dcde8d'
LIMIT = 1_048_576
COUNTS = collections.Counter()
RECORDS = []
ROOT = None
ORACLE = None
# Exact reviewed members are embedded below; loading requires their identity.
FROZEN_ORACLE = {'cases/all-controls-key.args': 'b6231be5fe288744c120ba0e28b73c7df3dd383623d7dc42fd36ca24a1aeb0b0',
 'cases/all-controls-value.args': 'b7dc3372bd74f535baf89ac817925de0a762de18fb7d583591c3df90a429149e',
 'cases/array-128-scalars.args': 'ef0b19cb96d10e6523b35f751a126fa83c8013a66adcfa9697440a00b4326594',
 'cases/backslash-runs.args': 'fba6ef025087e25519d51a94a32a058559b705d85836583f0cd4c21a517b272c',
 'cases/bom-key-value.args': '3c84692b928f58ae8520abab8d86e5b76c2678a13d28ca22d4d56eff5a9b923f',
 'cases/depth-32-empty-leaf.args': 'd143229a2ece8495a02664ae36f7d6387717efbd9a92dddf07ae08912394ceb6',
 'cases/depth-64-open-arrays.args': '146d25bea66dcad2ad86ac7b03a7a6393c0ade728ab8315b693b86afccff69e7',
 'cases/escaped-equivalent-empty-keys.args': '96d8f646adae0523048301dbce87a25ce17ce55102b65ec8dca3e99d18d96ef3',
 'cases/escaped-equivalent-inner.args': 'd8441463ae56708a461d8b1f709f85bb5e7dd6d71c525de9162c166df61bc1f7',
 'cases/escaped-input-spellings.args': '4191d42488e6cf88ae9f8f657578cd269d6ea007e5b7d3cba9420aff5160e6d7',
 'cases/escaped-supplementary.args': '650d53af9413a7117ddf105eef1aa0a8fc745a947bbbb9bcdd84a28ace2fe529',
 'cases/input-whitespace.args': '840b683d94d160525840ef63689c8796d87e09fc70e458102894204276c9dbb4',
 'cases/integer-5000-digits.args': 'f38e75e4fc921513b8b2545125952638487a062724c8603b31340bf1d2732874',
 'cases/json-invalid-bad-unicode-escape.args': '8ae70726ce2df9e0763c12426e3bca617ce426800cdfe109adabd115a01ede1e',
 'cases/json-invalid-bom-outside.args': '254b29703236447d350c62ccb3971d4d842c714c633e1d633ff26abe7cdba63d',
 'cases/json-invalid-comment.args': '000919282e8ee8f1089850f21730a62a365322f9fb31f979e434415b6a1c3bb0',
 'cases/json-invalid-decimal-without-integer.args': '10e647a973a9de2d487a685a6dda064849cf3e582916a9931e4d1fb5ef073b0a',
 'cases/json-invalid-em-space-outside.args': '3a3a3a9a5a6f3a21f6fa7067e83ea6fcf4e98012b4e6f9aa1f2c4ff7a7437dd1',
 'cases/json-invalid-empty.args': 'c23e11fab337cc1868eda0a70ea2b49029c1ddaa4d09c4d9ebf8f0a6fefa26e9',
 'cases/json-invalid-file-separator-outside.args': '65f7c5bf4f5fb32dd107e6181be8f9845f017bb8b132a35072a6766b7419f939',
 'cases/json-invalid-form-feed-outside.args': 'e91b73dbe426894ab7b298e2d5c357da8ca89c25028e362332c81a713c2b7115',
 'cases/json-invalid-infinity.args': '1e2f3fd9054fc7c9d8fbf537c58b1fe7d9db6b59bd540a6173f9b03cadbc8f8a',
 'cases/json-invalid-invalid-escape.args': 'cb8472f6c89afac61c7a2eaf95d58c92bd792e948559c809dc5b6dc8f3c8706e',
 'cases/json-invalid-leading-plus.args': 'e14ef5574887bab8907ae7141b2f20561a77ea40c8bc7c8eab13e6426ede7e47',
 'cases/json-invalid-leading-zero.args': '5133dcb1acc2db28b2b339c490d1841b6b4dec45e063878ec1d0cd58a816f1ef',
 'cases/json-invalid-line-separator-outside.args': '36babb0206e8dc14dea05f65f0d361e722fa089451910af2a5b0cabb21e688ae',
 'cases/json-invalid-missing-colon.args': '89e2a40978276e686e6c60f58e1d570d9357d62e88625211c726eda6aae12331',
 'cases/json-invalid-missing-comma.args': '20ea32b805d17936c0de20a4ffd97f4df4348162b0c19a089d6dc934061c33a1',
 'cases/json-invalid-missing-exponent.args': 'bb96ca5f4a07a843b7e9c98b55be4995ee467e81b0b97fb952c52d025ccc01f5',
 'cases/json-invalid-missing-fraction.args': '2c7780652ad825bf78628dc4b47a41e67383440f1b3c686b4d85d9d4ff1aaa32',
 'cases/json-invalid-nan-nested.args': 'c91bd0f39ba17ea84c4b90f59ac8653672f27cb4e28c745da7f41339e532382b',
 'cases/json-invalid-nan.args': 'ad7ed33569f727ed88c05c11e7013e3cf43be5b029c434b525058f20a7d8e283',
 'cases/json-invalid-nbsp-outside.args': 'bc648b21117ce8650381b2e81da372c3e93fe0c3d0ca81741aac9b4d478a0ae2',
 'cases/json-invalid-negative-infinity.args': 'dbfac6465233dcb6d48ccebb8361ca565e1a97ea89c5bced68182748d5582810',
 'cases/json-invalid-negative-leading-zero.args': 'd112e9965f58bc2dfeba207c5e6b4e1cfabe033a8978d39782acb6555abf868f',
 'cases/json-invalid-nel-outside.args': '67f3a89f07b60aa07332ce5d2cc72e7e8b74a7a10ef504f163f62d6c147d2090',
 'cases/json-invalid-paragraph-separator-outside.args': '282509339fddc715d8216d850203dced94671841b95babe93cb61b3888fed8a2',
 'cases/json-invalid-raw-lf-string.args': '87634c83a6e83c2feab35ee93123ae9cb6e5fea6b081dc3deb00ea38a5f8b76f',
 'cases/json-invalid-raw-nul-string.args': '70b0af06283ae3cfb93da5c052a00e8385f2a38a57b06080f128642894454be4',
 'cases/json-invalid-raw-tab-string.args': '809867985427d611625173721541a528e46c617c9430e289b15e769e2315a0d0',
 'cases/json-invalid-short-unicode-escape.args': '23f13db9953f4193e9765775e4be8b87123746f63e5b652d433f6e31056d493e',
 'cases/json-invalid-single-quote.args': 'bd5dd7d81d9af2c92ad09a3ca89dc9ecea153881fc3d453c8f68575d9eb94400',
 'cases/json-invalid-surrogate-broken-pair.args': '03e3e78276823c42e325d98569fd0f82b2c6aaeec34f49e8188cc27848d3502a',
 'cases/json-invalid-surrogate-high.args': '135a51b7f48c811be30b86f4a35c1d6f4c6536d63538a14e17fc4ffc36fa5f0a',
 'cases/json-invalid-surrogate-key.args': '9658dc12149d45c55cbdb049cd71214cf29546d09d7ec7e6ecec5253283e2a59',
 'cases/json-invalid-surrogate-low.args': 'd88ee97a8d77bfbfb1de2fd619e506221055c109e35fd28c1a6f3f401833fcfb',
 'cases/json-invalid-surrogate-reversed.args': 'ca4a935aebc8f9a0b8fb7af7acbec31c483256cdc73280d4049c34c35e358bc2',
 'cases/json-invalid-surrogate-unused-value.args': 'ee1672b25623ea053c0dfc2b817ccc101b99638401285ad3b3294ef4e1f46021',
 'cases/json-invalid-trailing-comma-array.args': '9bd61a40ef1d461bc3a15b69e34bb3ab026c4054be1d6a65ad4e57b9b0629c38',
 'cases/json-invalid-trailing-comma-object.args': '43550a2c5b5e6f532bfb5f3fe3e8f7a15b7690358e2213cd4784e77f01f84a3c',
 'cases/json-invalid-trailing-value.args': '9e8059fa5e8e359b1ad2ac89069522556312caf88a159e25a48bdbce8969faf6',
 'cases/json-invalid-unclosed-array.args': '36c8e93dcd820320d0d06108289beab3b0120b0615a21570d135af99e3b86044',
 'cases/json-invalid-unclosed-object.args': 'a802524058271ccf722aef34394f9b495817ad638c06cb1efc304a5e0df98798',
 'cases/json-invalid-unclosed-string.args': '039dd44ec10e15b7133941591f857037c2e96cd3d093d803d6741b94fd6a8cfd',
 'cases/json-invalid-unquoted-key.args': '5b6a73293dea8d73f713b7cbc7deee516f360822827e16a8a7ba6085582af15d',
 'cases/json-invalid-vertical-tab-outside.args': '49c2ab21aab5459bc423ea2ede18e94cf05ac382ee39573221a796088445e5c1',
 'cases/json-invalid-whitespace.args': '797d03d140ddffe21ca84657f85c15851cd7fb6467e16c833b6fc6f551aa5547',
 'cases/json-invalid-zero-width-outside.args': 'bc94ea9560c324c40f5cc95be036606fa9476f1b51ade161155dba77cc0bd962',
 'cases/large-ascii-key.args': 'ba0604da177b2f874757b8a16c45d8c6a85fdd4169d78fee9782545ce5c6fa3b',
 'cases/large-ascii-value.args': 'd9022cb310dd495b6c4e5e6aa5006dfcfc12dc4747cdd0b768d466ce8cfa8c7d',
 'cases/large-unicode-key.args': '0c52d51f6ea027a627e37191b2dad8c99b2ab8a65cf7b058d91e29bd167a94ce',
 'cases/large-unicode-value.args': '7b92c7f9888ea0c46b77bf06bd9926fcabca0861a38566815dc464b4a190902f',
 'cases/line-separator-fake-state.args': '056fcd35f4558a9c3a9528aafbad4d02ac42a89620fe02e7e3f0d34d271a30f7',
 'cases/mixed-depth-16.args': '8a6d94cd93e2cb08cd3b5cdb07388353487ae7082cee3afefa0a39a101c5e5da',
 'cases/nested-empty-mixed.args': '66afd5ce198e4f6ee761efea5e974382279485b97d39acd86d164de0732e8d94',
 'cases/number-spellings.args': '00d9a0caf0993a397756cbc09e931a722d5e68314bbd693b6c2c379ab6ca8d65',
 'cases/object-128-mixed-duplicate-pairs.args': 'ea314ae58ff3ec4a0993dd0e37b498992b81975a13f220478b6b92f11900c01e',
 'cases/pair-order.args': 'f08ada23b488b8a0720f9c8d098837a610d2ec1fa932af6a52a36c6b769c08c6',
 'cases/request-invalid-absent.args': 'bc7deded38cbecdf0eae4d895e7220b189c26dfbbaab13d8dd556cc3d7f7e92d',
 'cases/request-invalid-bad-escape-unused.args': '8ab55e274f1c6414bd7038e6f7b96f231d85c4e4dfa9854643fad66d0178eff1',
 'cases/request-invalid-bom-outside.args': 'd7746979a47018d8ed048a6e841267222c74ee16a089f07d7d4e46c66703757a',
 'cases/request-invalid-duplicate-priority.args': 'badb2f20da8515bd24878a2f486364c5c266cac788c969ee5c91f8477f5ce7ca',
 'cases/request-invalid-duplicate.args': '61de1af3bbf80630229ade5e8c43b577794840c5b6a4fc5d742bf7738a2e413c',
 'cases/request-invalid-em-space-outside.args': 'fed03089754dc93e63cd3840a30a73d09043b3a58f4eb4d8d5fdad7d847fbe4a',
 'cases/request-invalid-empty.args': '63663e0c4b42d17147f3f605d36343e3d8039f44e0795a6e370f17bb8caea5e6',
 'cases/request-invalid-escaped-duplicate.args': 'f1d7ae4c69d8e3adb337725dec0884106b422850c3caceb997887ca0e61c598c',
 'cases/request-invalid-infinity-unused.args': '2913c8a6a4980427d5566c4fe6877bd9349eb974124a70074930ea379b95a832',
 'cases/request-invalid-leading-zero-unused.args': '0246c8a573f75c37241c7659d78b51ef09ee6dbc6e9a633d465a2f1c071a04ca',
 'cases/request-invalid-malformed-trailer.args': 'a48f3e01d04fbe002c7a54ed6bff8eab6cbc3807d874ed694ea5d241f83b3a8e',
 'cases/request-invalid-malformed-unused.args': 'aba865cac327ab29d96a52eecfa4e1793cb94ef21635fd51529cbe6f3802d380',
 'cases/request-invalid-nan.args': '403bc30f66bc3d18fd5aa0b8e2272bf519940763d6a3ed8c7e1f70ebf3a53125',
 'cases/request-invalid-nbsp-outside.args': '3684add423981f1b45abc06fa83fc8df377ed4fa17e2d7009015302363139541',
 'cases/request-invalid-near-name.args': '436d973ba279012a78041182a46f709eea68d012881acf64aeb42f7ffa4d99d2',
 'cases/request-invalid-negative-infinity-unused.args': 'b65f83dee605e310f8c6016826c425a7f42f0e3624339927a48f3f2d24335a91',
 'cases/request-invalid-nonobject-array.args': '7efde84f2d958bb3f4b9bbf212dce82303ff4873255f292c8fe5c4d8048050a1',
 'cases/request-invalid-nonobject-boolean.args': '48b84a2d1b194f15c58a71a9ed4e185652604e26ed062a753b0d38866c1d2f9e',
 'cases/request-invalid-nonobject-null.args': 'f566a587c8dcd8618972ea0e5eb5f4cc4f0ef563869063c22ead2683f6e1606a',
 'cases/request-invalid-nonobject-number.args': 'f0d9f8326b9db15509e1880af45d5b9a1d3fef904c1fc17d491a2c815caefe61',
 'cases/request-invalid-nonobject-string.args': '74ed53c5f08506ff5bedf6b28a6328d2daa84803aed4834405605a9470165eea',
 'cases/request-invalid-raw-control.args': 'f2533e21939c5efb2e0580dae690af1308f0b25d2dc6e2180e03c6606623be40',
 'cases/request-invalid-selected-array.args': '194ef476e775dc782b8218c7049f5cc371deb0adb34a6c7abbff72a89ca7f663',
 'cases/request-invalid-selected-bool.args': '23603c412a2d66557abba1cbe1ee39cd56e639ee11530c72592dba645d6f08bb',
 'cases/request-invalid-selected-null.args': '048316778966349e608e7c14a846a465826b1fe1d654fc0a3e466bf6b68ca7be',
 'cases/request-invalid-selected-number.args': '4960cad0741649b3683157aea82be9e49f5e0fdeafcacc9db92ef271d117ec67',
 'cases/request-invalid-selected-object.args': '2b4d6c2b539f6dc8735a1f0e251fe6ea18cc5c1644305a1766c8a0fabc054e2c',
 'cases/request-invalid-surrogate-selected.args': '43d48d787e1258887432a296edd69a50244606145e5098ba51b0c74836b7d265',
 'cases/request-invalid-surrogate-unused-deep.args': '07196bc0153bc4d622050b47f98927fb336d5f4d011d8384ddfd104cd06dc74a',
 'cases/request-invalid-surrogate-unused-key.args': 'fd4d1797ae1e589ea9dbc63a9d819531a5bfb11e6a7e9f8989281d062b389291',
 'cases/request-invalid-surrogate-unused-value.args': '53fee69f1b7506dd35326d77c7d591967b50c42939b717b2148d793edc5764eb',
 'cases/request-invalid-trailing-comma.args': 'b2c6c4e44ffe056542e87fc8bc67c43c5f93dbe41886be8ff654ac7686883abe',
 'cases/request-invalid-unclosed.args': 'c7d862dc663b56992f7f40f45aa6c8c4cf1c94efd189d53350919d60383d7077',
 'cases/request-invalid-whitespace.args': 'c69f67192d39e27d981e79deea6742226b8618e23c889fadf29a0bc9b7d930b7',
 'cases/request-invalid-zero-width-outside.args': 'be02db8603ab37a3165f1478b13fa1d0e23958019dd8fc06b7734a00286d5429',
 'cases/root-array.args': '31df26c3e34ac1e00ebfe4407dc364ec7f9e3d913fd3e76d4f6d26066015c9fb',
 'cases/root-empty-array.args': 'd9b2a86fdf23de3345409bf47a7a7bc65e79de2fc2e1f211def8fb9a3d6e4c8a',
 'cases/root-empty-object.args': '393877ce9a0da409d938f1121650282bdbe7ec8acafb5b6f64100a80d80c6daa',
 'cases/root-empty-string.args': 'b30edc03c245912186aeda3047b3b991d943cfa580aa2040075e914dfb485378',
 'cases/root-false.args': 'c329fc4c041bfa9fb0f304d8ebec3a575196469df5048ce2748d636f21264fe4',
 'cases/root-null.args': 'e82b809593c4f5d911011c86d6b524cd8c5e12567bad5942be59506c4b84e62c',
 'cases/root-number.args': '9b61a905e1a8676746ac8bfbd08cbba139f74a2319a31797511622f7d2feec50',
 'cases/root-object.args': '96dd451be592bd29e1bcdebf0b3ca2517d63282123fe2e36206d45fd60d8ff59',
 'cases/root-string.args': '27c5b5159d4fbb13b93a60aa59d4a4572fe91523f7d4b19492c16c333c030806',
 'cases/root-true.args': 'c1632117086f8cea20da9edd4d0b77285e1dd09b22dc2de3d759a3b65460852a',
 'cases/seeded-tree-000.args': '393877ce9a0da409d938f1121650282bdbe7ec8acafb5b6f64100a80d80c6daa',
 'cases/seeded-tree-001.args': 'e82b809593c4f5d911011c86d6b524cd8c5e12567bad5942be59506c4b84e62c',
 'cases/seeded-tree-002.args': 'c81290aab6b49c5186f55de7a57fdde29e8ec993e0e75095188efd6fdab8a707',
 'cases/seeded-tree-003.args': 'bd1e9c1dd0b343e20dfd9af8e0ce1730c2e09dfd9aee81c9a5b30e1f34d02489',
 'cases/seeded-tree-004.args': '393877ce9a0da409d938f1121650282bdbe7ec8acafb5b6f64100a80d80c6daa',
 'cases/seeded-tree-005.args': '2ec72eacb4d4a7e3b15dcc3648d10b5501343ec8116535ce43041b57cfae3608',
 'cases/seeded-tree-006.args': '6c49ac254e22f27d18bd57d4a89536ccced2f271f53339322f4d5a1cc79825f2',
 'cases/seeded-tree-007.args': 'd057b726a33a75ae5a9012e1f45b552e8ecf6a079880ecee49f9e7e49483cd93',
 'cases/seeded-tree-008.args': 'e82b809593c4f5d911011c86d6b524cd8c5e12567bad5942be59506c4b84e62c',
 'cases/seeded-tree-009.args': '9b61a905e1a8676746ac8bfbd08cbba139f74a2319a31797511622f7d2feec50',
 'cases/seeded-tree-010.args': 'df089771f5543ca3bac2d34b0679376bad251dd2cf5c0c0d995631300d313b62',
 'cases/seeded-tree-011.args': '079d9ccd149379246821314bf1f70ff5256f6ce5c85c4b1a416a031a15a357ed',
 'cases/seeded-tree-012.args': '9b61a905e1a8676746ac8bfbd08cbba139f74a2319a31797511622f7d2feec50',
 'cases/seeded-tree-013.args': '6c3ade8f2329f5cf2b99602cde696b86c99aaabde7c2f766dd0c0ea54f139076',
 'cases/seeded-tree-014.args': '60c85510a5bfeab41a038f2a0e9e25aba084a433fb8fce72340349e2b46a4fb5',
 'cases/seeded-tree-015.args': '217753a952a87fe81f1b8311a980078fcedc9f00668c86e995be44853ce570a7',
 'cases/seeded-tree-016.args': 'e0c850858550e3f4dc449250cfa2c713ae91326379e503d87ee73045b3665676',
 'cases/seeded-tree-017.args': 'f676ef2c88b9c10dba3d3d2a50fa07284b56150eca04a2840cb6824c5a6475b5',
 'cases/seeded-tree-018.args': '7b2015da2d1e27640d95e16ad8d12399fef0b29d268b68f9e376d8b416cd279e',
 'cases/seeded-tree-019.args': 'c1632117086f8cea20da9edd4d0b77285e1dd09b22dc2de3d759a3b65460852a',
 'cases/seeded-tree-020.args': '63cc7048c48fa6c159ba9db21fa60aa15ba9985b08794d445c43b173a9977c5f',
 'cases/seeded-tree-021.args': '393877ce9a0da409d938f1121650282bdbe7ec8acafb5b6f64100a80d80c6daa',
 'cases/seeded-tree-022.args': 'e2271fde3a81fac7b1063b5b0ad904b0a0f99971ff536a9088eb6786e7e57325',
 'cases/seeded-tree-023.args': '8ff96b44f4b60dbb1ee0fe12932a214e9422ca9e72ae7dde3dca5351f8806592',
 'cases/seeded-tree-024.args': '8dddeebac065410d697522a9093e2bc945d4d0a896e8508603a08df86844f3d9',
 'cases/seeded-tree-025.args': '7b2015da2d1e27640d95e16ad8d12399fef0b29d268b68f9e376d8b416cd279e',
 'cases/seeded-tree-026.args': '1ca61b62b2948a211b401da8ce18fdcad92274b4c888249d5eb1bf39231ca087',
 'cases/seeded-tree-027.args': 'b78086ae9b40916a1ea19f9571ff21251b0920b5981d56af731919ccb621b7cd',
 'cases/seeded-tree-028.args': 'd9b2a86fdf23de3345409bf47a7a7bc65e79de2fc2e1f211def8fb9a3d6e4c8a',
 'cases/seeded-tree-029.args': 'a72e6477548d357237fdbcbd337e1e0e043e738982361627eb077395ae283483',
 'cases/seeded-tree-030.args': 'fcd713b7ee3e57d2874b4d41e0ffb391819afd6e672e2d8891aae50355efdc27',
 'cases/seeded-tree-031.args': '393877ce9a0da409d938f1121650282bdbe7ec8acafb5b6f64100a80d80c6daa',
 'cases/seeded-tree-032.args': 'd9b2a86fdf23de3345409bf47a7a7bc65e79de2fc2e1f211def8fb9a3d6e4c8a',
 'cases/seeded-tree-033.args': 'c329fc4c041bfa9fb0f304d8ebec3a575196469df5048ce2748d636f21264fe4',
 'cases/seeded-tree-034.args': 'a637088fce1d3839231aff9bb6c5bbc1e046477b8286843bc4da9cd6daf26622',
 'cases/seeded-tree-035.args': 'cf6f36566fa68c1786fb23545d07ea03ac8eaac324ca7e20dff54634fd90f282',
 'cases/seeded-tree-036.args': '6c49ac254e22f27d18bd57d4a89536ccced2f271f53339322f4d5a1cc79825f2',
 'cases/seeded-tree-037.args': 'b6ac222874e853a0c647c042967e59ba2c3b37b38e4d02883e1da76b889b331e',
 'cases/seeded-tree-038.args': 'd55ace1c5ff705e33d3f1aaff5ad294c1d8bf92bb23f701276b5ba93def50cc3',
 'cases/seeded-tree-039.args': '3c7096bd645d2ce4c6bf5e0c171025e055d13c02e1c0f1a660e0f76ebb7bf362',
 'cases/spec-duplicate-a.args': '1b8394c73e8b749be48c0306db8d39c014086ed497897de6598d522940908561',
 'cases/string-structure-delimiters.args': 'c00744cd72bec3159f817d75b324e37c488cc25019f6e27375000a9765e1a79b',
 'cases/unicode-normalization-case.args': '46eb74a5c9c4ac2a3e048e7d705104f4f278c8209880566f59ae36a5e0c601e0',
 'cases/unicode-special-contents.args': '8a0ca3fe7d064af7605857fee769ce6e1ee2ba25b62e72c9bfb0a002175e1540',
 'cases/wrapper-escaped-selected.args': 'a3086c1b79c3e8c7e9081cb12290f9c9740f0b55a00b97909ff0ff5990c80d4e',
 'cases/wrapper-huge-ignored-numbers.args': '027767c68a6715f1036e95dbe18f448566de409db6fbe95db93c93c33c6f57bc',
 'cases/wrapper-ignored-duplicates.args': '177ef17defd7e38dfff6dd075fa1a2b541ef3342fefa45b06efe5ff6cbf5f4b0',
 'cases/wrapper-nested-json-ignored.args': '47e4609e0edbd77f98fd50dd71777463576786bf10cf7e5c8de185c949bcb3d9',
 'cases/wrapper-unrelated-key-case.args': 'c9ecc358264c700934cf79e496ac528d0fb70e30321200f31423a2e038228eb0',
 'intent.md': 'f013a166f9af3f3690091560b534fd0c648530d723f216e4363726c18d289d8c',
 'reference.py': 'c1cb174ad80613fdd813f29cf7d2397504fe35acbafaa3e7089e5330208bfe5b'}
FROZEN_LIBRARY = {'examples/ail_std_json_pretty.ail/README.md': 'f714b6b3e6ee0a9e5faf4abdd0ba0718a1c798647b703b95e4b712eccfd30189',
 'examples/ail_std_json_pretty.ail/ail-package.md': '7a41304151d6653d25dcfed853c077d88da054220aa6f458d488fccdc74efccd',
 'examples/ail_std_json_pretty.ail/spec.ail-spec.md': '4014da002a43f1cb78696539f3d0d75be363da2c01beb56abd17b00534d5cce3'}


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
    assert len(members) == 164
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
    return result


def checked(command):
    r = invoke(command)
    assert r.returncode == 0 and r.stderr == b'', (command, r.returncode, r.stderr[-1500:])
    return r.stdout


def inspect(program):
    assert program['external_bindings'] == [] and program['capability_grants'] == []
    helpers = set()
    for action in program['actions']:
        pure = action['action'].startswith('Pretty.')
        if pure:
            helpers.add(action['action'])
            assert action['instructions'][0]['opcode'] == 'VALUE_FUNCTION'
        for instruction in action['instructions']:
            opcode = instruction['opcode']
            assert not opcode.startswith(('PROCESS_', 'HOST_', 'FILE_', 'DIRECTORY_'))
            assert opcode not in ('CALL_EXTERNAL', 'EVAL_EXPRESSION')
            if opcode.startswith(('STDIN_', 'STDOUT_', 'STDERR_')):
                assert action['action'] == 'Format' and opcode in ('STDIN_READ_TEXT', 'STDOUT_WRITE_TEXT')
            if pure:
                assert opcode not in ('ACTION_BEGIN', 'EMIT_TRACE', 'OUTPUT_FIELD', 'RETURN_SUCCESS')
                destination = instruction['operands'].get('key')
                assert destination is None or destination.startswith('local.'), instruction
                if opcode == 'CALL_VALUE':
                    assert instruction['operands']['target'].startswith('Pretty.'), instruction
    assert helpers == {'Pretty.quotedEnd', 'Pretty.scalarEnd', 'Pretty.render',
                       'Pretty.renderText', 'Pretty.renderRequest'}
    COUNTS['ir_inspections'] += 1


def build(root, package, stem, actions, full=True):
    frontends = [('source', [package])]
    if full:
        core = root / (stem + '.core')
        core.write_bytes(checked([AIL, 'ail-core', package]))
        rendered = root / (stem + '-rendered')
        rendered.mkdir()
        (rendered / 'ail-package.md').write_text(
            '# AIL Package\nname: pretty-rendered\nversion: 0.1.0\nprofile: Application\n'
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


def request_trace(text, wanted):
    if dict(wanted.pairs)['ok']:
        return ''
    if dict(wanted.pairs)['error'] == 'JSON':
        return ' -> recovered JSON into local.error'
    try:
        wrapper = ORACLE.parse(text)
    except ValueError:
        return ' -> recovered JSON into local.error'
    selected = [v for k, v in wrapper.pairs if k == 'json'] if isinstance(wrapper, ORACLE.Object) else []
    category = 'JSON-PATH' if isinstance(wrapper, ORACLE.Object) and len(selected) != 1 else 'JSON'
    return f' -> recovered {category} into local.error'


def selected(result, wanted_bytes, route, action, recovery=''):
    state = b'out.result=hex:' + wanted_bytes.hex().encode() + b'\n'
    steps = [f'action {action} started']
    if action == 'Analyze':
        steps += ['formatted JSON data.report']
    elif action == 'Render':
        steps += ['parsed JSON data.value']
    steps += ['computed bytes out.result', 'trace Rendered']
    if route[3]:
        stdout, stderr = state, ('\n'.join(steps) + '\n').encode()
    else:
        prefix = 'ail-vm succeeded\n' if 'ail-vm' in list(map(str, route[1])) else 'ail-run succeeded\n'
        trace = steps[0] + recovery + ' -> ' + ' -> '.join(steps[1:])
        stdout, stderr = prefix.encode() + state + ('trace=' + trace + '\n').encode(), b''
    assert result.returncode == 0 and result.stdout == stdout and result.stderr == stderr, (
        route[0], action, result.returncode, result.stdout[:700], stdout[:700], result.stderr[-1500:], stderr)
    COUNTS['checks'] += 1


def exact(result, wanted, context):
    assert result.returncode == 0 and result.stderr == b'' and result.stdout == wanted, (
        context, result.returncode, result.stdout[:500], wanted[:500], result.stderr[-1500:])
    COUNTS['checks'] += 1


def fatal(result, category, context, private=()):
    assert result.returncode > 0 and result.stdout == b'', (context, result.returncode, result.stdout[:700])
    if category == 'FUEL':
        assert b'AIL bytecode execution step limit exceeded' in result.stderr, (context, result.stderr[-1500:])
    else:
        assert ('AIL-RUNTIME-' + category + ':').encode() in result.stderr, (context, result.stderr[-1500:])
    assert all(secret not in result.stdout + result.stderr for secret in private), context
    COUNTS['checks'] += 1


def probe_package(root):
    package = root / 'direct-probe'
    package.mkdir()
    (package / 'ail-package.md').write_text(
        '# AIL Package\nname: pretty-direct-probe\nversion: 0.1.0\nprofile: Application\n'
        'entry: spec.ail-spec.md\nfeatures: things, actions, functions, text, bytes, json, expressions, control-flow, traces\n'
        f'imports: {LIBRARY} compatible ^0.1 as Pretty\nconformance: first-slice\n')
    (package / 'spec.ail-spec.md').write_text('''The application PrettyDirectProbe manages exact rendering tests.
A Data has:
- text: Text
- value: Json
- pretty: Text
An Out has:
- result: Bytes
Action: Render.
When Render happens:
- the system parses JSON text data.text into data.value
- the system calls Pretty.render with value = data.value into data.pretty
- the system encodes text data.pretty as UTF-8 bytes into out.result
- the system returns out.result
- the system records a trace event named Rendered
''')
    return package


def smoke(routes, probes):
    vectors = [('success', '{"json":"{\\"a\\":[1,{},[]],\\"a\\":-0}"}'),
               ('REQUEST', '{"json":1}'), ('JSON', '{"json":"["}')]
    for route in routes:
        for label, text in vectors:
            wanted = ORACLE.evaluate_request(text)
            selected(run(route, 'Analyze', ['data.request=' + text]), encoded(wanted),
                     route, 'Analyze', request_trace(text, wanted))
            COUNTS['ordinary_app_smokes'] += 1
        exact(run(route, 'Format', input=b'{"a":1,"a":-0}', stdio=True),
              b'{\n  "a": 1,\n  "a": -0\n}\n', (route[0], 'Format-smoke'))
        COUNTS['raw_smokes'] += 1
    for route in probes:
        document = '{"a":1,"a":-0}'
        selected(run(route, 'Render', ['data.text=' + document]),
                 b'{\n  "a": 1,\n  "a": -0\n}', route, 'Render')
        COUNTS['ordinary_direct_smokes'] += 1
    (ROOT / 'frontloaded-smokes.json').write_text(json.dumps(
        {'counts': dict(COUNTS), 'commands_through': len(RECORDS), 'exact_reports': True}, indent=2) + '\n')


def fixed_cases():
    cases = []
    for path in sorted((RUNG / 'cases').glob('*.args')):
        state = dict(line.split('=', 1) for line in path.read_text().splitlines()
                     if line.strip() and not line.lstrip().startswith('#'))
        assert set(state) == {'data.request'}
        text = state['data.request']
        cases.append((path.stem, text, ORACLE.evaluate_request(text)))
    assert len(cases) == 162
    return cases


def fixed(routes, cases):
    (ROOT / 'frozen-expectations.json').write_text(json.dumps(
        [{'case': label, 'expected_hex': encoded(wanted).hex()} for label, _, wanted in cases], indent=2) + '\n')
    for route in routes:
        for label, text, wanted in cases:
            selected(run(route, 'Analyze', ['data.request=' + text]), encoded(wanted),
                     route, 'Analyze', request_trace(text, wanted))
            COUNTS['frozen_requests'] += 1
        print('frozen', route[0], len(cases), flush=True)


def direct_vectors(cases):
    vectors = []
    for label, text, wanted in cases:
        if dict(wanted.pairs)['ok']:
            wrapper = ORACLE.parse(text)
            document = next(v for k, v in wrapper.pairs if k == 'json')
            vectors.append((label, document, dict(wanted.pairs)['text'].encode()))
    # Independent hand calculations are checked against the frozen oracle first.
    for label, document, pretty in [
            ('hand-null', 'null', 'null'), ('hand-empty', '{"a":[],"a":{}}', '{\n  "a": [],\n  "a": {}\n}'),
            ('hand-number', '[1.2500e-2,1E+0003,-0]', '[\n  1.2500e-2,\n  1E+0003,\n  -0\n]')]:
        assert ORACLE.render_text(document) == pretty
        vectors.append((label, document, pretty.encode()))
    for label, text in [('quote-window', 'x' * 255 + '"' + 'y' * 260),
                        ('slash-window', 'x' * 254 + '\\"' + 'z' * 257),
                        ('even-slashes', '\\' * 128 + '"[,]{}:'),
                        ('escaped-quotes', '"' * 512)]:
        value = ORACLE.Object([(text, text), (text, ORACLE.Number('1E+0003'))])
        vectors.append((label, ORACLE.encode(value), ORACLE.render(value).encode()))
    assert len(vectors) == 89
    return vectors


def direct(routes, vectors):
    (ROOT / 'direct-expectations.json').write_text(json.dumps(
        [{'case': label, 'document': text, 'expected_hex': wanted.hex()} for label, text, wanted in vectors],
        ensure_ascii=False, indent=2) + '\n')
    for route in routes:
        for label, text, wanted in vectors:
            selected(run(route, 'Render', ['data.text=' + text]), wanted, route, 'Render')
            COUNTS['direct_render'] += 1
        print('direct render', route[0], len(vectors), flush=True)


def io(routes, vectors):
    values = [(label, text.encode(), wanted + b'\n') for label, text, wanted in vectors]
    more = [('already-pretty', b'{\n  "a": [\n    1,\n    {}\n  ]\n}\n'),
            ('wrapper-is-object', b'{"json":"[1,2]","json":null}'),
            ('one-MiB-total', b'null' + b' \t\r\n' * ((LIMIT - 4) // 4)),
            ('sample', (PACKAGE / 'input.json').read_bytes())]
    values += [(label, text, ORACLE.render_text(text.decode()).encode() + b'\n') for label, text in more]
    assert len(more[2][1]) == LIMIT
    assert (PACKAGE / 'expected.json').read_bytes() == values[-1][2]
    private = (b'PRIVATE-PRETTY-INPUT',)
    bad = [('empty', b'', 'JSON'), ('whitespace', b' \t\r\n', 'JSON'),
           ('invalid-UTF8', b'\xff' + private[0], 'TEXT'),
           ('malformed', b'{"PRIVATE-PRETTY-INPUT":', 'JSON'),
           ('invalid-Unicode-key', b'{"\\ud800":"PRIVATE-PRETTY-INPUT"}', 'JSON'),
           ('invalid-Unicode-value', b'{"PRIVATE-PRETTY-INPUT":"\\udfff"}', 'JSON'),
           ('NBSP-whitespace', b'null\xc2\xa0', 'JSON'),
           ('two-documents', b'{"PRIVATE-PRETTY-INPUT":1}\nnull', 'JSON'),
           ('one-MiB-plus-one', b'null' + b' ' * (LIMIT - 3), 'TEXT-LIMIT'),
           ('depth-65', b'[' * 65 + b'0' + b']' * 65, 'JSON-LIMIT'),
           ('fuel-4000', b'[' + b','.join([b'0'] * 4000) + b']', 'FUEL'),
           ('allocation-expansion', ('[' + ','.join(['"' + 'x' * 4096 + '"'] * 128) + ']').encode(), 'TEXT-LIMIT')]
    assert len(bad[-1][1]) == 524673
    (ROOT / 'stdin-vectors.json').write_text(json.dumps(
        {'success': [{'case': label, 'bytes': len(text), 'stdin_sha256': digest(text),
                      'expected_sha256': digest(wanted)} for label, text, wanted in values],
         'failure': [{'case': label, 'bytes': len(text), 'category': category,
                      'stdin_sha256': digest(text)} for label, text, category in bad]}, indent=2) + '\n')
    for route in routes:
        for label, text, wanted in values:
            exact(run(route, 'Format', input=text, stdio=True), wanted, (route[0], label))
            COUNTS['raw_stdin_successes'] += 1
        fatal(run(route, 'Format', input=private[0]), 'CAPABILITY', (route[0], 'missing-stdio'), private)
        COUNTS['raw_stdin_failures'] += 1
        for label, text, category in bad:
            fatal(run(route, 'Format', input=text, stdio=True), category, (route[0], label), private)
            COUNTS['raw_stdin_failures'] += 1
        print('raw stdin', route[0], len(values), len(bad) + 1, flush=True)


def mutation(root):
    library = root / 'mutated-library.ail'
    shutil.copytree(LIBRARY, library)
    source = library / 'spec.ail-spec.md'
    operation = '- the function assigns "  " to local.spaces'
    replacement = '- the function assigns " " to local.spaces'
    original = source.read_text()
    assert original.count(operation) == 1
    source.write_text(original.replace(operation, replacement))
    package = root / 'mutated-app'
    shutil.copytree(PACKAGE, package)
    manifest = package / 'ail-package.md'
    manifest.write_text(manifest.read_text().replace('../ail_std_json_pretty.ail', str(library)))
    text = '{"json":"{\\"a\\":[1,{},[]],\\"a\\":-0}"}'
    wanted = ORACLE.evaluate_request(text)
    for route in build(root, package, 'mutation', ['Analyze'], full=False):
        result = run(route, 'Analyze', ['data.request=' + text])
        state = next(line[len(b'out.result=hex:'):] for line in result.stdout.split(b'\n')
                     if line.startswith(b'out.result=hex:'))
        actual = bytes.fromhex(state.decode())
        decoded = ORACLE.parse(actual.decode())
        selected(result, actual, route, 'Analyze')
        assert actual != encoded(wanted), 'independent oracle accepted a real layout mutation'
        assert dict(decoded.pairs)['ok'] is True and dict(decoded.pairs)['error'] == ''
        (root / ('mutation-' + ('native' if route[3] else 'VM') + '.json')).write_text(json.dumps(
            {'operation': operation, 'replacement': replacement, 'status': result.returncode,
             'expected_hex': encoded(wanted).hex(), 'actual_hex': actual.hex(),
             'independent_mismatch': True, 'reports_pass': True}, indent=2) + '\n')
        COUNTS['semantic_mutations_detected'] += 1
    print('successful wrong bytes rejected on VM/native', flush=True)


def main():
    global ROOT, ORACLE
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--artifact-dir', type=Path, help='retain actual commands, I/O and artifacts')
    args = parser.parse_args()
    before = frozen()
    deps = dependencies()
    compiler = digest(AIL.read_bytes())
    spec = importlib.util.spec_from_file_location('pretty_independent_oracle', RUNG / 'reference.py')
    ORACLE = importlib.util.module_from_spec(spec)
    sys.modules[spec.name] = ORACLE
    spec.loader.exec_module(ORACLE)
    assert ctypes.CDLL(None).prctl(36, 1, 0, 0, 0) == 0
    temporary = None
    if args.artifact_dir:
        ROOT = args.artifact_dir.resolve()
        ROOT.mkdir(parents=True, exist_ok=True)
        assert not any(ROOT.iterdir()), 'artifact directory must initially be empty'
    else:
        temporary = tempfile.TemporaryDirectory(prefix='ail-pretty-')
        ROOT = Path(temporary.name)
    (ROOT / 'io').mkdir()
    start = time.monotonic()
    try:
        probe = probe_package(ROOT)
        early = build(ROOT, PACKAGE, 'smoke-app', ['Analyze', 'Format'], full=False)
        early_probe = build(ROOT, probe, 'smoke-probe', ['Render'], full=False)
        smoke(early, early_probe)
        # Prove actual raw fatal diagnostics and semantic mutation before matrices.
        io(early, [])
        mutation(ROOT)
        (ROOT / 'focused-preflight.json').write_text(json.dumps(
            {'counts': dict(COUNTS), 'commands_through': len(RECORDS)}, indent=2) + '\n')
        routes = build(ROOT, PACKAGE, 'app', ['Analyze', 'Format'])
        probes = build(ROOT, probe, 'probe', ['Render'])
        assert len(routes) == len(probes) == 14
        cases = fixed_cases()
        vectors = direct_vectors(cases)
        fixed(routes, cases)
        direct(probes, vectors)
        io(routes, vectors)
        assert frozen() == before and dependencies() == deps
        assert digest(AIL.read_bytes()) == compiler
        summary = {'status': 'PASS', 'counts': dict(COUNTS), 'app_routes': len(routes),
                   'direct_routes': len(probes), 'commands': len(RECORDS),
                   'seconds': round(time.monotonic() - start, 4), 'frozen': FINGERPRINT,
                   'oracle_members': before, 'dependencies': deps, 'compiler_sha256': compiler}
        (ROOT / 'summary.json').write_text(json.dumps(summary, indent=2) + '\n')
        print(json.dumps({k: v for k, v in summary.items()
                          if k not in ('oracle_members', 'dependencies')}, sort_keys=True), flush=True)
    except BaseException as error:
        (ROOT / 'failure.json').write_text(json.dumps(
            {'exception': repr(error), 'counts': dict(COUNTS), 'commands': len(RECORDS)}, indent=2) + '\n')
        raise
    finally:
        (ROOT / 'commands.json').write_text(json.dumps(RECORDS, ensure_ascii=False, indent=2) + '\n')
        if temporary:
            temporary.cleanup()


if __name__ == '__main__':
    main()
