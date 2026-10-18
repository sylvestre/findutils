// Copyright 2017 Google Inc.
//
// Use of this source code is governed by a MIT-style
// license that can be found in the LICENSE file or at
// https://opensource.org/licenses/MIT.

pub mod find;
pub mod locate;
pub mod updatedb;
pub mod xargs;

/// Split a `PATH`-style list of paths.
///
/// `std::env::split_paths` panics on WASI, so split on `:` there.
pub(crate) fn split_paths(paths: &std::ffi::OsStr) -> Vec<std::path::PathBuf> {
    #[cfg(target_os = "wasi")]
    {
        use std::os::wasi::ffi::OsStrExt;
        paths
            .as_bytes()
            .split(|&b| b == b':')
            .map(|p| std::path::PathBuf::from(std::ffi::OsStr::from_bytes(p)))
            .collect()
    }
    #[cfg(not(target_os = "wasi"))]
    {
        std::env::split_paths(paths).collect()
    }
}
