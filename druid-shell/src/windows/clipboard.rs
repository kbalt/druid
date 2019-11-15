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

use std::ffi::CString;
use std::mem;
use std::ptr;

use winapi::shared::minwindef::{FALSE, UINT};
use winapi::shared::ntdef::{CHAR, LPWSTR, WCHAR};
use winapi::shared::winerror::ERROR_SUCCESS;
use winapi::um::errhandlingapi::GetLastError;
use winapi::um::winbase::{GlobalAlloc, GlobalLock, GlobalSize, GlobalUnlock, GMEM_MOVEABLE};
use winapi::um::winuser::{
    CloseClipboard, EmptyClipboard, EnumClipboardFormats, GetClipboardData,
    GetClipboardFormatNameA, IsClipboardFormatAvailable, OpenClipboard, RegisterClipboardFormatA,
    SetClipboardData, CF_UNICODETEXT,
};

use crate::clipboard::{ClipboardFormat, ClipboardItem, ClipboardRead};
use crate::util::{FromWide, ToWide};

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
    pub fn custom_value<T: ClipboardRead>(&self, reader: &T) -> Option<T::Data> {
        let opts = reader.read_options()?;
        let format = register_identifier(opts.identifier)?;
        unsafe {
            if OpenClipboard(ptr::null_mut()) == FALSE {
                return None;
            }

            if IsClipboardFormatAvailable(format) != 0 {
                let handle = GetClipboardData(format);
                let size = GlobalSize(handle);
                let locked = GlobalLock(handle) as *const u8;
                let mut dest = Vec::<u8>::with_capacity(size);
                ptr::copy_nonoverlapping(locked, dest.as_mut_ptr(), size);
                dest.set_len(size);
                GlobalUnlock(handle);
                CloseClipboard();
                return reader.parse(dest);
            }

            for format in iter_clipboard_types() {
                eprintln!(
                    "have clipboard format {}: '{}'",
                    format,
                    get_format_name(format)
                );
            }
            CloseClipboard();
        }
        None
    }
}

/// Platform-specific options returned by [`ClipboardRead::read_options`]
///
/// [`ClipboardRead::read_options`]: trait.ClipboardRead.html#tymethod.read_options
#[derive(Debug)]
pub struct ReadOpts {
    pub identifier: &'static str,
}

/// Platform-specific options returned by [`ClipboardWrite::write_options`]
///
/// [`ClipboardWrite::write_options`]: trait.ClipboardWrite.html#tymethod.write_options
#[derive(Debug)]
pub struct WriteOpts {
    pub identifier: &'static str,
}

pub fn set_clipboard_contents(new_contents: ClipboardItem) {
    unsafe {
        if OpenClipboard(ptr::null_mut()) == FALSE {
            return;
        }
        EmptyClipboard();

        for fmt in new_contents.iter_supported() {
            match fmt {
                ClipboardFormat::Text(string) => {
                    let wstr = string.to_wide();
                    let wstr_copy =
                        GlobalAlloc(GMEM_MOVEABLE, wstr.len() * mem::size_of::<WCHAR>());
                    let locked = GlobalLock(wstr_copy) as LPWSTR;
                    ptr::copy_nonoverlapping(wstr.as_ptr(), locked, wstr.len());
                    GlobalUnlock(wstr_copy);
                    let result = SetClipboardData(CF_UNICODETEXT, wstr_copy);
                    if result.is_null() {
                        log::warn!("failed to set clipboard {}", GetLastError());
                    }
                }
                ClipboardFormat::Custom { data, info } => {
                    let opts = info.write_options().unwrap();
                    let pb_format = match register_identifier(opts.identifier) {
                        Some(success) => success,
                        None => continue,
                    };
                    let data_handle =
                        GlobalAlloc(GMEM_MOVEABLE, data.len() * mem::size_of::<CHAR>());
                    let locked = GlobalLock(data_handle) as *mut u8;
                    ptr::copy_nonoverlapping(data.as_ptr(), locked, data.len());
                    GlobalUnlock(data_handle);
                    let result = SetClipboardData(pb_format, data_handle);
                    if result.is_null() {
                        log::warn!("failed to set clipboard {}", GetLastError());
                    }
                }
                other => log::warn!("unhandled clipboard data {:?}", other),
            }
        }
        CloseClipboard();
    }
}

/// old impl, will be deleted soon
pub(crate) fn get_clipboard_contents() -> Option<ClipboardItem> {
    unsafe {
        if OpenClipboard(ptr::null_mut()) == FALSE {
            return None;
        }

        let result = get_clipboard_impl();
        CloseClipboard();
        result
    }
}

