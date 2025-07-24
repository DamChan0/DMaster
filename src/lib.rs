use std::ffi::{CStr, CString};
use std::os::raw::c_char;
mod apply;
mod display_info;
mod profile;
mod query;
use crate::apply::apply_profile;
use crate::profile::{load_profile, save_profile};
use crate::query::get_display_profile;

#[unsafe(no_mangle)]
pub extern "C" fn get_display_profile_json() -> *mut c_char {
    let profile = get_display_profile();
    let json = serde_json::to_string_pretty(&profile).unwrap();
    CString::new(json).unwrap().into_raw()
}

#[unsafe(no_mangle)]
pub extern "C" fn apply_display_profile_json(json_ptr: *const c_char) -> i32 {
    if json_ptr.is_null() {
        return -1;
    }
    let c_str = unsafe { CStr::from_ptr(json_ptr) };
    match serde_json::from_str::<crate::display_info::DisplayProfile>(c_str.to_str().unwrap()) {
        Ok(profile) => {
            apply_profile(&profile);
            0
        }
        Err(_) => -2,
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn save_display_profile_json(
    json_ptr: *const c_char,
    path_ptr: *const c_char,
) -> i32 {
    if json_ptr.is_null() || path_ptr.is_null() {
        return -1;
    }
    let c_json = unsafe { CStr::from_ptr(json_ptr) };
    let c_path = unsafe { CStr::from_ptr(path_ptr) };
    let profile: crate::display_info::DisplayProfile =
        match serde_json::from_str(c_json.to_str().unwrap()) {
            Ok(p) => p,
            Err(_) => return -2,
        };
    let path = c_path.to_str().unwrap();
    save_profile(&profile, path);
    0
}

#[unsafe(no_mangle)]
pub extern "C" fn load_display_profile_json(path_ptr: *const c_char) -> *mut c_char {
    if path_ptr.is_null() {
        return std::ptr::null_mut();
    }
    let c_path = unsafe { CStr::from_ptr(path_ptr) };
    let path = c_path.to_str().unwrap();
    let profile = load_profile(path);
    let json = serde_json::to_string_pretty(&profile).unwrap();
    CString::new(json).unwrap().into_raw()
}

#[unsafe(no_mangle)]
pub extern "C" fn free_rust_string(ptr: *mut c_char) {
    if !ptr.is_null() {
        unsafe {
            CString::from_raw(ptr);
        }
    }
}
