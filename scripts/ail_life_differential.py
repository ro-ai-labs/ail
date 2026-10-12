#!/usr/bin/env python3
"""Independent Life acceptance: fourteen app and fourteen direct API routes.

Use an existing target/debug/ail. Every frozen request is tested through Analyze
and actual raw stdin. Artifact destinations must be fresh or empty, and are locked
before any child or evidence write. The bounded supervisor retains failed attempts.
Copy this file to an immutable attempt-specific path before invocation; set
AIL_LIFE_REPO to the repository root when executing such a copy.
"""
import argparse, collections, ctypes, fcntl, hashlib, importlib.util, json, os
from pathlib import Path
import shutil, sys, time
sys.dont_write_bytecode=True
REPO=Path(os.environ.get('AIL_LIFE_REPO',Path(__file__).resolve().parent.parent)).resolve()
sys.path.insert(0,str(REPO/'scripts'))
import ail_process_differential as harness
AIL=REPO/'target/debug/ail'
PACKAGE=REPO/'examples/conway-life'
LIBRARY=REPO/'examples/ail_std_life.ail'
RUNG=REPO/'bench/R025-conway-life'
FINGERPRINT='sha256:ce9f086748bbdd3abd28961f1e4f53ae'
COMPILER_SHA256='22bf54b8ea97847d52e6802d3f167162747137d1d4c92b8651cca303c9ad295d'
LIMIT=1048576
COUNTS=collections.Counter()
RECORDS=[]
ROOT=None
ORACLE=None
DRIVER=Path(__file__).resolve()
DRIVER_SHA=None
FROZEN_ORACLE={'cases/anchor-blinker-even.args': '02c067ef7cb3eb0d51b51074c1f270a0eb82462d31a5ce24a080429482ccc983', 'cases/anchor-blinker-odd.args': 'e9eac37fee9665f2b6b2a204a569bf53aeae5bc44e5f0a6b1ad1c0249756d2de', 'cases/anchor-block.args': '91067ab3591e320d5db173a3f492fd369dfe393b2ddfe23eea7d891be96607b1', 'cases/anchor-corner-birth.args': 'c6ec640b5245be88073f6847054764bb73f680ffb176b79ae498e5cf8e52527d', 'cases/anchor-dead-single.args': 'f927a908203da4706a76a29d2c162218a1e307757eb98b0d8a866124103d2e64', 'cases/anchor-live-single.args': 'f10ed52b44a9f6395e9fe8219e8ca6fd2aaebbc1f2c471a7599e28bb16c7cea3', 'cases/duplicate-cells-escaped.args': '90b11834b08571c22aa458d16afca0bfb5e0e5010e322595f3bb9f1cef6b9b51', 'cases/duplicate-cells-plain.args': 'ce27bc1c279da5fc62a43021b98706a99633704a99fb810cb6e6aae0da318714', 'cases/duplicate-generations-escaped.args': 'ccd02365ab06c213435fe8f053dec9d1197c8124b99869629592da6fcc771309', 'cases/duplicate-generations-plain.args': '78f8ffecc898e8bf5f8eebad8b32577d8e930465974800a1fe1a959c35058387', 'cases/duplicate-height-escaped.args': '059f3331cd72f8d9992887b25e23973b5da57790356512da63130cd5b8db22dc', 'cases/duplicate-height-plain.args': 'd7125bc51efa03850109bdf2bb6e784d4c11a99ab14c341ee4de99371a214942', 'cases/duplicate-width-escaped.args': '334b086cd8a0f61a1c28542c996c507b9fa9c01de35865d4ee820fe6de739141', 'cases/duplicate-width-plain.args': '28bc04e991ca70a28e032a6483d64d8e35ea3b82fbbef7a50d0a573eb253100c', 'cases/escaped-selected-once.args': 'a8a33091c566673e22975c8ec6a0355cbc92272978ad6ffffb5efd354ce18384', 'cases/generations-i64-min.args': '85567892124aca975e791389bcb8ba9fa57b0495d57d9ba2874e26cb347c6cdf', 'cases/generations-negative.args': '98c6154f93f2982da539e0e7b2be987a23e36bab55bfd8b60fb7b453af29f33c', 'cases/grid-cell-i64-max.args': 'bfac88070dab67ccb729673d0993c8948cfb9bf6faf6b5169128e382354f18c7', 'cases/grid-cell-i64-min.args': '1980633d79dc0b8675ab640eee65b6d1f893d33f919617ab49178a7c6a0a5d9c', 'cases/grid-empty.args': 'b777ba55418b12bdf8c9df73e356837e9b6c7496e690f3b27f746825cbc35751', 'cases/grid-generations-max-with-grid.args': 'dedeb820d64efde25221e8949d39a5bffb889c099ea3dd37313ecb42b8dfa65d', 'cases/grid-height-i64-max.args': 'dae26bc21bf68b6b7648bf6a58555bba073d9513acb1cacca43fc49784133843', 'cases/grid-height-i64-min.args': '1f3b128c630e38ec362ca3526e54f7e67cbf570b4ff3069a419717c8b8a49306', 'cases/grid-long.args': 'dac11ade9b6f1c4324a2882a63b8fbb19fce8e533b906d7fa790c47a454298d1', 'cases/grid-negative-height.args': 'd1422474b2e9487fe012082cac77451e0c28865c39e7d3dbe4ed521ad20bbc6d', 'cases/grid-negative-width.args': 'fee22ac7bdc07b8e058ab61c1c814c68bd4befd2f3f2f45852a487b0f10ffae1', 'cases/grid-nonbit-negative.args': '2272c97b0224d336d1318dc374e3f5bf52ea1507aea4efec3248a861caa40979', 'cases/grid-nonbit-positive.args': '221db37bef50d22b638b41c9b5a1bc916e727a0857f2b99904d1376431955b51', 'cases/grid-oversized-both.args': 'bcbf32f09ec8b6f98b6c2904325bf9263136b2d47e2592ba01fddc25726a939c', 'cases/grid-oversized-product.args': '03c58aeace7b78b669b3612573756c395ee5904370e8172428f5d1d917bc8f56', 'cases/grid-short.args': '93757aa39a60514980411315898512f91ee090d68328f7a4cbd9e82202684af4', 'cases/grid-width-i64-max.args': 'cf7719443634f7bd6a711ac0129c5068d7d70343fc819ab62705b238cfe469ac', 'cases/grid-width-i64-min.args': 'da8d75ff26867d46dab3e3984f2dcfb33976387293e91d9ebd3465e41fede0a9', 'cases/grid-zero-height.args': 'aecc84da18d4799e6f1ba933fa1734716a39963e242f6fd2c164120019d0e80f', 'cases/grid-zero-width.args': '000a9db3dbff7fd840f526eb5d4412e56a180497313ace6fa43a2c55ca241178', 'cases/ignored-deep-array.args': '3c0bb8db0a7b861f79e9db059abf719fe1373f74c59e295166e0b6098d7aa9ad', 'cases/ignored-deep-object.args': '824dd0ecac9f64ea2af2b038ccbb5be1474cd68cbdc73665feb1691a70d9927c', 'cases/ignored-duplicates.args': '2d46795516cfd75590a9f6fd9811395985f757a7cc85c27b780b95567c859c00', 'cases/ignored-exponent-5000.args': '6f66a7614edb0ccc3116b6d485040e87c6eec08818effcf2380af9e7debde60b', 'cases/ignored-fake-state-separators.args': '391c2231e204313250a3d6385b16397ca6d9f8102c71b0271b758ff2b2c214be', 'cases/ignored-fraction-5000.args': '975f7b413d057635a76536badf17297a412bbfe9139efe8949f4e0a843c87737', 'cases/ignored-invalid-5000-number.args': 'f19d284e04767b65fc9f29ab94a945890e689947ce941ccc6d7f8f5efbb620e9', 'cases/ignored-negative-number-5000.args': '7798711ef0e1ca8e53d9746eda4c7f5cdadfca052a9156b2842c2a4cf53a82ab', 'cases/ignored-nested-selected.args': '7b766e46e965278ed65a97d4778b744f02fb922d6bfa71980ab2f548fc270562', 'cases/ignored-nul-content.args': 'f4cb923675688eecd6bba6207dcdfc0e61b8b768855812550988fa7ae7514797', 'cases/ignored-number-5000.args': 'b980e15bba71f4e5cf27d0747dc170596f80d6aeccb9ac1ae62ae08344769bdf', 'cases/ignored-unicode-content.args': 'aaf1a1ce79cb74a97b5b412e65436a27387685b0af5df527746e6f27e1a25bb9', 'cases/ignored-unicode-pair.args': 'c9a981dd82f5f45ef0209991f1893c0a7b4dc69efd16e13e31e90b020753cf94', 'cases/inplace-distinguishing.args': 'e9eac37fee9665f2b6b2a204a569bf53aeae5bc44e5f0a6b1ad1c0249756d2de', 'cases/missing-cells.args': 'e6df4ae1cdb9d206d90602a6888c4167607452f1dfdc78c0964779fc0867528a', 'cases/missing-generations.args': '4c7355521079c3ba8248f3636d364989c82263c9e553435040efec713cf16c19', 'cases/missing-height.args': '97737e5b9ad506c833b3857bfd85a0292efb50cb92d9e4a830a59788ef962f62', 'cases/missing-width.args': '7f144c966c15fe7e03b469760c6aca3240136dc74a6b9b1ca3b4236e46b28e6a', 'cases/negative-zero-cells-generations.args': '12e9b44da6d9d195d564ce353d5a05ea5a5105910ce34b1414c8e61972ef18ee', 'cases/negative-zero-height.args': '6b5176e487e7092575c1fc5bfdea6c68dd9e3db661e738689cbd97275432c53d', 'cases/negative-zero-width.args': '1cf82429ce301c60761f670d0f9bc61f1fafba8d7e226c282d597b3161388d14', 'cases/priority-generations-max-request.args': 'ca64dd7d4e6000a2061cb2f1436f7190d97ea3b65aca0b5c11b2c0dea31a2729', 'cases/priority-grid-generations.args': 'f37e448749ef467a38fe07958d5ac2fcd44e8106494d1909f16bb0a0b739d5cc', 'cases/priority-nonbit-generations.args': 'c5458e96d18c2d4f8ed3b25c7623c5b64d82056e1d54fbe91871226d4a552792', 'cases/priority-request-grid-generations.args': '22099f4f999ed6c770f575ba4b70f2338accf2e618a8ee6b0b8a5fc4cc947544', 'cases/priority-syntax-grid-generations.args': 'b50b1277e90f7b8647353ee62e2e7c36020af3a787fac018a1cdd831b049b46b', 'cases/rectangular-even.args': 'f4d2f4209730a83cb5a9b5b92323ed34170e3d67d58efe92198c35de96424d4d', 'cases/rectangular-odd.args': '8bedd746d61cef12f59eba3a9368a1c6dea74ec69f44c6df1484e1b7e1e65fb8', 'cases/required-all-live-20x20-50.args': '69fcd5891595a50988b5953558a35127f83e83abcb877e378e27e29e1f730faf', 'cases/required-blinker-20x20-50.args': 'f2eb04970cbd9bad2078818c50f9660ad82a6d6d3c94ad10b9d7c002c447430a', 'cases/required-dead-20x20-50.args': 'f67962e5f9384b3c38e513af8fcf9c14065b6b3153b94770d010b4de36b0eeca', 'cases/required-glider-20x20-50.args': 'fb50bbe621fc97421200c71dccf22abc0d9e93970d49af3f83ec224dee7fca74', 'cases/required-mixed-20x20-50.args': 'aae4c81048498d5b40b08d19ef7d33cadb4227b6036f2d5e989a20ff4c52fbb4', 'cases/required-mixed-32x32-10.args': '64a60f0bf1ebe669390a8f0d6ccd871c19b60ef27c345927a0c564dfb468d326', 'cases/required-narrow-128x1-40.args': 'e3ef003377db3d69bb616c56c94328a7266208f8be53fc30e5d83041ad16e274', 'cases/required-narrow-1x128-40.args': 'b216539e80e91f1a9a76e9d8a8ba098e3f2436a4e12cb9d9e1965167f32f0a2a', 'cases/required-single-257.args': '5f67ddd80dcf3551a2a4f55dc454ebe1f651b5b077f5710f6122d958b44b6a4d', 'cases/single-column.args': '7bfe97f21a1e64f5d8b4dc480e71cfcef62652826b7dc2d5103421bafddff268', 'cases/single-row.args': 'e1bdc8d0971f6cb388903c71e1ba22fbb47043360507ee7d502a2c65fcd9ccaa', 'cases/syntax-bad-escape.args': 'ddc9fef28e3881ca8599b73cada68ef4fee1ba62e92380660f212b11c5cf427a', 'cases/syntax-bad-hex.args': '1496748086525ceeb22762a4ce708e0bac66cb011a8716b7f0065e9ea436ff39', 'cases/syntax-bom.args': 'f48845028ea0755a433ec63fe4d9725128b545fb1dfc5713bbe0f224e80831e0', 'cases/syntax-comment.args': 'fb67dc7a77067921cfc5a30810af4078f6826895aa325a7ca06600afc7f299ed', 'cases/syntax-em-space.args': '358a44ac5702d54d3db94883fca5205dc9184dc6e8630c0d2f1aa8d9a258d444', 'cases/syntax-empty.args': '63663e0c4b42d17147f3f605d36343e3d8039f44e0795a6e370f17bb8caea5e6', 'cases/syntax-exponent-missing.args': 'c1705486f387dd3c2276cb74d97f93ceb76f224188fd228f07ac6fbc5dd8ce03', 'cases/syntax-fraction-missing.args': '4219f34dfd4db3b69d819f1e6738aa80576733270bb26ae6f9a682fd0e2b458c', 'cases/syntax-ignored-array-comma.args': '09b46f40f6840c2b1f0038e312438eed865ac0a58ff30ef3ee9b0dbe7b8e1026', 'cases/syntax-ignored-object-comma.args': '14b0a72e8cb4e9c337cbb377e2e1e53965f0aa9dcd48550fa1b5ee5441aa35c0', 'cases/syntax-infinity.args': 'a5e24e9eae9c57ad3b5f679c93be76a418b0160ffbf96ecb5e73f2eb3c0fb7a1', 'cases/syntax-leading-zero.args': '5145f91a414c9f19e46bd42f13758893d6255322291cae23af97c251e1892d89', 'cases/syntax-nan.args': 'cfa1c1676023fa61126f6cbf5da501254295dda7b3c8328bca5db5e463b8ed67', 'cases/syntax-nbsp.args': '90b23cfa5b1145287007fd7ddb10ae528707783da0999e10a3ddaa8bec204788', 'cases/syntax-negative-infinity.args': '35ce3c07aec0874d53e8f2d0bf6d074332a6677e8bba1704a5f6761d29fb832b', 'cases/syntax-plus.args': 'c77e9386a2fc7222078844c21043a0cd0d79eb6b72d7d755549e4f502aa4fed6', 'cases/syntax-raw-control.args': 'ec7e6f9c89998a080e3a935b033cd11d6df9181e2aab4c47fd6acbcb145cead1', 'cases/syntax-root-array.args': '7efde84f2d958bb3f4b9bbf212dce82303ff4873255f292c8fe5c4d8048050a1', 'cases/syntax-root-bool.args': '48b84a2d1b194f15c58a71a9ed4e185652604e26ed062a753b0d38866c1d2f9e', 'cases/syntax-root-null.args': 'f566a587c8dcd8618972ea0e5eb5f4cc4f0ef563869063c22ead2683f6e1606a', 'cases/syntax-root-number.args': 'bc2b6b87c855ee58de8e075229e8451b7ca89fe1bb3cae53af3c5bf49d83d731', 'cases/syntax-root-string.args': '74ed53c5f08506ff5bedf6b28a6328d2daa84803aed4834405605a9470165eea', 'cases/syntax-trailing-comma.args': 'dd927946a9301a12b6e9df084f650c9f18fc0c23af681f8e73f740c722883098', 'cases/syntax-trailing-document.args': 'd7aec990f4555f5489f3d061a36809e535a23d55dc2f533970028af73b48ddac', 'cases/syntax-unclosed.args': '5dae20101fef2ff5619d4e8ccaa8a7292188cf83f14de13f4fbde77167fca293', 'cases/syntax-whitespace.args': 'c69f67192d39e27d981e79deea6742226b8618e23c889fadf29a0bc9b7d930b7', 'cases/type-cells-00.args': '53d99b233e0f2af983c529db9ca1ab22eb92287e51e52559790172d5c5b6fc1b', 'cases/type-cells-01.args': 'bbefd1c0a60c678d81abb9e6f07bc56c40e5014c7ef824f5c01cfb5253d3e7d6', 'cases/type-cells-02.args': '81026ebf1f790181111370c47eb421398d06b99d1a6a4e87ec3ab9f5ef01a59f', 'cases/type-cells-03.args': '7996eafb5804b5dd688bc2e4a99e40dc875867ebddfaa29705948a0fc68e8a46', 'cases/type-cells-04.args': 'dc2e63f7bc9287984226c3d07455a96c3a62db5da73418d795a9dcdacb1330f2', 'cases/type-cells-05.args': '64f139ea9bc57620e764b30c877a303b462608fcb1c1bb16a204c9e628ec87f5', 'cases/type-cells-06.args': 'c4b499de5e54115c4c223f86254a29f14d2095b528bf5f2bc288ae783a171809', 'cases/type-cells-07.args': 'bc59dc509260e896ae7569ae75c5b9e384c098572e38dc186ff97b78b992a448', 'cases/type-cells-08.args': '1dd5cd338c6947a2927226b14dad4ddd6df872f68deece987749d385396711db', 'cases/type-cells-09.args': '7d7b5f47473e7483c2582f36f9f4480d73864c65b6a8eaeb4a3fff6e9da47b01', 'cases/type-cells-10.args': '5af25d6d8a5a7f9220aa3c9f6e091c8559195d4083c20b6747e73f8b348b827b', 'cases/type-cells-11.args': '9855d0672b6122bc0896d1ae4725353869077e9d0278924a6cc631d37d6af6fe', 'cases/type-cells-12.args': 'b3505956eb340efff70adf66baf915572aeab4bca6191b5a9fdafabb35db4899', 'cases/type-cells-13.args': 'c3549514bc6597ceea2fa7073e1756970de30dd46e6cf41de184880ec15c493d', 'cases/type-cells-14.args': 'b3c03a0f915949a83b17ed5ead40a9269fb7cea4425b601d571ebac9c61e971d', 'cases/type-cells-15.args': 'c3c3cd005c6c81ce2fd8a9a620195186a326aed58cdfcaa4a665c4dd3292333c', 'cases/type-cells-16.args': '574dd898b013152415b946764324c8d3f609493d17b0fa53ea48b44f7705ac59', 'cases/type-cells-17.args': '5900a22480a777ebe900eefb492462711e917be9447f79ebde10aac8b6038c33', 'cases/type-cells-18.args': 'b50de7db812bbe0323b24ec6e3cdd7840be5796a2c2f9aac468b9eba8e06fc7d', 'cases/type-cells-19.args': '7b45349fb6e7fd2543562b362789a75eec2ee7799b34b737be3f8da34c5cee1a', 'cases/type-cells-20.args': 'e1ac9c6046104baae180105ee2764f435921e7db63fd8f6514ef5b770574807a', 'cases/type-cells-21.args': '9ef3f293e60fa3b7369f05aa5d8533324932fcd47e884c85edc001f924ea669f', 'cases/type-generations-00.args': 'c0a5f53ccc446d7a065fb7bc9f7383284bb7473400aedc2782215d12abf2788e', 'cases/type-generations-01.args': 'f11cc3bceaa995851fc37f54dce23bfc8fea1c7a0842bbb59a66d5ec8b69767f', 'cases/type-generations-02.args': 'eba8013be616899562afa31308d412d50443be10a9af22f458fabe97a50588ee', 'cases/type-generations-03.args': '576b6a52c3c9f654f576a55d6d1a9b762f0ab770528aa5bbdc9dc76dcd7b4b48', 'cases/type-generations-04.args': '048ef8156aad18947f03de20d578d72118c1d9bc2b88f7c3c620736305de63ca', 'cases/type-generations-05.args': '7c2b9784c8d2645e56d41b0586a094dd83b0870eb8f5c82860bf58ccc8e42eaf', 'cases/type-generations-06.args': 'b6ff9a83726f01472312bb35b8820384738da59bfec1c21e2545a0963d2119e6', 'cases/type-generations-07.args': 'fe804c75f365c3cec837cc68836e59fd70af7c2a12e2a4c1d685e7d24b068cd7', 'cases/type-generations-08.args': '02194e8a301e7af56f8aae0bf5960904a3914037070656e5f4e3c5a26a734abe', 'cases/type-generations-09.args': '5d3cc23981b4253be06f3e0e6dd131776ac1d3c2c1e3f729ab6e882a0690eb2b', 'cases/type-generations-10.args': 'a9a4406f35bffd83dd5c206068eb26299221f2837338265d03dc1c13a284e479', 'cases/type-generations-11.args': 'effc7e511b82125e96f236b90d68c9a5e591014e012208a1a780001bbac8233d', 'cases/type-height-00.args': '1214e0d730e12c20477783a93fbe575ecfacb581a7b01333183be754f32f34b6', 'cases/type-height-01.args': 'b063e6a033956e541878bf88a4b46017d4c94a4e60ea560a370d5391cd77f3c1', 'cases/type-height-02.args': 'aa4d6c78eacce2952862e6133acbc47eb42644739083ed6a3f2a2d22d39e56eb', 'cases/type-height-03.args': 'ccd9937aebb35df52225fad5ea4fb4678b09854f4f4450aab0dbe8b6a40e35f5', 'cases/type-height-04.args': 'b8f5d2f63151dc9ef2fc4a4aadb12d49f8ed41629948cee9980465e2dfb0c1d6', 'cases/type-height-05.args': 'b5ffff716461ce9d270afad5ca057d9add287e2ed0598e874407a2d8f52bea3f', 'cases/type-height-06.args': 'f75223191c3cdfca35c55da0561c994cf899c70547f440f59360ecd5897259c7', 'cases/type-height-07.args': '84a977a253c5f3d3a0cd5a8efaceb082a76eb57723938f8ac7ee95e1d2b60610', 'cases/type-height-08.args': '6e5a66f1874f41fc0f8bbb91d5f817025d2f0feeceef0e1a8da0fd6d804d1501', 'cases/type-height-09.args': 'f926c0231e83e22b7c94db9267ee5ec8193a21952d7542a73e52dfc1be4e2b54', 'cases/type-height-10.args': '68ebc8a584365034988ce6ace4a9893e4cea72c297c02e6ea283c52d552fde76', 'cases/type-height-11.args': 'b6a4c9d5f3945993c48f8207ae1ddb6d9a1042debd29b2f2682ad33ff96717c3', 'cases/type-width-00.args': '7e01b783c3bf67570b255f706160936140f49106460993764fc575ffef21eb21', 'cases/type-width-01.args': '6570967dfb49c4e801a0aff723cb278222dae0ce53f10cccd45e2664d6d1de26', 'cases/type-width-02.args': '2829d6f527219235e2e510d9990ececcdb7680e294257fbef64827eadac1d32b', 'cases/type-width-03.args': '4fb3a0cb12de47981c029c30eac5a96d79208e6bbd213a0ee9f9f6b6c37333d0', 'cases/type-width-04.args': '1e83ddc90f84b13b01916667b9420f2990bae1e5ee778a7384b4095b3c5a04c1', 'cases/type-width-05.args': 'c822f652b7a6cf5b9ae829df9336b091f7997ff59883afee7ac1e253ad0ef11c', 'cases/type-width-06.args': '6d804696566ce65bede453e7cd9e1bd2100f9f35ce786892d888b532d93f8872', 'cases/type-width-07.args': '4e678a0e71c83cbb53fce9f2ffbb88872ed61e45e5c5d0babbec2b976f58d184', 'cases/type-width-08.args': 'b215c6b517800d9ede573fa8ca6b0cb9b24a07387e6ea23b22104fc9b68d08e9', 'cases/type-width-09.args': 'f9901c40134b083afc38a4c42cde63974548487016a290959c96dc034002f5a1', 'cases/type-width-10.args': 'cb53cabd647e3bce8bdde21bce956243d84ab81ce55f2e92b5c50e162feb8e99', 'cases/type-width-11.args': 'a0a0b53898464436137792deba1046104c7d302e30434de9b5f7dad683cae526', 'cases/unicode-ignored-key-high-high.args': '925eb9a0739d5703cd9f3a95024c9050dd20afea78b34a6b814ac96d0fdf6900', 'cases/unicode-ignored-key-high-text.args': 'd26312fcd9c8235034c5bcbad7dffda1a93e5ba736cbb318bec99ac358c0cec5', 'cases/unicode-ignored-key-high.args': 'a8190c45bbee55660948aac1574a3383e598c362bca859a1bf25b7193e3f3e71', 'cases/unicode-ignored-key-low-high.args': '7bb1102b49cf33cb80436ac96e93c54678de484d63f9ba2c4a506f4c0ad7aec0', 'cases/unicode-ignored-key-low.args': '39188bab484e7bdf79d50f871f9d491115eb4a3fd01ed85725d3e837a9ae1ff6', 'cases/unicode-ignored-value-high-high.args': 'b58239c77603e019884115febbc01296e95645696783f0087010a6beda401b33', 'cases/unicode-ignored-value-high-text.args': '0aa388af9c8fa06fd0984b7eaad438377c2c5a6de340b491d29f7e110b7efb99', 'cases/unicode-ignored-value-high.args': '6f4eac6edbdbf4b263ca54f2ab34e6e2c76b08534e89807be9be258be48bc029', 'cases/unicode-ignored-value-low-high.args': '70f6ec6be5626f38c50e908f32326e6c1ade968cdb08f94a5c737d47ca161efd', 'cases/unicode-ignored-value-low.args': 'd76d7f3c3764de4c9201ed59a8b992983af0d2295fe6b32a4e20d4d06714efb1', 'cases/unicode-priority-grid.args': 'c9f047965655d2219541fc5d4b5bafff8b9c141f141a2baf96ee8c3d42c05d02', 'cases/whitespace-json.args': '3514eb58fb45bb6613074c6e42905d92e783cacce763827d38cf48ded0e55ef1', 'cases/wrapped-edge-distinguishing.args': '5df102232b9bdc7ac7537b489f4c60b875aa2717446fade3b6a9ec9d99730caa', 'cases/zero-steps.args': '49a575ce82361f29ab8318e18e17161070c36fe19e38a88e62fb9e86cada90fc', 'intent.md': '7aed5ea39f1511d628ad6585390825b152f025fd36595c03d2821d646c6f386f', 'reference.py': 'a13da234e3053d2f24a47d1dfa4e76456cbab791d11b57f9454449f91cd5170e'}
FROZEN_LIBRARY={'examples/ail_std_life.ail/README.md': 'ec764c7df772afb861e05448c69ec3fd084203a5836d67984b4132185fd56564', 'examples/ail_std_life.ail/ail-package.md': '9803896c47d8c985894aace2e3444a561cc335054eea832f33b248c4c5554947', 'examples/ail_std_life.ail/spec.ail-spec.md': '53f8a7ffb5f806a1b81e85594a3a3a49a53ca5f5da0cf71b9d82c74cca5f9d17'}

