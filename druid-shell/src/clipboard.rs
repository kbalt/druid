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

pub use crate::platform::clipboard::{self as platform, ClipboardContents};

/// An item to be put on the clipboard.
///
/// An item may have multiple representations; each representation
/// can be provided as a separate `ClipboardFormat`.
#[derive(Debug)]
pub struct ClipboardItem(Vec<ClipboardFormat>);

/// A representation of some data that can be put on the system clipboard.
#[derive(Debug)]
pub enum ClipboardFormat {
    Text(String),
    Custom {
        data: Vec<u8>,
        info: Box<dyn ClipboardWrite>,
    },
    #[doc(hidden)]
    /// Adding future items will not be a breaking change.
    __NotExhaustive,
    // other things?
}

impl ClipboardItem {
    /// Create a new `ClipboardFormat`.
    pub fn new(item: impl Into<ClipboardFormat>) -> Self {
        ClipboardItem(vec![item.into()])
    }

    /// A builder method to add a representation to this clipboard.
    pub fn add_format(mut self, item: impl Into<ClipboardFormat>) -> Self {
        self.0.push(item.into());
        self
    }

    /// An iterator over formats supported on the current platform.
    pub fn iter_supported(&self) -> impl Iterator<Item = &ClipboardFormat> {
        self.0.iter().filter(|fmt| fmt.supports_current_platform())
    }
}

impl ClipboardFormat {
    /// Create a new `ClipboardFormat`.
    pub fn new(item: impl Into<ClipboardFormat>) -> Self {
        item.into()
    }

    pub fn supports_current_platform(&self) -> bool {
        match self {
            ClipboardFormat::Text(_) => true,
            ClipboardFormat::Custom { info, .. } => info.write_options().is_some(),
            ClipboardFormat::__NotExhaustive => false,
        }
    }
}

//TODO: make custom formats work on windows, gtk.
// https://docs.microsoft.com/en-us/windows/win32/dataxchg/clipboard-formats#registered-clipboard-formats

/// A trait for types that can be written to the clipboard.
pub trait ClipboardWrite {
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

impl std::fmt::Debug for dyn ClipboardWrite {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "ClipboardWrite(\"{:?}\")", self.write_options())
    }
}

/// A trait for types that can be read from the clipboard.
pub trait ClipboardRead {
    /// The final (parsed) type of the data we will read.
    type Data;

    /// On each platform where this type can be read, returns platform-specific
    /// arguments for retrieving the raw data from the clipboard on that platform.
    fn read_options(&self) -> Option<platform::ReadOpts> {
        None
    }

    /// Attempts to parse raw data from the clipboard into `Self::Type`.
    //NOTE: this should probably be returning a Result<T>?
    fn parse(&self, data: Vec<u8>) -> Option<Self::Data>;
}

// an example:
#[derive(Debug, Clone)]
pub struct GlyphsBinaryPlist;

impl ClipboardWrite for GlyphsBinaryPlist {
    #[cfg(target_os = "macos")]
    fn write_options(&self) -> Option<platform::WriteOpts> {
        Some(platform::WriteOpts {
            identifier: platform::Identifier::uti("Glyphs elements pasteboard type"),
            data_type: platform::DataType::BinaryPlist,
        })
    }
}

pub struct Pdf;

impl ClipboardWrite for Pdf {
    #[cfg(target_os = "macos")]
    fn write_options(&self) -> Option<platform::WriteOpts> {
        Some(platform::WriteOpts {
            identifier: platform::Identifier::uti("com.adobe.pdf"),
            data_type: platform::DataType::Data,
        })
    }
}

impl ClipboardRead for Pdf {
    type Data = Vec<u8>;

    #[cfg(target_os = "macos")]
    fn read_options(&self) -> Option<platform::ReadOpts> {
        Some(platform::ReadOpts {
            identifier: platform::Identifier::uti("com.adobe.pdf"),
        })
    }

    fn parse(&self, data: Vec<u8>) -> Option<Self::Data> {
        Some(data)
    }
}

impl From<String> for ClipboardFormat {
    fn from(src: String) -> ClipboardFormat {
        ClipboardFormat::Text(src)
    }
}

impl From<String> for ClipboardItem {
    fn from(src: String) -> ClipboardItem {
        ClipboardItem(vec![ClipboardFormat::Text(src)])
    }
}

impl From<&str> for ClipboardFormat {
    fn from(src: &str) -> ClipboardFormat {
        ClipboardFormat::Text(src.to_string())
    }
}

impl From<&str> for ClipboardItem {
    fn from(src: &str) -> ClipboardItem {
        ClipboardItem(vec![ClipboardFormat::Text(src.to_string())])
    }
}

impl<T: ClipboardWrite + 'static> From<(Vec<u8>, T)> for ClipboardFormat {
    fn from(src: (Vec<u8>, T)) -> ClipboardFormat {
        ClipboardFormat::Custom {
            data: src.0,
            info: Box::new(src.1),
        }
    }
}

impl From<Vec<ClipboardFormat>> for ClipboardItem {
    fn from(src: Vec<ClipboardFormat>) -> ClipboardItem {
        ClipboardItem(src)
    }
}
