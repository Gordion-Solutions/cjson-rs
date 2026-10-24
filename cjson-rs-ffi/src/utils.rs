//! cJSON_Utils.h — JSON Pointer (RFC 6901), JSON Patch (RFC 6902) and
//! JSON Merge Patch (RFC 7386), as a C-ABI shim.
//!
//! Like upstream's `cJSON_Utils.c`, this layer is a pure *consumer* of the
//! public cJSON API: it creates, walks, copies and frees nodes only through
//! the `cJSON_*` functions this crate already exports. It is written from
//! the three RFCs and validated against cJSON's own utils tests, not by
//! translating `cJSON_Utils.c`.
//!
//! Every public function runs inside `panic_guard::guard(...)` and tolerates
//! NULL, matching cJSON's "return NULL / return non-zero on failure"
//! convention.

use crate::convert::alloc_cstr;
use crate::panic_guard::guard;
use crate::types::cJSON;
use crate::cJSON_bool;
use std::cmp::Ordering;
use std::ffi::{c_char, c_int, CStr};
use std::ptr;

// The building blocks, all exported from lib.rs.
use crate::{
    cJSON_AddItemToObject, cJSON_Compare, cJSON_CreateNull, cJSON_CreateObject,
    cJSON_Delete, cJSON_DeleteItemFromObject,
    cJSON_DeleteItemFromObjectCaseSensitive, cJSON_DetachItemFromObject,
    cJSON_DetachItemFromObjectCaseSensitive, cJSON_Duplicate, cJSON_GetArrayItem,
    cJSON_GetArraySize, cJSON_IsArray, cJSON_IsNull, cJSON_IsObject,
};

const TRUE: cJSON_bool = 1;

/// Compare two object-member keys. Case-insensitive folding is ASCII-only,
/// matching cJSON's own `case_insensitive_strcmp`.
fn compare_keys(a: &[u8], b: &[u8], case_sensitive: bool) -> Ordering {
    if case_sensitive {
        a.cmp(b)
    } else {
        let la = a.iter().map(|c| c.to_ascii_lowercase());
        let lb = b.iter().map(|c| c.to_ascii_lowercase());
        la.cmp(lb)
    }
}

/// The NUL-terminated key bytes of an object member (empty if unnamed).
unsafe fn key_bytes<'a>(node: *const cJSON) -> &'a [u8] {
    let s = (*node).string;
    if s.is_null() {
        &[]
    } else {
        CStr::from_ptr(s).to_bytes()
    }
}

// ---- JSON Pointer (RFC 6901) ------------------------------------------------

/// Decode one reference token's escapes: `~1` → `/`, then `~0` → `~`
/// (RFC 6901 §4, in that order). Returns an owned byte buffer.
fn decode_token(raw: &[u8]) -> Vec<u8> {
    let mut out = Vec::with_capacity(raw.len());
    let mut i = 0;
    while i < raw.len() {
        if raw[i] == b'~' && i + 1 < raw.len() {
            match raw[i + 1] {
                b'1' => {
                    out.push(b'/');
                    i += 2;
                    continue;
                }
                b'0' => {
                    out.push(b'~');
                    i += 2;
                    continue;
                }
                _ => {}
            }
        }
        out.push(raw[i]);
        i += 1;
    }
    out
}

/// Match an object member by decoded key. cJSON compares the raw bytes of
/// the key, case-sensitively or not; it does not require valid UTF-8, so we
/// compare bytes directly rather than going through `CStr`/`str`.
unsafe fn object_child_by_key(object: *const cJSON, key: &[u8], case_sensitive: bool) -> *mut cJSON {
    let mut child = (*object).child;
    while !child.is_null() {
        let name = (*child).string;
        if !name.is_null() {
            let name_bytes = CStr::from_ptr(name).to_bytes();
            let hit = if case_sensitive {
                name_bytes == key
            } else {
                name_bytes.eq_ignore_ascii_case(key)
            };
            if hit {
                return child;
            }
        }
        child = (*child).next;
    }
    ptr::null_mut()
}

