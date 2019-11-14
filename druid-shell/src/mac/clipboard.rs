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

//! Interactions with the system pasteboard on macOS.

use cocoa::appkit::NSPasteboardTypeString;
use cocoa::base::{id, nil, BOOL, YES};
use cocoa::foundation::NSArray;

use super::util;
use crate::clipboard::{ClipboardFormat, ClipboardItem, ClipboardRead};

#[derive(Debug, Clone, Default)]
pub struct ClipboardContents;

/// A trait that represents the contents of the system clipboard.
impl ClipboardContents {
    /// Return the contents of the clipboard as a string, if possible.
    pub fn string_value(&self) -> Option<String> {
        unsafe {
            let pasteboard: id = msg_send![class!(NSPasteboard), generalPasteboard];
            let contents: id = msg_send![pasteboard, stringForType: NSPasteboardTypeString];
            if contents.is_null() {
                None
            } else {
                Some(util::from_nsstring(contents))
            }
        }
    }

    /// Attempts to retrieve the type of data described by the the provided
    /// [`ClipboardRead`].
    ///
    /// [`ClipboardRead`]: trait.ClipboardRead.html
    //NOTE: semantically, this should probably be returning a Result<T> or an
    // Option<Result<T>>, because parsing can fail. It isn't clear that anything
    // is really possible in that scenario, though, and the API is worse.
    pub fn custom_value<T: ClipboardRead>(&self, reader: &T) -> Option<T::Data> {
        let opts = reader.read_options()?;
        let pb_type = opts.identifier.to_nsstring();
        unsafe {
            let pasteboard: id = msg_send![class!(NSPasteboard), generalPasteboard];
            let avail_types: id = msg_send![pasteboard, types];
            let has_got_it: BOOL = msg_send![avail_types, containsObject: pb_type];
            if has_got_it == YES {
                let data: id = msg_send![pasteboard, dataForType: pb_type];
                if !data.is_null() {
                    let data = util::from_nsdata(data);
                    return reader.parse(data);
                } else {
                    log::warn!("pasteboard returned nil for ident {}", opts.identifier);
                }
            } else {
                log::info!("nothing in pasteboard for {}", opts.identifier);
            }
        }
        None
    }
}

/// Corresponds to [`NSPasteboardType`][].
///
/// In general, you should use a [Universal Type Identifier][] as your
/// pasteboard type; if you do not, a dynamic UTI will be generated for you
/// by Cocoa (if writing) or you will be ignored (if reading).
///
/// [`NSPasteboardType`]: https://developer.apple.com/documentation/appkit/nspasteboardtype
/// [Universal Type Identifier]: https://developer.apple.com/library/archive/documentation/FileManagement/Conceptual/understanding_utis/understand_utis_intro/understand_utis_intro.html
#[derive(Debug)]
pub enum Identifier {
    Uti(&'static str),
    //TODO: support non UTI identifiers, using core carbon functions like
    //UTTypeCreatePreferredIdentifierForTag and UTTypeCopyPreferredTagWithClass.
    #[doc(hidden)]
    __NonExhaustive,
}

/// Platform-specific options returned by [`ClipboardRead::read_options`]
///
/// [`ClipboardRead::read_options`]: trait.ClipboardRead.html#tymethod.read_options
pub struct ReadOpts {
    pub identifier: Identifier,
}

/// Platform-specific options returned by [`ClipboardWrite::write_options`]
///
/// [`ClipboardWrite::write_options`]: trait.ClipboardWrite.html#tymethod.write_options
#[derive(Debug)]
pub struct WriteOpts {
    pub identifier: Identifier,
    pub data_type: DataType,
}

/// The different raw types we can write to the clipboard on macOS.
///
/// If you wish to write a plist, you must encode it as a binary plist.
/// This limitation is imposed by druid for the sake of simplicity.
#[derive(Debug)]
pub enum DataType {
    String,
    Data,
    BinaryPlist,
}

impl Identifier {
    pub fn uti(id: &'static str) -> Self {
        Identifier::Uti(id)
    }

    pub(crate) fn to_nsstring(&self) -> id {
        if let Identifier::Uti(id) = self {
            return util::make_nsstring(id);
        }
        unreachable!("no other variants used");
    }
}

impl std::fmt::Display for Identifier {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            Identifier::Uti(s) => write!(f, "{}", s),
            __NonExhaustive => write!(f, "wat"), // very rare identifier
        }
    }
}

// platform-specific impls
impl ClipboardItem {
    pub(crate) fn make_types_array(&self) -> Option<id> {
        unsafe {
            let formats: Vec<_> = self
                .iter_supported()
                .map(|fmt| match fmt {
                    ClipboardFormat::Text(_) => NSPasteboardTypeString,
                    ClipboardFormat::Custom { info, .. } => {
                        info.write_options().unwrap().identifier.to_nsstring()
                    }
                    _ => unreachable!(),
                })
                .collect();

            if formats.is_empty() {
                None
            } else {
                Some(NSArray::arrayWithObjects(nil, &formats))
            }
        }
    }
}
