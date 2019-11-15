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

//! Windows implementation of features at the application scope.

use crate::clipboard::ClipboardItem;

pub struct Application;

impl Application {
    pub fn quit() {
        crate::runloop::request_quit();
    }

    /// Returns the contents of the clipboard, if any.
    #[deprecated(since = "0.4.0", note = "use methods on ClipboardContents instead")]
    pub fn get_clipboard_contents() -> Option<ClipboardItem> {
        super::clipboard::get_clipboard_contents()
    }

    pub fn set_clipboard_contents(item: ClipboardItem) {
        super::clipboard::set_clipboard_contents(item)
    }
}