/// Walk `object` by the JSON Pointer `pointer`.
///
/// An empty pointer selects `object` itself. Each step is `/` followed by a
/// reference token: a decoded member name for objects, a base-10 index for
/// arrays. Anything that does not resolve returns NULL.
unsafe fn get_pointer(object: *mut cJSON, pointer: *const c_char, case_sensitive: bool) -> *mut cJSON {
    if object.is_null() || pointer.is_null() {
        return ptr::null_mut();
    }
    let bytes = CStr::from_ptr(pointer).to_bytes();
    let mut cur = object;
    let mut i = 0;

    while i < bytes.len() {
        if bytes[i] != b'/' {
            // A non-empty pointer must be a sequence of '/'-prefixed tokens.
            return ptr::null_mut();
        }
        i += 1;
        let start = i;
        while i < bytes.len() && bytes[i] != b'/' {
            i += 1;
        }
        let raw = &bytes[start..i];

        if cur.is_null() {
            return ptr::null_mut();
        }

        if cJSON_IsArray(cur) == TRUE {
            // RFC 6901: array index is base-10 with no leading zeros
            // (except "0" itself). "-" (past-the-end) does not resolve.
            let idx = match parse_array_index(raw) {
                Some(n) => n,
                None => return ptr::null_mut(),
            };
            if idx >= cJSON_GetArraySize(cur) {
                return ptr::null_mut();
            }
            cur = cJSON_GetArrayItem(cur, idx);
        } else if cJSON_IsObject(cur) == TRUE {
            let key = decode_token(raw);
            cur = object_child_by_key(cur, &key, case_sensitive);
        } else {
            return ptr::null_mut();
        }
    }
    cur
}

/// Parse an RFC 6901 array index: digits only, no leading zeros unless the
/// token is exactly "0". Returns None for "-" and anything non-numeric.
fn parse_array_index(raw: &[u8]) -> Option<c_int> {
    if raw.is_empty() || !raw.iter().all(|b| b.is_ascii_digit()) {
        return None;
    }
    if raw.len() > 1 && raw[0] == b'0' {
        return None;
    }
    std::str::from_utf8(raw).ok()?.parse::<c_int>().ok()
}

// ---- Merge Patch (RFC 7386) -------------------------------------------------

/// RFC 7386 MergePatch, consuming `target` and returning the merged tree.
///
/// The non-object branch duplicates `patch` *before* freeing `target`, so a
/// `patch` that aliases a node inside `target` cannot read freed memory.
/// This is the fix for CVE-2026-87933 (upstream frees `target` first, then
/// duplicates the now-dangling `patch`).
unsafe fn merge_patch(mut target: *mut cJSON, patch: *const cJSON, case_sensitive: bool) -> *mut cJSON {
    if patch.is_null() || cJSON_IsObject(patch) != TRUE {
        // Scalar, array or NULL patch: the result is a copy of the patch.
        let replacement = cJSON_Duplicate(patch, TRUE);
        if !target.is_null() {
            cJSON_Delete(target);
        }
        return replacement;
    }

    if cJSON_IsObject(target) != TRUE {
        if !target.is_null() {
            cJSON_Delete(target);
        }
        target = cJSON_CreateObject();
        if target.is_null() {
            return ptr::null_mut();
        }
    }

    let mut patch_child = (*patch).child;
    while !patch_child.is_null() {
        let key = (*patch_child).string;
        if cJSON_IsNull(patch_child) == TRUE {
            // null means "remove the member", RFC 7386 §1.
            if case_sensitive {
                cJSON_DeleteItemFromObjectCaseSensitive(target, key);
            } else {
                cJSON_DeleteItemFromObject(target, key);
            }
        } else {
            let detached = if case_sensitive {
                cJSON_DetachItemFromObjectCaseSensitive(target, key)
            } else {
                cJSON_DetachItemFromObject(target, key)
            };
            let replacement = merge_patch(detached, patch_child, case_sensitive);
            if replacement.is_null() {
                cJSON_Delete(target);
                return ptr::null_mut();
            }
            cJSON_AddItemToObject(target, key, replacement);
        }
        patch_child = (*patch_child).next;
    }
    target
}

