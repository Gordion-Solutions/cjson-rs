//! cJSON_Utils FFI tests: JSON Pointer (RFC 6901) and Merge Patch (RFC 7386).
//!
//! These exercise the exports through the same `unsafe` calls a C caller
//! makes. The merge-patch alias test is the regression test for
//! CVE-2026-87933 and is meant to be run under Miri, which reports the
//! use-after-free that upstream cJSON 1.7.19 has on this input.

#![allow(non_snake_case)]

use cjson::*;
use std::ffi::{CStr, CString};
use std::ptr;

fn cstr(s: &str) -> CString {
    CString::new(s).unwrap()
}

unsafe fn parse(s: &str) -> *mut cjson::types::cJSON {
    let c = cstr(s);
    let p = cJSON_Parse(c.as_ptr());
    assert!(!p.is_null(), "parse failed: {s}");
    p
}

unsafe fn unformatted(item: *const cjson::types::cJSON) -> String {
    let p = cJSON_PrintUnformatted(item);
    assert!(!p.is_null());
    let s = CStr::from_ptr(p).to_str().unwrap().to_owned();
    cJSON_free(p as *mut _);
    s
}

// ---- JSON Pointer (RFC 6901) ----

#[test]
fn pointer_empty_selects_whole_document() {
    unsafe {
        let doc = parse(r#"{"a":1}"#);
        let got = cJSONUtils_GetPointer(doc, cstr("").as_ptr());
        assert_eq!(got, doc);
        cJSON_Delete(doc);
    }
}

#[test]
fn pointer_object_and_array_steps() {
    unsafe {
        let doc = parse(r#"{"a":{"b":[10,20,30]}}"#);
        let got = cJSONUtils_GetPointer(doc, cstr("/a/b/2").as_ptr());
        assert!(!got.is_null());
        assert_eq!(unformatted(got), "30");
        cJSON_Delete(doc);
    }
}

#[test]
fn pointer_escapes_tilde_and_slash() {
    unsafe {
        // RFC 6901 §4: "~1" decodes to "/", "~0" to "~".
        let doc = parse(r#"{"a/b":1,"c~d":2}"#);
        assert_eq!(
            unformatted(cJSONUtils_GetPointer(doc, cstr("/a~1b").as_ptr())),
            "1"
        );
        assert_eq!(
            unformatted(cJSONUtils_GetPointer(doc, cstr("/c~0d").as_ptr())),
            "2"
        );
        cJSON_Delete(doc);
    }
}

#[test]
fn pointer_missing_and_out_of_range_return_null() {
    unsafe {
        let doc = parse(r#"{"a":[1,2]}"#);
        assert!(cJSONUtils_GetPointer(doc, cstr("/a/2").as_ptr()).is_null());
        assert!(cJSONUtils_GetPointer(doc, cstr("/a/-").as_ptr()).is_null());
        assert!(cJSONUtils_GetPointer(doc, cstr("/missing").as_ptr()).is_null());
        assert!(cJSONUtils_GetPointer(doc, cstr("no-leading-slash").as_ptr()).is_null());
        cJSON_Delete(doc);
    }
}

#[test]
fn pointer_case_sensitive_vs_insensitive() {
    unsafe {
        let doc = parse(r#"{"Key":1}"#);
        assert!(cJSONUtils_GetPointerCaseSensitive(doc, cstr("/key").as_ptr()).is_null());
        assert!(!cJSONUtils_GetPointer(doc, cstr("/key").as_ptr()).is_null());
        cJSON_Delete(doc);
    }
}

// ---- Merge Patch (RFC 7386) ----

#[test]
fn merge_patch_sets_updates_and_removes() {
    unsafe {
        // RFC 7386 §3 example-ish: null removes, objects merge recursively.
        let target = parse(r#"{"a":"b","c":{"d":"e","f":"g"}}"#);
        let patch = parse(r#"{"a":"z","c":{"f":null}}"#);
        let result = cJSONUtils_MergePatch(target, patch);
        assert!(!result.is_null());
        assert_eq!(unformatted(result), r#"{"a":"z","c":{"d":"e"}}"#);
        cJSON_Delete(result);
        cJSON_Delete(patch);
    }
}

#[test]
fn merge_patch_non_object_replaces_whole_target() {
    unsafe {
        let target = parse(r#"{"a":"b"}"#);
        let patch = parse(r#"[1,2,3]"#);
        let result = cJSONUtils_MergePatch(target, patch);
        assert_eq!(unformatted(result), "[1,2,3]");
        cJSON_Delete(result);
        cJSON_Delete(patch);
    }
}

/// Regression test for CVE-2026-87933.
///
/// The patch argument aliases a node held inside the target. When that node
/// is not an object, merge-patch must return a copy of it without reading
/// freed memory. Upstream cJSON 1.7.19 frees the target first and then reads
/// the dangling patch; run under Miri this test reports that use-after-free
/// on upstream and passes clean here.
#[test]
fn merge_patch_patch_aliases_target_is_safe() {
    unsafe {
        let target = parse(r#"{"a":[1,2,3,4,5,6,7,8,9,10]}"#);
        let patch = cJSONUtils_GetPointer(target, cstr("/a").as_ptr());
        assert!(!patch.is_null());

        let result = cJSONUtils_MergePatch(target, patch);
        assert!(!result.is_null());
        assert_eq!(unformatted(result), "[1,2,3,4,5,6,7,8,9,10]");
        cJSON_Delete(result);
    }
}

#[test]
fn merge_patch_null_patch_deletes_target() {
    unsafe {
        let target = parse(r#"{"a":1}"#);
        // A JSON null patch replaces the whole target with null.
        let null_patch = parse("null");
        let result = cJSONUtils_MergePatch(target, null_patch);
        assert_eq!(cjson::cJSON_IsNull(result), 1);
        cJSON_Delete(result);
        cJSON_Delete(null_patch);
        let _ = ptr::null::<u8>();
    }
}
