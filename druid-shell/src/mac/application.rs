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

//! macOS implementation of features at the application scope.

#![allow(non_upper_case_globals)]

use super::util;
use crate::clipboard::{ClipboardItem, CustomData};
use cocoa::appkit::{NSApp, NSPasteboardTypeString};
use cocoa::base::{id, nil, BOOL, YES};
use cocoa::foundation::{NSInteger, NSUInteger};

pub struct Application;

const kCFPropertyListBinaryFormat_v1_0: NSUInteger = 200;

impl Application {
    pub fn quit() {
        unsafe {
            let () = msg_send![NSApp(), terminate: nil];
        }
    }

    /// Hide the application this window belongs to. (cmd+H)
    pub fn hide() {
        unsafe {
            let () = msg_send![NSApp(), hide: nil];
        }
    }

    /// Hide all other applications. (cmd+opt+H)
    pub fn hide_others() {
        unsafe {
            let workspace = class!(NSWorkspace);
            let shared: id = msg_send![workspace, sharedWorkspace];
            let () = msg_send![shared, hideOtherApplications];
        }
    }

    /// Returns the contents of the clipboard, if any.
    pub fn get_clipboard_contents() -> Option<ClipboardItem> {
        unsafe {
            let nspasteboard = class!(NSPasteboard);
            let pasteboard: id = msg_send![nspasteboard, generalPasteboard];
            let data_types: id = msg_send![pasteboard, types];
            let count: usize = msg_send![data_types, count];

            for i in 0..count {
                let dtype: id = msg_send![data_types, objectAtIndex: i];
                let is_string: BOOL = msg_send![dtype, isEqualToString: NSPasteboardTypeString];
                if is_string == YES {
                    let contents: id = msg_send![pasteboard, stringForType: dtype];
                    let contents = util::from_nsstring(contents);
                    return Some(contents.into());
                } else {
                    log::info!("unhandled pasteboard type {}", util::from_nsstring(dtype));
                }
                //TODO: handle other data types
            }
            None
        }
    }

    /// Sets the contents of the system clipboard.
    pub fn set_clipboard_contents(item: ClipboardItem) {
        use crate::clipboard::platform::DataType;
        unsafe {
            let nspasteboard = class!(NSPasteboard);
            let pasteboard: id = msg_send![nspasteboard, generalPasteboard];
            match item {
                ClipboardItem::Text(string) => {
                    let nsstring = util::make_nsstring(&string);
                    let _: NSInteger = msg_send![pasteboard, clearContents];
                    let _: BOOL =
                        msg_send![pasteboard, setString: nsstring forType: NSPasteboardTypeString];
                }
                ClipboardItem::Custom(CustomData { ref data, ref info })
                    if info.write_options().is_some() =>
                {
                    let _: NSInteger = msg_send![pasteboard, clearContents];
                    let write_options = info.write_options().unwrap();
                    let pb_type = write_options.identifier.to_nsstring();
                    let pb_data: id = match write_options.data_type {
                        crate::clipboard::platform::DataType::String => {
                            let string = String::from_utf8_lossy(&data);
                            util::make_nsstring(string.as_ref())
                        }
                        DataType::Data => util::make_nsdata(&data),
                        DataType::BinaryPlist => {
                            let data = util::make_nsdata(&data);
                            let format = kCFPropertyListBinaryFormat_v1_0;
                            let formatp = &format as *const NSUInteger;
                            //TODO: proper error handling. This means an error module for each
                            //platform, that represents the native platform error, and is converted
                            //to a top-level Error::Platform variant in druid-shell?
                            let plist_ser = class!(NSPropertyListSerialization);
                            msg_send![plist_ser, propertyListWithData: data options: 0 format: formatp error: nil]
                        }
                    };
                    if pb_data.is_null() {
                        log::warn!(
                            "failed to create pasteboard data of type '{}'",
                            write_options.identifier
                        );
                        return;
                    }

                    let _success: BOOL = match write_options.data_type {
                        DataType::String => {
                            msg_send![pasteboard, setString: pb_data forType: pb_type]
                        }
                        DataType::Data => msg_send![pasteboard, setData: pb_data forType: pb_type],
                        DataType::BinaryPlist => {
                            msg_send![pasteboard, setPropertyList: pb_data forType: pb_type]
                        }
                    };
                }
                other => log::warn!("unhandled clipboard data {:?}", other),
            }
        }
    }
}