// ---- Find pointer (RFC 6901 encoding) ---------------------------------------

/// Encode a reference token: `~` → `~0`, `/` → `~1` (RFC 6901 §3).
fn encode_token(key: &[u8]) -> Vec<u8> {
    let mut out = Vec::with_capacity(key.len());
    for &b in key {
        match b {
            b'~' => out.extend_from_slice(b"~0"),
            b'/' => out.extend_from_slice(b"~1"),
            _ => out.push(b),
        }
    }
    out
}

/// Depth-first search for `target` under `haystack`, building the JSON
/// Pointer path to it. Returns the path bytes (empty when
/// `haystack == target`), or None if not found.
unsafe fn find_path(haystack: *const cJSON, target: *const cJSON) -> Option<Vec<u8>> {
    if haystack == target {
        return Some(Vec::new());
    }
    if haystack.is_null() {
        return None;
    }

    if cJSON_IsArray(haystack) == TRUE {
        let mut index: usize = 0;
        let mut child = (*haystack).child;
        while !child.is_null() {
            if let Some(sub) = find_path(child, target) {
                let mut path = format!("/{index}").into_bytes();
                path.extend_from_slice(&sub);
                return Some(path);
            }
            index += 1;
            child = (*child).next;
        }
    } else if cJSON_IsObject(haystack) == TRUE {
        let mut child = (*haystack).child;
        while !child.is_null() {
            if let Some(sub) = find_path(child, target) {
                let mut path = vec![b'/'];
                path.extend_from_slice(&encode_token(key_bytes(child)));
                path.extend_from_slice(&sub);
                return Some(path);
            }
            child = (*child).next;
        }
    }
    None
}

// ---- Sort object members ----------------------------------------------------

/// Reorder an object's direct members by key (stable), in place. Not
/// recursive — matches cJSON's `cJSONUtils_SortObject`.
unsafe fn sort_object(object: *mut cJSON, case_sensitive: bool) {
    if object.is_null() || cJSON_IsObject(object) != TRUE {
        return;
    }
    let mut children: Vec<*mut cJSON> = Vec::new();
    let mut child = (*object).child;
    while !child.is_null() {
        children.push(child);
        child = (*child).next;
    }
    if children.len() < 2 {
        return;
    }
    children.sort_by(|&a, &b| compare_keys(key_bytes(a), key_bytes(b), case_sensitive));

    // Relink the doubly-linked list in the new order.
    let n = children.len();
    for i in 0..n {
        let node = children[i];
        (*node).prev = if i == 0 { ptr::null_mut() } else { children[i - 1] };
        (*node).next = if i + 1 == n { ptr::null_mut() } else { children[i + 1] };
    }
    (*object).child = children[0];
}

// ---- Generate merge patch (RFC 7386, reverse) -------------------------------

