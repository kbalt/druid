// Copyright 2019 The xi-editor Authors.
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

//! Interactions with the system pasteboard on Windows.

use crate::clipboard::ClipboardRead;

#[derive(Debug, Clone, Default)]
pub struct ClipboardContents;

/// A trait that represents the contents of the system clipboard.
impl ClipboardContents {
    /// Return the contents of the clipboard as a string, if possible.
    pub fn string_value(&self) -> Option<String> {
        None
    }

    /// Attempts to retrieve the type of data described by the the provided
    /// [`ClipboardRead`].
    ///
    /// [`ClipboardRead`]: trait.ClipboardRead.html
    //NOTE: semantically, this should probably be returning a Result<T> or an
    // Option<Result<T>>, because parsing can fail. It isn't clear that anything
    // is really possible in that scenario, though, and the API is worse.
    fn custom_value<T: ClipboardRead>(&self, reader: &T) -> Option<T::Data> {
        None
    }
}

/// Platform-specific options returned by [`ClipboardRead::read_options`]
///
/// [`ClipboardRead::read_options`]: trait.ClipboardRead.html#tymethod.read_options
#[derive(Debug)]
pub struct ReadOpts;

/// Platform-specific options returned by [`ClipboardWrite::write_options`]
///
/// [`ClipboardWrite::write_options`]: trait.ClipboardWrite.html#tymethod.write_options
#[derive(Debug)]
pub struct WriteOpts;
