//! Utility functions related to FFI functionality.  The C Shim re-exports several functions from libtickgrinder under a C API
//! so that it is usable by external applications such as the NodeJS FFI.

use std::ffi::CString;
use std::slice;
use libc::{c_int, c_char, c_void, memchr};
use std::mem;
use uuid::Uuid;

pub use transport::commands::CLogLevel;

/// Takes a pointer to a string from C and copies it into a Rust-owned `CString`.
pub unsafe fn ptr_to_cstring(ptr: *mut c_char) -> CString {
    // expect that no strings are longer than 100000 bytes
    let end_ptr = memchr(ptr as *const c_void, 0, 100000);
    let len: usize = end_ptr as usize - ptr as usize;
    let slice: &[u8] = slice::from_raw_parts(ptr as *const u8, len);
    CString::new(slice).expect("Unable to convert the slice into a CString")
}

use transport::command_server::CommandServer;
use transport::commands::HistTickDst;
use transport::data::transfer_data as rust_transfer_data;

const FLATFILE: c_int = 0; // { filename: String }
const POSTGRES: c_int = 1;// { table: String },
const REDIS_CHANNEL: c_int = 2; // { host: String, channel: String },
const REDIS_SET: c_int = 3; // { host: String, set_name: String },
const CONSOLE: c_int = 4;
const CSTRING_CONV_ERR: &'static str = "Unable to convert `CString` to `str`";

/// Wrapper around the internal `transfer_data` function that moves stored historical tick data from one location to
/// another, converting it as necessary.  The returned boolean is true if there was no error and false if there was an
/// error, which will be logged to the supplied `CommandServer`
#[no_mangle]
pub unsafe extern "C" fn c_transfer_data(
    src: c_int, dst: c_int, src_arg1: *mut c_void, src_arg2: *mut c_void,
    dst_arg1: *mut c_void, dst_arg2: *mut c_void, cs_ptr: *mut c_void
) -> bool {
    let cs: &mut CommandServer = &mut *(cs_ptr as *mut CommandServer);

    // try to build a `HistTickDst` from the given src arguments
    let src_htd: HistTickDst = match src {
        FLATFILE | POSTGRES | REDIS_CHANNEL | REDIS_SET => {
            build_htd(src, src_arg1, src_arg2)
        },
        CONSOLE => { // can't use the console as a data source, so error out and return
            cs.error(None as Option<&str>, "Unable to use the console as a source of historical ticks!");
            return false;
        },
        _ => { // invalid src identifier provided
            cs.error(None as Option<&str>, &format!("Unknown historical tick src provided: {}", src));
            return false;
        }
    };

    // build a `HistTickDst` for the dst arguments and transfer the data
    let dst_htd = build_htd(dst, dst_arg1, dst_arg2);
    rust_transfer_data(src_htd, dst_htd, cs.clone());

    // I'm not 100% sure if the `CommandServer` would be dropped here, so I'm `forget`ting it explicitly
    mem::forget(cs);

    true
}

/// Given the raw parts from the FFI, attempts to build a `HistTickDst` from them.
unsafe fn build_htd(id: c_int, arg1: *mut c_void, arg2: *mut c_void) -> HistTickDst {
    match id {
        FLATFILE => {
            let filename_cstring = ptr_to_cstring(arg1 as *mut c_char);
            let filename_string = String::from(filename_cstring.to_str().expect(CSTRING_CONV_ERR));
            HistTickDst::Flatfile{filename: filename_string}
        },
        POSTGRES => {
            let table_cstring = ptr_to_cstring(arg1 as *mut c_char);
            let table_string = String::from(table_cstring.to_str().expect(CSTRING_CONV_ERR));
            HistTickDst::Postgres{table: table_string}
        },
        REDIS_CHANNEL => {
            let host_cstring = ptr_to_cstring(arg1 as *mut c_char);
            let host_string = String::from(host_cstring.to_str().expect(CSTRING_CONV_ERR));
            let channel_cstring = ptr_to_cstring(arg2 as *mut c_char);
            let channel_string = String::from(channel_cstring.to_str().expect(CSTRING_CONV_ERR));

            HistTickDst::RedisChannel{
                host: host_string,
                channel: channel_string,
            }
        },
        REDIS_SET => {
            let host_cstring = ptr_to_cstring(arg1 as *mut c_char);
            let host_string = String::from(host_cstring.to_str().expect(CSTRING_CONV_ERR));
            let set_cstring = ptr_to_cstring(arg2 as *mut c_char);
            let set_string = String::from(set_cstring.to_str().expect(CSTRING_CONV_ERR));

            HistTickDst::RedisSet{
                host: host_string,
                set_name: set_string,
            }
        },
        CONSOLE => HistTickDst::Console,
        _ => panic!("Invalid ID given for `HistTickDst` conversion function"),
    }
}

