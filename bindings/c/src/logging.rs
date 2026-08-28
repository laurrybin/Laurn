// Copyright 2026 laurrybin and Laurn Contributors
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

use log::{LevelFilter, Log, Metadata, Record};
use std::os::raw::c_char;
use std::ptr;
use std::sync::atomic::{AtomicPtr, Ordering};

pub type LaurnLogCallback = unsafe extern "C" fn(level: i32, msg: *const c_char);

static LOG_CALLBACK: AtomicPtr<()> = AtomicPtr::new(ptr::null_mut());

struct LaurnLogger;

impl Log for LaurnLogger {
    fn enabled(&self, _metadata: &Metadata) -> bool {
        true
    }

    fn log(&self, record: &Record) {
        let callback_ptr = LOG_CALLBACK.load(Ordering::Relaxed);
        if callback_ptr.is_null() {
            return;
        }

        let callback: LaurnLogCallback = unsafe { std::mem::transmute(callback_ptr) };
        let msg = format!("{}", record.args());
        if let Ok(c_msg) = std::ffi::CString::new(msg) {
            let level = match record.level() {
                log::Level::Error => 1,
                log::Level::Warn => 2,
                log::Level::Info => 3,
                log::Level::Debug => 4,
                log::Level::Trace => 5,
            };
            unsafe {
                callback(level, c_msg.as_ptr());
            }
        }
    }

    fn flush(&self) {}
}

static LOGGER: LaurnLogger = LaurnLogger;
static INIT: std::sync::Once = std::sync::Once::new();

/// Sets the global log callback.
#[no_mangle]
pub extern "C" fn laurn_set_log_callback(callback: Option<LaurnLogCallback>) -> crate::LaurnResult {
    crate::catch_unwind_ffi(|| {
        if let Some(cb) = callback {
            LOG_CALLBACK.store(cb as *mut (), Ordering::SeqCst);
            INIT.call_once(|| {
                let _ = log::set_logger(&LOGGER);
                log::set_max_level(LevelFilter::Trace);
            });
        } else {
            LOG_CALLBACK.store(ptr::null_mut(), Ordering::SeqCst);
        }
        crate::LaurnResult::Success
    })
}