fn register_identifier(ident: &str) -> Option<UINT> {
    let cstr = match CString::new(ident) {
        Ok(s) => s,
        Err(_) => {
            // granted this should happen _never_, but unwrap feels bad
            log::warn!("Null byte in clipboard identifier '{}'", ident);
            return None;
        }
    };
    unsafe {
        let pb_format = RegisterClipboardFormatA(cstr.as_ptr());
        if pb_format == 0 {
            let err = GetLastError();
            log::warn!(
                "failed to register clipboard format '{}'; error {}.",
                ident,
                err
            );
            return None;
        }
        Some(pb_format)
    }
}

#[allow(clippy::single_match)] // we will support more types 'soon'
unsafe fn get_clipboard_impl() -> Option<ClipboardItem> {
    for format in iter_clipboard_types() {
        match format {
            CF_UNICODETEXT => return get_unicode_text(),
            other => eprintln!("other clipboard format '{}'", other),
        }
    }
    None
}

unsafe fn get_unicode_text() -> Option<ClipboardItem> {
    let handle = GetClipboardData(CF_UNICODETEXT);
    let result = if handle.is_null() {
        let unic_str = GlobalLock(handle) as LPWSTR;
        let result = unic_str.from_wide();
        GlobalUnlock(handle);
        result
    } else {
        None
    };
    result.map(Into::into)
}

pub(crate) fn iter_clipboard_types() -> impl Iterator<Item = UINT> {
    struct ClipboardTypeIter {
        last: UINT,
        done: bool,
    }

    impl Iterator for ClipboardTypeIter {
        type Item = UINT;

        fn next(&mut self) -> Option<Self::Item> {
            if self.done {
                return None;
            }
            unsafe {
                let nxt = EnumClipboardFormats(self.last);
                match nxt {
                    0 => {
                        self.done = true;
                        match GetLastError() {
                            ERROR_SUCCESS => (),
                            other => {
                                log::error!("iterating clipboard formats failed, error={}", other)
                            }
                        }
                        None
                    }
                    nxt => {
                        self.last = nxt;
                        Some(nxt)
                    }
                }
            }
        }
    }
    ClipboardTypeIter {
        last: 0,
        done: false,
    }
}

fn get_format_name(format: UINT) -> String {
    if let Some(name) = get_standard_format_name(format) {
        return name;
    }

    const BUF_SIZE: usize = 64;
    unsafe {
        let mut buffer: [CHAR; BUF_SIZE] = [0; BUF_SIZE];
        let result = GetClipboardFormatNameA(format, buffer.as_mut_ptr(), BUF_SIZE as i32);
        if result > 0 {
            let len = result as usize;
            let lpstr = std::slice::from_raw_parts(buffer.as_ptr() as *const u8, len);
            let name = String::from_utf8_lossy(&lpstr).into_owned();
            name
        } else {
            let err = GetLastError();
            if err == 87 {
                String::from("Unknown Format")
            } else {
                log::warn!(
                    "error getting clipboard format name for format {}, errno {}",
                    format,
                    err
                );
                String::new()
            }
        }
    }
}

// https://docs.microsoft.com/en-ca/windows/win32/dataxchg/standard-clipboard-formats
fn get_standard_format_name(format: UINT) -> Option<String> {
    let name = match format {
        1 => "CF_TEXT",
        2 => "CF_BITMAP",
        3 => "CF_METAFILEPICT",
        4 => "CF_SYLK",
        5 => "CF_DIF",
        6 => "CF_TIFF",
        7 => "CF_OEMTEXT",
        8 => "CF_DIB",
        9 => "CF_PALETTE",
        10 => "CF_PENDATA",
        11 => "CF_RIFF",
        12 => "CF_WAVE",
        13 => "CF_UNICODETEXT",
        14 => "CF_ENHMETAFILE",
        15 => "CF_HDROP",
        16 => "CF_LOCALE",
        17 => "CF_DIBV5",
        0x0080 => "CF_OWNERDISPLAY",
        0x0081 => "CF_DSPTEXT",
        0x0082 => "CF_DSPBITMAP",
        0x0083 => "CF_DSPMETAFILEPICT",
        0x008E => "CF_DSPENHMETAFILE",
        0x0200 => "CF_PRIVATEFIRST",
        0x02FF => "CF_PRIVATELAST",
        0x0300 => "CF_GDIOBJFIRST",
        0x03FF => "CF_GDIOBJLAST",
        _ => return None,
    };
    Some(name.into())
}
