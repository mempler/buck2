# Copyright (c) Meta Platforms, Inc. and affiliates.
#
# This source code is licensed under both the MIT license found in the
# LICENSE-MIT file in the root directory of this source tree and the Apache
# License, Version 2.0 found in the LICENSE-APACHE file in the root directory
# of this source tree.

def _lbl(*args):
    _ = args
    return ""

def _set(_values, overwrite = False):
    _ = overwrite

ci_package = struct(
    set = _set,
    mac = _lbl,
    windows = _lbl,
    skip_test = _lbl,
    mode = _lbl,
    aarch64 = _lbl,
)