/// Produce a merge patch that turns `from` into `to`: the patch `p` such that
/// `MergePatch(dup(from), p)` equals `to`. Returns NULL when `from` and `to`
/// are already equal (no changes needed).
unsafe fn generate_merge_patch(from: *const cJSON, to: *const cJSON, case_sensitive: bool) -> *mut cJSON {
    let both_objects = !from.is_null()
        && !to.is_null()
        && cJSON_IsObject(from) == TRUE
        && cJSON_IsObject(to) == TRUE;

    if !both_objects {
        if !from.is_null() && cJSON_Compare(from, to, case_sensitive as cJSON_bool) == TRUE {
            return ptr::null_mut();
        }
        return cJSON_Duplicate(to, TRUE);
    }

    let patch = cJSON_CreateObject();
    if patch.is_null() {
        return ptr::null_mut();
    }

    // Members present in `from` but not in `to`: emit null (remove).
    let mut fc = (*from).child;
    while !fc.is_null() {
        let key = key_bytes(fc);
        if object_child_by_key(to, key, case_sensitive).is_null() {
            cJSON_AddItemToObject(patch, (*fc).string, cJSON_CreateNull());
        }
        fc = (*fc).next;
    }

    // Members in `to`: add when new, recurse when changed, skip when equal.
    let mut tc = (*to).child;
    while !tc.is_null() {
        let key = key_bytes(tc);
        let from_child = object_child_by_key(from, key, case_sensitive);
        if from_child.is_null() {
            cJSON_AddItemToObject(patch, (*tc).string, cJSON_Duplicate(tc, TRUE));
        } else if cJSON_Compare(from_child, tc, case_sensitive as cJSON_bool) != TRUE {
            let sub = generate_merge_patch(from_child, tc, case_sensitive);
            if !sub.is_null() {
                cJSON_AddItemToObject(patch, (*tc).string, sub);
            }
        }
        tc = (*tc).next;
    }
    patch
}

// ---- Public C ABI -----------------------------------------------------------

#[no_mangle]
pub unsafe extern "C" fn cJSONUtils_GetPointer(object: *mut cJSON, pointer: *const c_char) -> *mut cJSON {
    guard(ptr::null_mut(), || get_pointer(object, pointer, false))
}

#[no_mangle]
pub unsafe extern "C" fn cJSONUtils_GetPointerCaseSensitive(
    object: *mut cJSON,
    pointer: *const c_char,
) -> *mut cJSON {
    guard(ptr::null_mut(), || get_pointer(object, pointer, true))
}

#[no_mangle]
pub unsafe extern "C" fn cJSONUtils_MergePatch(target: *mut cJSON, patch: *const cJSON) -> *mut cJSON {
    guard(ptr::null_mut(), || merge_patch(target, patch, false))
}

#[no_mangle]
pub unsafe extern "C" fn cJSONUtils_MergePatchCaseSensitive(
    target: *mut cJSON,
    patch: *const cJSON,
) -> *mut cJSON {
    guard(ptr::null_mut(), || merge_patch(target, patch, true))
}

#[no_mangle]
pub unsafe extern "C" fn cJSONUtils_FindPointerFromObjectTo(
    object: *const cJSON,
    target: *const cJSON,
) -> *mut c_char {
    guard(ptr::null_mut(), || {
        if object.is_null() || target.is_null() {
            return ptr::null_mut();
        }
        match find_path(object, target) {
            Some(path) => {
                // path is ASCII pointer syntax plus raw key bytes; build a
                // C string from the bytes directly.
                let s = String::from_utf8_lossy(&path);
                alloc_cstr(&s)
            }
            None => ptr::null_mut(),
        }
    })
}

#[no_mangle]
pub unsafe extern "C" fn cJSONUtils_SortObject(object: *mut cJSON) {
    guard((), || sort_object(object, false))
}

#[no_mangle]
pub unsafe extern "C" fn cJSONUtils_SortObjectCaseSensitive(object: *mut cJSON) {
    guard((), || sort_object(object, true))
}

#[no_mangle]
pub unsafe extern "C" fn cJSONUtils_GenerateMergePatch(from: *mut cJSON, to: *mut cJSON) -> *mut cJSON {
    guard(ptr::null_mut(), || generate_merge_patch(from, to, false))
}

#[no_mangle]
pub unsafe extern "C" fn cJSONUtils_GenerateMergePatchCaseSensitive(
    from: *mut cJSON,
    to: *mut cJSON,
) -> *mut cJSON {
    guard(ptr::null_mut(), || generate_merge_patch(from, to, true))
}