def digest(data):
    return hashlib.sha256(data).hexdigest()

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

def checked(command):
    r = invoke(command)
    assert r.returncode == 0 and r.stderr == b'', (command, r.returncode, r.stderr[-1500:])
    return r.stdout

def build(root, package, stem, actions, full=True):
    frontends = [('source', [package])]
    if full:
        core = root / (stem + '.core')
        core.write_bytes(checked([AIL, 'ail-core', package]))
        rendered = root / (stem + '-rendered')
        rendered.mkdir()
        (rendered / 'ail-package.md').write_text(
            '# AIL Package\nname: life-rendered\nversion: 0.1.0\nprofile: Application\n'
            'entry: spec.ail-spec.md\nfeatures: things, actions, functions, text, bytes, json, lists, stdio, expressions, control-flow, traces\n'
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

def run(route, action, arguments=(), input=b'', stdio=False, timeout=60):
    _, vm, binaries, native = route
    command = [binaries[action]] if native else [*vm, '--action', action]
    return invoke(command + (['--stdio'] if stdio else []) + list(arguments), input, timeout)

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

def frozen():
    members={str(p.relative_to(RUNG)):digest(p.read_bytes()) for p in sorted(RUNG.rglob('*')) if p.is_file() and p.relative_to(RUNG).parts[0]!='program.ail'}
    assert members==FROZEN_ORACLE, 'oracle membership/bytes changed'
    assert len(members)==174
    hashed=hashlib.sha256()
    for name in members: hashed.update(name.encode()+b'\0'+(RUNG/name).read_bytes()+b'\0')
    assert 'sha256:'+hashed.hexdigest()[:32]==FINGERPRINT
    assert {str(p.relative_to(REPO)):digest(p.read_bytes()) for p in LIBRARY.rglob('*') if p.is_file()}==FROZEN_LIBRARY
    return members


def invoke(command,input=b'',timeout=60):
    assert digest(DRIVER.read_bytes())==DRIVER_SHA, 'active driver changed'
    command=list(map(str,command));index=len(RECORDS);start=time.monotonic()
    files={str(Path(s).resolve()):digest(Path(s).read_bytes()) for s in command if len(s)<4096 and Path(s).is_absolute() and Path(s).is_file()}
    for s in command:
        if len(s)<4096 and Path(s).is_absolute() and Path(s).is_dir():
            files.update({str(p.resolve()):digest(p.read_bytes()) for p in Path(s).rglob('*') if p.is_file()})
    record={'command':command,'timeout_seconds':timeout,'stdin_bytes':len(input),'stdin_sha256':digest(input),'driver':str(DRIVER),'driver_sha256':DRIVER_SHA,'command_files':files,'startup_identity_sha256':digest((ROOT/'identity-before.json').read_bytes())}
    RECORDS.append(record);(ROOT/'io'/f'{index:05d}.stdin').write_bytes(input)
    (ROOT/'commands.json').write_text(json.dumps(RECORDS,ensure_ascii=False,indent=2)+'\n')
    try: result=harness.invoke(command,input,cwd=REPO,timeout=timeout)
    except BaseException as error:
        record.update(status=None,exception=repr(error),seconds=time.monotonic()-start)
        for suffix,value in [('stdout',getattr(error,'output',b'') or b''),('stderr',getattr(error,'stderr',b'') or b'')]: (ROOT/'io'/f'{index:05d}.{suffix}').write_bytes(value)
        (ROOT/'commands.json').write_text(json.dumps(RECORDS,ensure_ascii=False,indent=2)+'\n');raise
    record.update(status=result.returncode,seconds=time.monotonic()-start,stdout_bytes=len(result.stdout),stderr_bytes=len(result.stderr),stdout_sha256=digest(result.stdout),stderr_sha256=digest(result.stderr))
    for suffix,value in [('stdout',result.stdout),('stderr',result.stderr)]: (ROOT/'io'/f'{index:05d}.{suffix}').write_bytes(value)
    assert digest(DRIVER.read_bytes())==DRIVER_SHA
    for name,sha in files.items(): assert digest(Path(name).read_bytes())==sha, name
    (ROOT/'commands.json').write_text(json.dumps(RECORDS,ensure_ascii=False,indent=2)+'\n')
    return result


def inspect(program):
    assert program['external_bindings']==[] and program['capability_grants']==[]
    helpers=set()
    for action in program['actions']:
        pure=action['action'].startswith('Life.')
        if pure:
            helpers.add(action['action']);assert action['instructions'][0]['opcode']=='VALUE_FUNCTION'
        for instruction in action['instructions']:
            op=instruction['opcode']
            assert not op.startswith(('PROCESS_','HOST_','FILE_','DIRECTORY_')) and op not in ('CALL_EXTERNAL','EVAL_EXPRESSION')
            if op.startswith(('STDIN_','STDOUT_','STDERR_')): assert action['action']=='Run' and op in ('STDIN_READ_TEXT','STDOUT_WRITE_TEXT')
            if pure:
                assert op not in ('ACTION_BEGIN','EMIT_TRACE','OUTPUT_FIELD','RETURN_SUCCESS')
                key=instruction['operands'].get('key');assert key is None or key.startswith('local.')
                if op=='CALL_VALUE':assert instruction['operands']['target'].startswith('Life.')
        if action['action']=='Run':
            assert [i['opcode'] for i in action['instructions']]==['ACTION_BEGIN','STDIN_READ_TEXT','CALL_VALUE','JSON_FORMAT','ASSIGN_LITERAL','TEXT_JOIN','STDOUT_WRITE_TEXT','EMIT_TRACE','RETURN_SUCCESS']
        if action['action']=='Analyze':assert [i['operands'] for i in action['instructions'] if i['opcode']=='OUTPUT_FIELD']==[{'key':'out.result'}]
    assert helpers=={'Life.'+n for n in ['population','validateGrid','neighbors','generation','step','evolveValidated','evolve','failure','runRequest']}
    COUNTS['ir_inspections']+=1


ACTIONS=['Step','Evolve','Population','Request','Composition','CatchStep','CatchEvolve','CatchPopulation']
PROBE_SOURCE='''The application LifeProbe manages Life test inputs.
An Input has:
- width: Int
- height: Int
- cells: List<Int>
- generations: Int
- text: Text
An Out has:
- cells: List<Int>
- population: Int
- document: Json
- first: List<Int>
- combined: List<Int>
- repeated: List<Int>
- error: Text
Action: Step.
When Step happens:
- the system calls Life.step with width = input.width; height = input.height; cells = input.cells into out.cells
- the system returns out.cells
- the system records a trace event named Checked
Action: Evolve.
When Evolve happens:
- the system calls Life.evolve with width = input.width; height = input.height; cells = input.cells; generations = input.generations into out.cells
- the system returns out.cells
- the system records a trace event named Checked
Action: Population.
When Population happens:
- the system calls Life.population with cells = input.cells into out.population
- the system returns out.population
- the system records a trace event named Checked
Action: Request.
When Request happens:
- the system calls Life.runRequest with text = input.text into out.document
- the system returns out.document
- the system records a trace event named Checked
Action: Composition.
When Composition happens:
- the system calls Life.step with width = input.width; height = input.height; cells = input.cells into out.first
- the system calls Life.evolve with width = input.width; height = input.height; cells = out.first; generations = input.generations into out.combined
- the system calls Life.evolve with width = input.width; height = input.height; cells = input.cells; generations = input.generations + 1 into out.repeated
- the system returns input.cells
- the system returns out.first
- the system returns out.combined
- the system returns out.repeated
- the system records a trace event named Checked
'''
for action,target,parameters in [('CatchStep','step','width = input.width; height = input.height; cells = input.cells'),('CatchEvolve','evolve','width = input.width; height = input.height; cells = input.cells; generations = input.generations'),('CatchPopulation','population','cells = input.cells')]:
    PROBE_SOURCE+=f'''Action: {action}.
When {action} happens:
- the system assigns "" to out.error
- try:
- the system calls Life.{target} with {parameters} into out.{'population' if target=='population' else 'cells'}
- recover VALUE into out.error:
- end try
- the system returns out.error
- the system records a trace event named Checked
'''

def probe_package(root):
    p=root/'direct-probe';p.mkdir()
    (p/'ail-package.md').write_text('# AIL Package\nname: life-direct-probe\nversion: 0.1.0\nprofile: Application\nentry: spec.ail-spec.md\nfeatures: things, actions, functions, text, bytes, json, lists, expressions, control-flow, traces\nimports: '+str(LIBRARY)+' compatible ^0.1 as Life\nconformance: first-slice\n')
    (p/'spec.ail-spec.md').write_text(PROBE_SOURCE);return p


def encoded(value):return ORACLE.encode(value).encode('utf-8')

def fixed_cases():
    cases=[]
    for p in sorted((RUNG/'cases').glob('*.args')):
        lines=[l.strip() for l in p.read_text().splitlines() if l.strip() and not l.lstrip().startswith('#')]
        assert len(lines)==1 and lines[0].startswith('data.request=')
        text=lines[0][len('data.request='):];cases.append((p.stem,text,encoded(ORACLE.evaluate_request(text))))
    assert len(cases)==172;return cases


def direct_vectors(cases):
    vectors=[]
    def add(label,action,args,fields,recovery=()):vectors.append((label,action,args,b''.join(k.encode()+b'='+(v.encode() if isinstance(v,str) else encoded(v))+b'\n' for k,v in sorted(fields)),list(recovery)))
    for label,text,wanted in cases:
        add('request-'+label,'Request',['input.text='+text],[('out.document',ORACLE.evaluate_request(text))])
        result=ORACLE.evaluate_request(text)
        if result['ok']:
            fields=dict(ORACLE._parse(text));w=ORACLE._selected_integer(fields['width']);h=ORACLE._selected_integer(fields['height']);g=ORACLE._selected_integer(fields['generations']);cells=tuple(ORACLE._selected_integer(x) for x in fields['cells'])
            args=[f'input.width={w}',f'input.height={h}','input.cells='+ORACLE.encode(cells),f'input.generations={g}']
            add('evolve-'+label,'Evolve',args,[('out.cells',ORACLE.evolve(w,h,cells,g))])
            add('step-'+label,'Step',args,[('out.cells',ORACLE.step(w,h,cells))])
            add('population-'+label,'Population',args,[('out.population',ORACLE.population(cells))])
    cells=(0,0,0,1,1,1,0,0,0);args=['input.width=3','input.height=3','input.cells='+ORACLE.encode(cells),'input.generations=4'];first=ORACLE.step(3,3,cells)
    add('composition-retained-independent','Composition',args,[('input.cells',cells),('out.first',first),('out.combined',ORACLE.evolve(3,3,first,4)),('out.repeated',ORACLE.evolve(3,3,cells,5))])
    add('population-empty','Population',['input.cells=[]'],[('out.population',0)])
    add('canonical-negative-zero','Evolve',['input.width=1','input.height=1','input.cells=[-0]','input.generations=-0'],[('out.cells',[0])])
    for w,h,c,g in [(0,1,[0],0),(1,-1,[0],0),(2,1,[0],0),(1,1,[2],0),(9223372036854775807,9223372036854775807,[0],0),(1,1,[0],-1)]:
        args=[f'input.width={w}',f'input.height={h}','input.cells='+ORACLE.encode(c),f'input.generations={g}']
        for action in ['CatchStep','CatchEvolve']:
            try:(ORACLE.step(w,h,c) if action=='CatchStep' else ORACLE.evolve(w,h,c,g));error=''
            except ORACLE.VALUE:error='AIL-RUNTIME-VALUE'
            add('domain-'+str(len(vectors)),action,args,[('out.error',error)],['VALUE'] if error else [])
    add('population-value','CatchPopulation',['input.cells=[1,2]'],[('out.error','AIL-RUNTIME-VALUE')],['VALUE'])
    # Literal whitespace survives actual argv and stdin, not .args line stripping.
    for suffix in [' ','\t','\r','\n',' \t\r\n']:
        text=' \t\r\n'+cases[0][1]+suffix
        add('literal-whitespace-'+repr(suffix),'Request',['input.text='+text],[('out.document',ORACLE.evaluate_request(text))])
    return vectors


def direct_check(result,route,vector,complete=False):
    label,action,args,lines,recoveries=vector
    assert result.returncode==0,(route[0],label,result.returncode,result.stderr[-1500:])
    actual=b''.join(l for l in result.stdout.splitlines(keepends=True) if l.startswith((b'out.',b'input.cells=')))
    assert actual==lines,(route[0],label,actual[:500],lines[:500])
    if complete:
        trace=[f'action {action} started']
        if action.startswith('Catch'): trace+=['assigned literal out.error']
        if not route[3]: trace += ['recovered '+c+' into '+('out.error' if action.startswith('Catch') else 'local.error') for c in recoveries]
        trace+=['trace Checked']
        stdout=lines if route[3] else b'ail-run succeeded\n'+lines+b'trace='+(' -> '.join(trace)).encode()+b'\n'
        stderr=('\n'.join(trace)+'\n').encode() if route[3] else b''
        assert result.stdout==stdout and result.stderr==stderr,(label,route[0],result.stdout[:800],stdout[:800],result.stderr,stderr)
        tag=str(len(RECORDS)-1);(ROOT/(tag+'.expected.stdout')).write_bytes(stdout);(ROOT/(tag+'.expected.stderr')).write_bytes(stderr)
        COUNTS['complete_direct_reports']+=1
    else:
        if route[3]: assert result.stdout==lines and result.stderr.endswith(b'trace Checked\n')
        else: assert result.stderr==b'' and result.stdout.endswith(b'trace Checked\n')
    COUNTS['direct_results']+=1


def selected(result,wanted,route,recoveries=(),complete=False):
    line=b'out.result=hex:'+wanted.hex().encode()+b'\n'
    assert result.returncode==0,(route[0],result.returncode,result.stderr[-1500:])
    assert [l for l in result.stdout.splitlines(keepends=True) if l.startswith(b'out.result=')]==[line],route[0]
    trace=['action Analyze started']+([] if route[3] else ['recovered '+c+' into local.error' for c in recoveries])+['formatted JSON data.report','computed bytes out.result','trace Simulated']
    if complete:
        stdout=line if route[3] else b'ail-run succeeded\n'+line+b'trace='+(' -> '.join(trace)).encode()+b'\n'
        stderr=('\n'.join(trace)+'\n').encode() if route[3] else b''
        assert result.stdout==stdout and result.stderr==stderr,(route[0],result.stdout[:800],stdout[:800],result.stderr,stderr)
        tag=str(len(RECORDS)-1);(ROOT/(tag+'.expected.stdout')).write_bytes(stdout);(ROOT/(tag+'.expected.stderr')).write_bytes(stderr)
        COUNTS['complete_app_reports']+=1
    else:
        if route[3]:assert result.stdout==line and result.stderr.endswith(b'trace Simulated\n')
        else:assert result.stderr==b'' and result.stdout.endswith(b'trace Simulated\n')
    COUNTS['exact_results']+=1


def exact(result,wanted,context):
    assert result.returncode==0 and result.stdout==wanted and result.stderr==b'',(context,result.returncode,result.stdout[:500],result.stderr[-1500:])
    COUNTS['exact_stdin']+=1


def fatal(result,category,context,private=(b'PRIVATE-LIFE-INPUT',)):
    assert result.returncode>0 and result.stdout==b'',(context,result.returncode,result.stdout[:500])
    token=b'AIL bytecode execution step limit exceeded' if category=='FUEL' else ('AIL-RUNTIME-'+category+':').encode()
    assert token in result.stderr and all(s not in result.stdout+result.stderr for s in private),(context,result.stderr[-1500:])
    COUNTS['fatal_checks']+=1


def stdin_cases(cases):
    values=[(label,text.encode(),wanted+b'\n') for label,text,wanted in cases]
    for suffix in [' ','\t','\r','\n',' \t\r\n']:
        text=' \t\r\n'+cases[0][1]+suffix;values.append(('literal-whitespace-'+repr(suffix),text.encode(),encoded(ORACLE.evaluate_request(text))+b'\n'))
    values.append(('sample',(PACKAGE/'input.json').read_bytes(),(PACKAGE/'expected.json').read_bytes()))
    assert values[-1][2]==encoded(ORACLE.evaluate_request(values[-1][1].decode()))+b'\n'
    return values


def limits(routes):
    small=b'{"width":1,"height":1,"cells":[-0],"generations":0}'
    padded=small+b' '*(LIMIT-len(small))
    private=b'PRIVATE-LIFE-INPUT'
    grid=b','.join([b'0']*1024)
    alloc=b'{"private":"'+private+b'","width":32,"height":32,"cells":['+grid+b'],"generations":70}'
    control=alloc.replace(b'":70}',b'":1}')
    fuel=b'{"private":"'+private+b'","width":1,"height":1,"cells":[0],"generations":10000}'
    fuel_control=fuel.replace(b'":10000}',b'":257}')
    depth=b'{"private":"'+private+b'","extra":'+b'['*64+b'0'+b']'*64+b',"width":1,"height":1,"cells":[0],"generations":0}'
    items=b'{"private":"'+private+b'","width":65537,"height":1,"cells":['+b','.join([b'0']*65537)+b'],"generations":0}'
    for route in routes:
        for name,text in [('exact-one-MiB',padded),('fuel-control257',fuel_control),('allocation-control32x32x1',control)]:exact(run(route,'Run',input=text,stdio=True),encoded(ORACLE.evaluate_request(text.decode()))+b'\n',(route[0],name))
        fatal(run(route,'Run',input=private),'CAPABILITY',(route[0],'missing-grants'))
        for name,text,category in [('invalid-UTF8',b'\xff'+private,'TEXT'),('one-MiB-plus-one',padded+b' ','TEXT-LIMIT'),('depth65',depth,'JSON-LIMIT'),('cells65537',items,'JSON-LIMIT'),('real-fuel',fuel,'FUEL'),('real-list-allocation',alloc,'LIST-LIMIT')]:
            fatal(run(route,'Run',input=text,stdio=True,timeout=300 if name=='real-list-allocation' else 60),category,(route[0],name))
        print('limits',route[0],flush=True)
    (ROOT/'limits.json').write_text(json.dumps({'input_bytes':LIMIT,'input_plus_one':'TEXT-LIMIT','cells65537':'JSON-LIMIT before typed-list extraction','depth65':'JSON-LIMIT','fuel':'1x1x10000','fuel_control':'1x1x257','list_allocation':'32x32x70','list_control':'32x32x1','canonical_bit_response_bound_bytes':131073,'one_MiB_canonical_bit_response':'unreachable; not claimed measured'},indent=2)+'\n')


def mutations(root):
    original=(LIBRARY/'spec.ail-spec.md').read_text()
    for kind in ['sequential','wrapped']:
        lib=root/(kind+'-library.ail');shutil.copytree(LIBRARY,lib)
        text=original
        if kind=='sequential':
            start=text.index('Function: generation.');end=text.index('Function: step.',start);chunk=text[start:end]
            chunk=chunk.replace('- result: List<Int>\n- x: Int','- result: List<Int>\n- work: List<Int>\n- x: Int')
            chunk=chunk.replace('- the function creates an empty list into local.result','- the function slices list local.cells from 0 to local.width * local.height into local.work\n- the function creates an empty list into local.result')
            chunk=chunk.replace('cells = local.cells; x = local.x','cells = local.work; x = local.x')
            chunk=chunk.replace('- the function appends integer local.bit to list local.result into local.result','- the function appends integer local.bit to list local.result into local.result\n- the function sets item local.y * local.width + local.x in list local.work to integer local.bit into local.work')
            text=text[:start]+chunk+text[end:]
            request='{"width":3,"height":3,"cells":[0,0,0,1,1,1,0,0,0],"generations":1}'
        else:
            text=text.replace('local.ny as local.y + local.dy','local.ny as (local.y + local.dy + local.height) % local.height').replace('local.nx as local.x + local.dx','local.nx as (local.x + local.dx + local.width) % local.width')
            request='{"width":3,"height":3,"cells":[1,1,0,1,0,0,0,0,0],"generations":1}'
        assert text!=original;(lib/'spec.ail-spec.md').write_text(text)
        p=root/(kind+'-app');shutil.copytree(PACKAGE,p);manifest=p/'ail-package.md';manifest.write_text(manifest.read_text().replace('../ail_std_life.ail',str(lib)))
        wanted=encoded(ORACLE.evaluate_request(request))
        for route in build(root,p,kind,['Analyze'],False):
            result=run(route,'Analyze',['data.request='+request]);assert result.returncode==0
            line=next(l for l in result.stdout.splitlines() if l.startswith(b'out.result=hex:'));actual=bytes.fromhex(line[len(b'out.result=hex:'):].decode())
            selected(result,actual,route,complete=True);assert json.loads(actual)['ok'] is True and actual!=wanted
            (root/(kind+'-'+('native' if route[3] else 'VM')+'-rejected.json')).write_text(json.dumps({'status':0,'expected_hex':wanted.hex(),'actual_hex':actual.hex(),'rejected':True},indent=2)+'\n');COUNTS['semantic_mutations_detected']+=1


def transport_limits(root,probe):
    # These explicit bytecode fixtures test direct typed transport via actual stdin.
    # They retain the unchanged pure Life routines and contain no host simulation.
    original=json.loads(checked([AIL,'ail-lower',probe]))
    def op(name,**values):return {'opcode':name,'operands':values}
    for action,target,arguments,key in [('RawStep','Life.step','cells=f:input.cells;height=f:input.height;width=f:input.width','out.cells'),('RawPopulation','Life.population','cells=f:input.cells','out.population')]:
        ir=json.loads(json.dumps(original))
        instructions=[op('ACTION_BEGIN',action=action),op('STDIN_READ_TEXT',key='input.cells')]
        if action=='RawStep':
            instructions += [op('ASSIGN_LITERAL',key='input.width',type='Int',value='1'),op('ASSIGN_LITERAL',key='input.height',type='Int',value='1')]
        instructions += [op('CALL_VALUE',arguments=arguments,key=key,target=target),op('STDOUT_WRITE_TEXT',source=key),op('RETURN_SUCCESS')]
        ir['actions']=[a for a in ir['actions'] if a['action'].startswith('Life.')]+[{'action':action,'instructions':instructions}]
        artifact=root/(action+'.ailbc.json');artifact.write_text(json.dumps(ir)+'\n');binary=root/(action+'.elf')
        checked([AIL,'ail-compile',artifact,'--action',action,'--target','linux-x86_64-elf','--out',binary])
        commands=[[AIL,'ail-vm',artifact,'--action',action,'--stdio'],[binary,'--stdio']]
        for command in commands:
            control=b'[-0]'+b' '*(LIMIT-4) if action=='RawStep' else b'[1,0,1]'
            exact(invoke(command,control),b'[0]' if action=='RawStep' else b'2',(action,'complete-stdin-transport-control'))
            fatal(invoke(command,control+b' ' if action=='RawStep' else b'['+b','.join([b'0']*65537)+b']'),'TEXT-LIMIT' if action=='RawStep' else 'LIST-LIMIT',(action,'transport-boundary'))
            fatal(invoke(command,b'["PRIVATE-LIFE-INPUT"]'),'LIST',(action,'typed-malformed'))
    (root/'typed-transport.json').write_text(json.dumps({'raw_List_control_bytes':LIMIT,'canonical_result':'[0]','stdin_plus_one':'TEXT-LIMIT at stream read','item_count65537':'LIST-LIMIT at direct typed argument boundary','malformed_list':'LIST; private-marker absent','fixture':'explicit bytecode transport fixture; unchanged Life API routines'},indent=2)+'\n')


def inherit_preflight(path,deps):
    path=path.resolve();identity=json.loads((path/'identity-before.json').read_text())
    prior=identity['dependencies']
    # Only the acceptance harness changed to correct its isolated mutant/reuse
    # machinery. All application/library/compiler/runtime/oracle semantics match.
    excluded={'scripts/ail_life_differential.py'}
    assert {k:v for k,v in prior.items() if k not in excluded}=={k:v for k,v in deps.items() if k not in excluded},'preflight semantic input identity changed'
    assert identity['oracle']==FROZEN_ORACLE and identity['compiler']==COMPILER_SHA256
    completed=(path/'summary.json').is_file()
    if completed:
        summary=json.loads((path/'summary.json').read_text());assert summary['status']=='PASS' and summary['counts']['semantic_mutations_detected']==4 and summary['counts']['fatal_checks']==8
        partial=Path(json.loads((path/'preflight.json').read_text())['inherited_from'])
        failure=json.loads((partial/'failure.json').read_text())
    else:failure=json.loads((path/'failure.json').read_text())
    assert failure['counts']['complete_app_reports']==8 and failure['counts']['complete_direct_reports']==22 and failure['counts']['exact_stdin']==14 and failure['counts']['fatal_checks']==14
    assert 'AIL-LITERAL-001' in failure['exception'] and 'sequential-app' in failure['exception'],'unsupported preflight checkpoint'
    assert Path(identity['driver']).read_bytes()==(path/'executed-driver.py').read_bytes()
    assert digest(Path(identity['driver']).read_bytes())==identity['driver_sha256']
    for p in path.glob('*.expected.stdout'):
        number=p.name.split('.')[0];assert p.read_bytes()==(path/'io'/f'{int(number):05d}.stdout').read_bytes()
        assert p.with_suffix('.stderr').read_bytes()==(path/'io'/f'{int(number):05d}.stderr').read_bytes()
    inherited={str(p.resolve()):digest(p.read_bytes()) for p in path.rglob('*') if p.is_file()}
    if completed:
        historical=json.loads((path/'reused-preflight.json').read_text())['files']
        assert all(digest(Path(p).read_bytes())==v for p,v in historical.items())
        inherited.update(historical)
    (ROOT/'reused-preflight.json').write_text(json.dumps({'path':str(path),'status':'successful complete-report/control/fatal tuples retained; final isolated mutant compile failed','counts':failure['counts'],'driver_sha256':identity['driver_sha256'],'files':inherited},indent=2)+'\n')
    return inherited,completed


def existing_routes(root,package,stem,actions):
    """Use exact already-built artifacts; do not reconstruct or rewrite them."""
    routes=[]
    frontends=[('source',[package]),('core',['--core-file',root/(stem+'.core')]),('rendered',[root/(stem+'-rendered')])]
    for name,arguments in frontends:
        forms=[('direct',[AIL,'ail-run',*arguments]),('ste',[AIL,'ail-vm',root/f'{stem}-{name}.ail-ste'])]
        if name=='source':forms.append(('bytecode',[AIL,'ail-vm',root/f'{stem}-{name}.ailbc.json']))
        for form,vm in forms:
            binaries={action:root/f'{stem}-{name}-{form}-{action}.elf' for action in actions}
            assert all(p.is_file() for p in binaries.values())
            routes += [(f'{stem}/{name}/{form}/VM',vm,binaries,False),(f'{stem}/{name}/{form}/native',[],binaries,True)]
    return routes


def inherit_matrix(path,cases,deps):
    path=path.resolve();identity=json.loads((path/'identity-before.json').read_text());failure=json.loads((path/'failure.json').read_text())
    excluded={'scripts/ail_life_differential.py'}
    assert {k:v for k,v in identity['dependencies'].items() if k not in excluded}=={k:v for k,v in deps.items() if k not in excluded},'matrix semantic dependencies changed'
    assert identity['oracle']==FROZEN_ORACLE and identity['compiler']==COMPILER_SHA256
    assert failure['exception']=="OSError(36, 'File name too long')" and failure['counts']=={'ir_inspections':6,'exact_results':36} and failure['commands']==122
    assert digest(Path(identity['driver']).read_bytes())==identity['driver_sha256'] and Path(identity['driver']).read_bytes()==(path/'executed-driver.py').read_bytes()
    records=json.loads((path/'commands.json').read_text());assert len(records)==122
    completed=[]
    for number,(label,text,wanted) in enumerate(cases[:36],len(records)-36):
        record=records[number];expected=[str(AIL),'ail-run',str(PACKAGE),'--action','Analyze','data.request='+text]
        assert record['command']==expected and record['status']==0 and record['timeout_seconds']==60
        stdout=(path/'io'/f'{number:05d}.stdout').read_bytes();stderr=(path/'io'/f'{number:05d}.stderr').read_bytes()
        assert digest(stdout)==record['stdout_sha256'] and digest(stderr)==record['stderr_sha256'] and stderr==b''
        line=b'out.result=hex:'+wanted.hex().encode()+b'\n'
        assert [l for l in stdout.splitlines(keepends=True) if l.startswith(b'out.result=')]==[line] and stdout.startswith(b'ail-run succeeded\n') and stdout.endswith(b'trace Simulated\n')
        completed.append({'route':'app/source/direct/VM','case':label,'command_index':number,'expected_hex':wanted.hex(),'stdout_sha256':digest(stdout),'stderr_sha256':digest(stderr)})
    files={str(p.resolve()):digest(p.read_bytes()) for p in path.rglob('*') if p.is_file()}
    (ROOT/'reused-matrix.json').write_text(json.dumps({'path':str(path),'driver_sha256':identity['driver_sha256'],'inherited_results':completed,'source_artifact_files':files,'reason':'absolute path discovery correction only; successful tuples and artifacts retained without replay'},indent=2)+'\n')
    COUNTS['inherited_exact_results']+=len(completed)
    routes=existing_routes(path,PACKAGE,'app',['Analyze','Run']);probes=existing_routes(path,path/'direct-probe','probe',ACTIONS)
    # Reinspect the six immutable ordinary lowered programs, without rebuilding.
    for stem in ['app','probe']:
        for name in ['source','core','rendered']:inspect(json.loads((path/f'{stem}-{name}.ailbc.json').read_text()))
    return routes,probes,len(completed),files


def main():
    global ROOT,ORACLE,DRIVER_SHA
    parser=argparse.ArgumentParser(description=__doc__);parser.add_argument('--artifact-dir',required=True,type=Path);parser.add_argument('--preflight-only',action='store_true');parser.add_argument('--preflight-evidence',type=Path);parser.add_argument('--matrix-evidence',type=Path);parser.add_argument('--worker',action='store_true',help=argparse.SUPPRESS);parser.add_argument('--reservation-fd',type=int,help=argparse.SUPPRESS);args=parser.parse_args()
    assert args.reservation_fd is not None,'worker requires supervisor reservation'
    ROOT=args.artifact_dir.resolve();reserved=os.fstat(args.reservation_fd);actual=ROOT.stat();assert (reserved.st_dev,reserved.st_ino)==(actual.st_dev,actual.st_ino)
    assert not any(ROOT.iterdir());(ROOT/'io').mkdir();assert ctypes.CDLL(None).prctl(36,1,0,0,0)==0
    DRIVER_SHA=digest(DRIVER.read_bytes());before=frozen();deps=dependencies();assert digest(AIL.read_bytes())==COMPILER_SHA256
    (ROOT/'identity-before.json').write_text(json.dumps({'driver':str(DRIVER),'driver_sha256':DRIVER_SHA,'oracle':before,'dependencies':deps,'compiler':COMPILER_SHA256},indent=2)+'\n')
    for name in deps:
        p=REPO/name;d=ROOT/'sources'/name;d.parent.mkdir(parents=True,exist_ok=True);d.write_bytes(p.read_bytes())
    (ROOT/'executed-driver.py').write_bytes(DRIVER.read_bytes())
    spec=importlib.util.spec_from_file_location('life_independent_oracle',RUNG/'reference.py');ORACLE=importlib.util.module_from_spec(spec);sys.modules[spec.name]=ORACLE;spec.loader.exec_module(ORACLE)
    start=time.monotonic()
    try:
        cases=fixed_cases();vectors=direct_vectors(cases);probe=probe_package(ROOT)
        inherited={}
        if args.preflight_evidence:
            inherited,completed=inherit_preflight(args.preflight_evidence,deps)
            if not completed:mutations(ROOT);transport_limits(ROOT,probe)
            (ROOT/'preflight.json').write_text(json.dumps({'counts':dict(COUNTS),'inherited_from':str(args.preflight_evidence),'inherited_counts':json.loads((args.preflight_evidence/('summary.json' if completed else 'failure.json')).read_text())['counts']},indent=2)+'\n')
        else:
            early=build(ROOT,PACKAGE,'smoke-app',['Analyze','Run'],False);early_probe=build(ROOT,probe,'smoke-probe',ACTIONS,False)
            smoke=[('{"width":1,"height":1,"cells":[1],"generations":1}',[]),('{}',['JSON-PATH']),('{"width":0,"height":1,"cells":[0],"generations":-1}',['VALUE']),('{"width":1,"height":1,"cells":[0],"generations":-1}',[])]
            for route in early:
                for text,recovery in smoke:
                    selected(run(route,'Analyze',['data.request='+text]),encoded(ORACLE.evaluate_request(text)),route,recovery,True)
                    exact(run(route,'Run',input=text.encode(),stdio=True),encoded(ORACLE.evaluate_request(text))+b'\n',(route[0],'smoke'))
            # All four APIs, composition/retained input, zero-step bad grid and VALUE recovery
            # receive exact full reports before the broad matrix starts.
            preflight=[]
            for action in ACTIONS:
                preflight.append(next(v for v in vectors if v[1]==action and (action!='Request' or v[0]=='request-'+cases[0][0])))
            preflight += [v for v in vectors if v[0] in ('population-empty','canonical-negative-zero')]
            # Deliberately choose the zero-generation invalid bit grid.
            preflight.append(next(v for v in vectors if v[1]=='CatchEvolve' and 'input.cells=[2]' in v[2]))
            for route in early_probe:
                for v in preflight:direct_check(run(route,v[1],v[2]),route,v,True)
            limits(early);mutations(ROOT);transport_limits(ROOT,probe)
            (ROOT/'preflight.json').write_text(json.dumps({'counts':dict(COUNTS),'commands':len(RECORDS),'direct_report_labels':[v[0] for v in preflight]},indent=2)+'\n')
        routes=[];probes=[];matrix_skip=0;matrix_files={}
        if not args.preflight_only:
            if args.matrix_evidence:routes,probes,matrix_skip,matrix_files=inherit_matrix(args.matrix_evidence,cases,deps)
            else:routes=build(ROOT,PACKAGE,'app',['Analyze','Run']);probes=build(ROOT,probe,'probe',ACTIONS)
            assert len(routes)==len(probes)==14
            assert ((args.matrix_evidence or ROOT)/'app-source.ail-ste').read_bytes()==(REPO/'examples/ste/conway-life.ail-ste').read_bytes(),'source STE must execute exact checked export bytes'
            (ROOT/'expectations.json').write_text(json.dumps({'requests':[(n,w.hex()) for n,_,w in cases],'direct':[(n,a,args,lines.hex(),recovery) for n,a,args,lines,recovery in vectors]},ensure_ascii=False,indent=2)+'\n')
            for route_number,route in enumerate(routes):
                for number,(label,text,wanted) in enumerate(cases):
                    if route_number==0 and number<matrix_skip:continue
                    selected(run(route,'Analyze',['data.request='+text]),wanted,route)
                for label,text,wanted in stdin_cases(cases):exact(run(route,'Run',input=text,stdio=True),wanted,(route[0],label))
                print('app matrix',route[0],len(cases),len(stdin_cases(cases)),flush=True)
            for route in probes:
                for v in vectors:direct_check(run(route,v[1],v[2]),route,v)
                print('direct matrix',route[0],len(vectors),flush=True)
        assert all(digest(Path(p).read_bytes())==v for p,v in matrix_files.items()),'prior matrix artifacts changed'
        assert all(digest(Path(p).read_bytes())==v for p,v in inherited.items()),'prior preflight evidence changed'
        assert frozen()==before and dependencies()==deps and digest(AIL.read_bytes())==COMPILER_SHA256 and digest(DRIVER.read_bytes())==DRIVER_SHA
        summary={'status':'PASS','counts':dict(COUNTS),'app_routes':len(routes),'direct_routes':len(probes),'direct_vectors':len(vectors),'request_cases':len(cases),'commands':len(RECORDS),'seconds':time.monotonic()-start,'fingerprint':FINGERPRINT,'driver_sha256':DRIVER_SHA}
        (ROOT/'identity-after.json').write_text(json.dumps({'driver_sha256':digest(DRIVER.read_bytes()),'dependencies':dependencies(),'oracle':frozen(),'compiler':digest(AIL.read_bytes())},indent=2)+'\n')
        (ROOT/'summary.json').write_text(json.dumps(summary,indent=2)+'\n');print(json.dumps(summary),flush=True)
    except BaseException as error:
        (ROOT/'failure.json').write_text(json.dumps({'exception':repr(error),'counts':dict(COUNTS),'commands':len(RECORDS)},indent=2)+'\n');raise
    finally:(ROOT/'commands.json').write_text(json.dumps(RECORDS,ensure_ascii=False,indent=2)+'\n')


if __name__=='__main__':
    if '--worker' in sys.argv or '--help' in sys.argv or '-h' in sys.argv: main()
    else:
        supervisor=argparse.ArgumentParser(description=__doc__);supervisor.add_argument('--artifact-dir',required=True,type=Path);supervisor.add_argument('--preflight-only',action='store_true');supervisor.add_argument('--preflight-evidence',type=Path);supervisor.add_argument('--matrix-evidence',type=Path);args=supervisor.parse_args()
        destination=args.artifact_dir.resolve();descriptor=reserve_destination(destination)
        try:
            assert ctypes.CDLL(None).prctl(36,1,0,0,0)==0
            sha=digest(DRIVER.read_bytes());command=[sys.executable,str(DRIVER),'--worker','--reservation-fd',str(descriptor),*sys.argv[1:]];start=time.monotonic()
            try:result=harness.invoke(command,cwd=REPO,timeout=7200,pass_fds=(descriptor,))
            except BaseException as error:
                write_supervisor_records(descriptor,getattr(error,'output',b'') or b'',getattr(error,'stderr',b'') or b'',{'command':command,'timeout_seconds':7200,'status':None,'exception':repr(error),'seconds':time.monotonic()-start,'driver_sha256':sha});raise
            assert digest(DRIVER.read_bytes())==sha
            write_supervisor_records(descriptor,result.stdout,result.stderr,{'command':command,'timeout_seconds':7200,'status':result.returncode,'seconds':time.monotonic()-start,'driver_sha256':sha,'stdout_sha256':digest(result.stdout),'stderr_sha256':digest(result.stderr)})
            sys.stdout.buffer.write(result.stdout);sys.stderr.buffer.write(result.stderr);sys.exit(result.returncode)
        finally:os.close(descriptor)