/// Creates a CommandServer on the heap and returns it as a mutable reference.
#[no_mangle]
pub unsafe extern "C" fn get_command_server(name: *mut c_char) -> *mut c_void {
    let name_cstring = ptr_to_cstring(name);
    let name_str = name_cstring.to_str().expect("Unable to convert `CString` into `str`");
    let cs_box = Box::new(CommandServer::new(Uuid::new_v4(), name_str));
    Box::into_raw(cs_box) as *mut c_void
}

/*** C wrappers around functions for running commands on `CommandServer` pointers.  For all functions, passing a
**** null pointer as a category is the same as `None`.
***/

/// Maps an `Option<*mut c_char>` into an `Option<&str>`
unsafe fn map_ptr_opt(ptr_opt: Option<*mut c_char>) -> Option<String> {
    ptr_opt.map(|ptr| {
        let cat_cstring = ptr_to_cstring(ptr);
        String::from(cat_cstring.to_str().expect(CSTRING_CONV_ERR))
    })
}

#[no_mangle]
pub unsafe extern "C" fn c_cs_debug(cs_ptr: *mut c_void, category: *mut c_char, msg: *mut c_char) {
    let cs: &mut CommandServer = &mut *(cs_ptr as *mut CommandServer);
    let cat_cstring = ptr_to_cstring(category);
    let cat_str = cat_cstring.to_str().expect(CSTRING_CONV_ERR);
    let msg_cstring = ptr_to_cstring(msg);
    let msg_str = msg_cstring.to_str().expect(CSTRING_CONV_ERR);
    cs.debug(Some(cat_str), msg_str);
    mem::forget(cs);
}

#[no_mangle]
pub unsafe extern "C" fn c_cs_notice(cs_ptr: *mut c_void, category: *mut c_char, msg: *mut c_char) {
    let cs: &mut CommandServer = &mut *(cs_ptr as *mut CommandServer);
    let cat_cstring = ptr_to_cstring(category);
    let cat_str = cat_cstring.to_str().expect(CSTRING_CONV_ERR);
    let msg_cstring = ptr_to_cstring(msg);
    let msg_str = msg_cstring.to_str().expect(CSTRING_CONV_ERR);
    cs.notice(Some(cat_str), msg_str);
    mem::forget(cs);
}

#[no_mangle]
pub unsafe extern "C" fn c_cs_warning(cs_ptr: *mut c_void, category: *mut c_char, msg: *mut c_char) {
    let cs: &mut CommandServer = &mut *(cs_ptr as *mut CommandServer);
    let cat_cstring = ptr_to_cstring(category);
    let cat_str = cat_cstring.to_str().expect(CSTRING_CONV_ERR);
    let msg_cstring = ptr_to_cstring(msg);
    let msg_str = msg_cstring.to_str().expect(CSTRING_CONV_ERR);
    cs.warning(Some(cat_str), msg_str);
    mem::forget(cs);
}

#[no_mangle]
pub unsafe extern "C" fn c_cs_error(cs_ptr: *mut c_void, category: *mut c_char, msg: *mut c_char) {
    let cs: &mut CommandServer = &mut *(cs_ptr as *mut CommandServer);
    let cat_cstring = ptr_to_cstring(category);
    let cat_str = cat_cstring.to_str().expect(CSTRING_CONV_ERR);
    let msg_cstring = ptr_to_cstring(msg);
    let msg_str = msg_cstring.to_str().expect(CSTRING_CONV_ERR);
    cs.error(Some(cat_str), msg_str);
    mem::forget(cs);
}

#[no_mangle]
pub unsafe extern "C" fn c_cs_critical(cs_ptr: *mut c_void, category: *mut c_char, msg: *mut c_char) {
    let cs: &mut CommandServer = &mut *(cs_ptr as *mut CommandServer);
    let cat_cstring = ptr_to_cstring(category);
    let cat_str = cat_cstring.to_str().expect(CSTRING_CONV_ERR);
    let msg_cstring = ptr_to_cstring(msg);
    let msg_str = msg_cstring.to_str().expect(CSTRING_CONV_ERR);
    cs.critical(Some(cat_str), msg_str);
    mem::forget(cs);
}
