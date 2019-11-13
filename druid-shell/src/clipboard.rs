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

//! Interacting with the system pasteboard/clipboard.

use std::fmt::Debug;
use std::sync::Arc;

/// An item on the system clipboard.
#[derive(Debug, Clone)]
pub enum ClipboardItem {
    Text(String),
    Custom(Arc<dyn ClipboardWrite>),

    #[doc(hidden)]
    __NotExhaustive,
    // other things
}

impl ClipboardItem {
    /// Create a new `ClipboardItem`.
    pub fn new(item: impl Into<ClipboardItem>) -> Self {
        item.into()
    }
}

impl<T: ClipboardWrite + 'static> From<T> for ClipboardItem {
    fn from(src: T) -> ClipboardItem {
        ClipboardItem::Custom(Arc::new(src))
    }
}

impl From<String> for ClipboardItem {
    fn from(src: String) -> ClipboardItem {
        ClipboardItem::Text(src)
    }
}

impl From<&str> for ClipboardItem {
    fn from(src: &str) -> ClipboardItem {
        ClipboardItem::Text(src.to_string())
    }
}

//TODO: make custom formats work on windows, gtk.
// https://docs.microsoft.com/en-us/windows/win32/dataxchg/clipboard-formats#registered-clipboard-formats

/// A trait for types that can be written to the clipboard.
pub trait ClipboardWrite: Debug {
    /// The data to be written. How this data is interpreted will depend
    /// on the `WriteOpts` provided for a given platform.
    fn data(&self) -> &[u8];
    /// Returns, for a given platform, additional information for writing
    /// this data type on that platform. If `None`, this data will not
    /// be written on this platform.
    ///
    /// This method should only be implemented behind a `#[cfg()]` guard for
    /// a given backend. It might be implemented multiple times for different
    /// backends.
    fn write_options(&self) -> Option<platform::WriteOpts> {
        None
    }
}

#[derive(Debug, Clone)]
pub struct GlyphsBinaryPlist(pub Vec<u8>);

impl ClipboardWrite for GlyphsBinaryPlist {
    fn data(&self) -> &[u8] {
        self.0.as_slice()
    }

    #[cfg(target_os = "macos")]
    fn write_options(&self) -> Option<platform::WriteOpts> {
        Some(platform::WriteOpts {
            identifier: "Glyphs elements pasteboard type",
            data_type: platform::DataType::BinaryPlist,
        })
    }
}

/// Platform-specific clipboard types.
pub mod platform {
    #[cfg(all(target_os = "macos", not(feature = "use_gtk")))]
    pub use mac::*;

    /// Placeholder; platforms with special behaviour should have their
    /// own version.
    #[cfg(any(feature = "use_gtk", not(target_os = "macos")))]
    struct WriteOpts;

    pub mod mac {
        pub struct WriteOpts {
            /// Corresponds to [`NSPasteboardType`][].
            ///
            /// In general, you should use a [Universal Type Identifier][] as your
            /// pasteboard type; if you do not, a dynamic UTI will be generated for you
            /// by Cocoa.
            ///
            /// [`NSPasteboardType`]: https://developer.apple.com/documentation/appkit/nspasteboardtype
            /// [Universal Type Identifier]: https://developer.apple.com/library/archive/documentation/FileManagement/Conceptual/understanding_utis/understand_utis_intro/understand_utis_intro.html
            pub identifier: &'static str,
            pub data_type: DataType,
        }

        /// The different raw types we can write to the clipboard on macOS.
        ///
        /// If you wish to write a plist, you must encode it as a binary plist.
        /// This limitation is imposed by druid for the sake of simplicity.
        pub enum DataType {
            String,
            Data,
            BinaryPlist,
        }
    }
}
