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

use crate::alloc_hooks;
use crate::convert::alloc_cstr;
use crate::panic_guard::guard;
use crate::types::{cJSON, CJSON_ARRAY, CJSON_OBJECT};
use crate::cJSON_bool;
use std::cmp::Ordering;
use std::ffi::{c_char, c_int, c_void, CStr, CString};
use std::ptr;

// The building blocks, all exported from lib.rs.
use crate::{
    cJSON_AddItemToArray, cJSON_AddItemToObject, cJSON_Compare, cJSON_CreateArray,
    cJSON_CreateNull, cJSON_CreateObject, cJSON_CreateString, cJSON_Delete, cJSON_DeleteItemFromArray,
    cJSON_DeleteItemFromObject, cJSON_DeleteItemFromObjectCaseSensitive,
    cJSON_DetachItemFromArray, cJSON_DetachItemFromObject,
    cJSON_DetachItemFromObjectCaseSensitive, cJSON_Duplicate, cJSON_GetArrayItem,
    cJSON_GetArraySize, cJSON_InsertItemInArray, cJSON_IsArray, cJSON_IsNull,
    cJSON_IsObject, cJSON_IsString,
    cJSON_ReplaceItemInObject, cJSON_ReplaceItemInObjectCaseSensitive,
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
    get_pointer_bytes(object, bytes, case_sensitive)
}

/// As `get_pointer`, but over a byte slice (no trailing NUL needed). Used by
/// the patch engine, which slices paths apart.
unsafe fn get_pointer_bytes(object: *mut cJSON, bytes: &[u8], case_sensitive: bool) -> *mut cJSON {
    if object.is_null() {
        return ptr::null_mut();
    }
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

// ---- JSON Patch (RFC 6902) --------------------------------------------------

/// Replace the *contents* of `dst` with those of `src`, in place, keeping
/// `dst`'s own node identity and list linkage. Used for operations whose
/// path is "" (the whole document). `src` is consumed.
unsafe fn overwrite_in_place(dst: *mut cJSON, src: *mut cJSON) {
    let old_child = (*dst).child;
    let old_valuestring = (*dst).valuestring;

    (*dst).type_ = (*src).type_;
    (*dst).valuestring = (*src).valuestring;
    (*dst).valueint = (*src).valueint;
    (*dst).valuedouble = (*src).valuedouble;
    (*dst).child = (*src).child;

    if !old_child.is_null() {
        cJSON_Delete(old_child);
    }
    if !old_valuestring.is_null() {
        alloc_hooks::hook_free(old_valuestring as *mut c_void);
    }

    // Free only the `src` shell (and its key), not the guts we moved out.
    (*src).child = ptr::null_mut();
    (*src).valuestring = ptr::null_mut();
    cJSON_Delete(src);
}

/// Split a non-empty pointer path into (parent-path, decoded-last-token).
/// Returns None if the path does not start with '/'.
fn split_last(path: &[u8]) -> Option<(&[u8], Vec<u8>)> {
    if path.is_empty() || path[0] != b'/' {
        return None;
    }
    let last = path.iter().rposition(|&b| b == b'/').unwrap();
    Some((&path[..last], decode_token(&path[last + 1..])))
}

/// Byte slice of a string-typed object member, if present.
unsafe fn member_string<'a>(object: *const cJSON, key: &[u8], case_sensitive: bool) -> Option<&'a [u8]> {
    let m = object_child_by_key(object, key, case_sensitive);
    if m.is_null() || cJSON_IsString(m) != TRUE || (*m).valuestring.is_null() {
        return None;
    }
    Some(CStr::from_ptr((*m).valuestring).to_bytes())
}

/// Place an owned `item` at `path` within `root` (the shared add/replace-at
/// logic for add/move/copy). On any failure, `item` is deleted and false is
/// returned. `replace_existing` chooses add-vs-replace semantics for objects
/// and arrays.
unsafe fn place_item(root: *mut cJSON, path: &[u8], item: *mut cJSON, case_sensitive: bool) -> bool {
    if path.is_empty() {
        overwrite_in_place(root, item);
        return true;
    }
    let (parent_path, token) = match split_last(path) {
        Some(v) => v,
        None => {
            cJSON_Delete(item);
            return false;
        }
    };
    let parent = get_pointer_bytes(root, parent_path, case_sensitive);
    if parent.is_null() {
        cJSON_Delete(item);
        return false;
    }

    if cJSON_IsArray(parent) == TRUE {
        let size = cJSON_GetArraySize(parent);
        if token == b"-" {
            cJSON_AddItemToArray(parent, item);
            return true;
        }
        match parse_array_index(&token) {
            Some(idx) if idx >= 0 && idx <= size => {
                if idx == size {
                    cJSON_AddItemToArray(parent, item);
                } else {
                    cJSON_InsertItemInArray(parent, idx, item);
                }
                true
            }
            _ => {
                cJSON_Delete(item);
                false
            }
        }
    } else if cJSON_IsObject(parent) == TRUE {
        let key = match CString::new(token) {
            Ok(k) => k,
            Err(_) => {
                cJSON_Delete(item);
                return false;
            }
        };
        let exists = !object_child_by_key(parent, key.to_bytes(), case_sensitive).is_null();
        if exists {
            if case_sensitive {
                cJSON_ReplaceItemInObjectCaseSensitive(parent, key.as_ptr(), item);
            } else {
                cJSON_ReplaceItemInObject(parent, key.as_ptr(), item);
            }
        } else {
            cJSON_AddItemToObject(parent, key.as_ptr(), item);
        }
        true
    } else {
        cJSON_Delete(item);
        false
    }
}

/// Remove whatever `path` points to within `root`. False on any failure
/// (missing parent, missing target, bad index).
unsafe fn remove_at(root: *mut cJSON, path: &[u8], case_sensitive: bool) -> bool {
    let (parent_path, token) = match split_last(path) {
        Some(v) => v,
        None => return false,
    };
    let parent = get_pointer_bytes(root, parent_path, case_sensitive);
    if parent.is_null() {
        return false;
    }
    if cJSON_IsArray(parent) == TRUE {
        match parse_array_index(&token) {
            Some(idx) if idx >= 0 && idx < cJSON_GetArraySize(parent) => {
                cJSON_DeleteItemFromArray(parent, idx);
                true
            }
            _ => false,
        }
    } else if cJSON_IsObject(parent) == TRUE {
        let key = match CString::new(token) {
            Ok(k) => k,
            Err(_) => return false,
        };
        if object_child_by_key(parent, key.to_bytes(), case_sensitive).is_null() {
            return false;
        }
        if case_sensitive {
            cJSON_DeleteItemFromObjectCaseSensitive(parent, key.as_ptr());
        } else {
            cJSON_DeleteItemFromObject(parent, key.as_ptr());
        }
        true
    } else {
        false
    }
}

/// Detach (and return ownership of) whatever `path` points to. Null on any
/// failure. Used by `move`.
unsafe fn detach_at(root: *mut cJSON, path: &[u8], case_sensitive: bool) -> *mut cJSON {
    let (parent_path, token) = match split_last(path) {
        Some(v) => v,
        None => return ptr::null_mut(),
    };
    let parent = get_pointer_bytes(root, parent_path, case_sensitive);
    if parent.is_null() {
        return ptr::null_mut();
    }
    if cJSON_IsArray(parent) == TRUE {
        match parse_array_index(&token) {
            Some(idx) if idx >= 0 && idx < cJSON_GetArraySize(parent) => {
                cJSON_DetachItemFromArray(parent, idx)
            }
            _ => ptr::null_mut(),
        }
    } else if cJSON_IsObject(parent) == TRUE {
        let key = match CString::new(token) {
            Ok(k) => k,
            Err(_) => return ptr::null_mut(),
        };
        if case_sensitive {
            cJSON_DetachItemFromObjectCaseSensitive(parent, key.as_ptr())
        } else {
            cJSON_DetachItemFromObject(parent, key.as_ptr())
        }
    } else {
        ptr::null_mut()
    }
}

/// True when `prefix` is `path` or a parent of it (at a token boundary).
fn is_pointer_prefix(prefix: &[u8], path: &[u8]) -> bool {
    if prefix.is_empty() {
        return true;
    }
    path.len() >= prefix.len()
        && &path[..prefix.len()] == prefix
        && (path.len() == prefix.len() || path[prefix.len()] == b'/')
}

/// Apply one patch operation to `root`. Returns true on success.
unsafe fn apply_one(root: *mut cJSON, op_obj: *const cJSON, case_sensitive: bool) -> bool {
    if cJSON_IsObject(op_obj) != TRUE {
        return false;
    }
    let op = match member_string(op_obj, b"op", case_sensitive) {
        Some(o) => o,
        None => return false,
    };
    let path = match member_string(op_obj, b"path", case_sensitive) {
        Some(p) => p,
        None => return false,
    };

    match op {
        b"add" => {
            let value = object_child_by_key(op_obj, b"value", case_sensitive);
            if value.is_null() {
                return false;
            }
            let dup = cJSON_Duplicate(value, TRUE);
            if dup.is_null() {
                return false;
            }
            place_item(root, path, dup, case_sensitive)
        }
        b"remove" => {
            if path.is_empty() {
                return false;
            }
            remove_at(root, path, case_sensitive)
        }
        b"replace" => {
            let value = object_child_by_key(op_obj, b"value", case_sensitive);
            if value.is_null() {
                return false;
            }
            // Target must already exist.
            if get_pointer_bytes(root, path, case_sensitive).is_null() && !path.is_empty() {
                return false;
            }
            let dup = cJSON_Duplicate(value, TRUE);
            if dup.is_null() {
                return false;
            }
            if path.is_empty() {
                overwrite_in_place(root, dup);
                return true;
            }
            // remove-then-add keeps object-key order close to upstream and
            // handles arrays uniformly.
            if !remove_at(root, path, case_sensitive) {
                cJSON_Delete(dup);
                return false;
            }
            place_item(root, path, dup, case_sensitive)
        }
        b"move" => {
            let from = match member_string(op_obj, b"from", case_sensitive) {
                Some(f) => f,
                None => return false,
            };
            if from.is_empty() {
                return false;
            }
            // RFC 6902: 'from' MUST NOT be a proper prefix of 'path'.
            if from != path && is_pointer_prefix(from, path) {
                return false;
            }
            if get_pointer_bytes(root, from, case_sensitive).is_null() {
                return false;
            }
            let detached = detach_at(root, from, case_sensitive);
            if detached.is_null() {
                return false;
            }
            place_item(root, path, detached, case_sensitive)
        }
        b"copy" => {
            let from = match member_string(op_obj, b"from", case_sensitive) {
                Some(f) => f,
                None => return false,
            };
            let src = get_pointer_bytes(root, from, case_sensitive);
            if src.is_null() {
                return false;
            }
            let dup = cJSON_Duplicate(src, TRUE);
            if dup.is_null() {
                return false;
            }
            place_item(root, path, dup, case_sensitive)
        }
        b"test" => {
            let value = object_child_by_key(op_obj, b"value", case_sensitive);
            if value.is_null() {
                return false;
            }
            let target = get_pointer_bytes(root, path, case_sensitive);
            if target.is_null() {
                return false;
            }
            cJSON_Compare(target, value, case_sensitive as cJSON_bool) == TRUE
        }
        _ => false,
    }
}

/// Apply a patch array to `object` in place. Returns 0 on success, non-zero
/// on the first failing operation (non-atomic, like cJSON).
unsafe fn apply_patches(object: *mut cJSON, patches: *const cJSON, case_sensitive: bool) -> c_int {
    if object.is_null() || patches.is_null() || cJSON_IsArray(patches) != TRUE {
        return 1;
    }
    let mut op = (*patches).child;
    while !op.is_null() {
        if !apply_one(object, op, case_sensitive) {
            return 1;
        }
        op = (*op).next;
    }
    0
}

// ---- Generate patches (RFC 6902) --------------------------------------------

/// Append `{op, path[, value]}` to `patches`.
unsafe fn add_patch(patches: *mut cJSON, op: &str, path: &[u8], value: *const cJSON) {
    let obj = cJSON_CreateObject();
    if obj.is_null() {
        return;
    }
    let opc = CString::new(op).unwrap();
    cJSON_AddItemToObject(obj, b"op\0".as_ptr().cast(), cJSON_CreateString(opc.as_ptr()));
    let pathc = CString::new(path).unwrap_or_default();
    cJSON_AddItemToObject(obj, b"path\0".as_ptr().cast(), cJSON_CreateString(pathc.as_ptr()));
    if !value.is_null() {
        cJSON_AddItemToObject(obj, b"value\0".as_ptr().cast(), cJSON_Duplicate(value, TRUE));
    }
    cJSON_AddItemToArray(patches, obj);
}

fn index_path(path: &[u8], index: usize) -> Vec<u8> {
    let mut p = path.to_vec();
    p.push(b'/');
    p.extend_from_slice(index.to_string().as_bytes());
    p
}

fn key_path(path: &[u8], key: &[u8]) -> Vec<u8> {
    let mut p = path.to_vec();
    p.push(b'/');
    p.extend_from_slice(&encode_token(key));
    p
}

/// Emit into `patches` the operations that turn `from` into `to` at `path`.
unsafe fn create_patches(patches: *mut cJSON, path: &[u8], from: *const cJSON, to: *const cJSON, cs: bool) {
    if cJSON_Compare(from, to, cs as cJSON_bool) == TRUE {
        return;
    }
    let tf = (*from).type_ & 0xFF;
    let tt = (*to).type_ & 0xFF;
    if tf != tt {
        add_patch(patches, "replace", path, to);
        return;
    }

    if tf == CJSON_ARRAY {
        let mut index: usize = 0;
        let mut fc = (*from).child;
        let mut tc = (*to).child;
        while !fc.is_null() && !tc.is_null() {
            create_patches(patches, &index_path(path, index), fc, tc, cs);
            index += 1;
            fc = (*fc).next;
            tc = (*tc).next;
        }
        // Surplus in `from`: remove at the same index repeatedly (each
        // removal shifts the tail down).
        while !fc.is_null() {
            add_patch(patches, "remove", &index_path(path, index), ptr::null());
            fc = (*fc).next;
        }
        // Surplus in `to`: append in order.
        while !tc.is_null() {
            add_patch(patches, "add", &index_path(path, index), tc);
            index += 1;
            tc = (*tc).next;
        }
    } else if tf == CJSON_OBJECT {
        // Removals: keys in `from` absent from `to`.
        let mut fc = (*from).child;
        while !fc.is_null() {
            let key = key_bytes(fc);
            if object_child_by_key(to, key, cs).is_null() {
                add_patch(patches, "remove", &key_path(path, key), ptr::null());
            }
            fc = (*fc).next;
        }
        // Additions and recursive changes for keys in `to`.
        let mut tc = (*to).child;
        while !tc.is_null() {
            let key = key_bytes(tc);
            let from_child = object_child_by_key(from, key, cs);
            if from_child.is_null() {
                add_patch(patches, "add", &key_path(path, key), tc);
            } else {
                create_patches(patches, &key_path(path, key), from_child, tc, cs);
            }
            tc = (*tc).next;
        }
    } else {
        // Scalars that compared unequal.
        add_patch(patches, "replace", path, to);
    }
}

unsafe fn generate_patches(from: *const cJSON, to: *const cJSON, cs: bool) -> *mut cJSON {
    if from.is_null() || to.is_null() {
        return ptr::null_mut();
    }
    let patches = cJSON_CreateArray();
    if patches.is_null() {
        return ptr::null_mut();
    }
    create_patches(patches, b"", from, to, cs);
    patches
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

#[no_mangle]
pub unsafe extern "C" fn cJSONUtils_ApplyPatches(object: *mut cJSON, patches: *const cJSON) -> c_int {
    guard(1, || apply_patches(object, patches, false))
}

#[no_mangle]
pub unsafe extern "C" fn cJSONUtils_ApplyPatchesCaseSensitive(
    object: *mut cJSON,
    patches: *const cJSON,
) -> c_int {
    guard(1, || apply_patches(object, patches, true))
}

#[no_mangle]
pub unsafe extern "C" fn cJSONUtils_GeneratePatches(from: *mut cJSON, to: *mut cJSON) -> *mut cJSON {
    guard(ptr::null_mut(), || generate_patches(from, to, false))
}

#[no_mangle]
pub unsafe extern "C" fn cJSONUtils_GeneratePatchesCaseSensitive(
    from: *mut cJSON,
    to: *mut cJSON,
) -> *mut cJSON {
    guard(ptr::null_mut(), || generate_patches(from, to, true))
}

#[no_mangle]
pub unsafe extern "C" fn cJSONUtils_AddPatchToArray(
    array: *mut cJSON,
    operation: *const c_char,
    path: *const c_char,
    value: *const cJSON,
) {
    guard((), || {
        if array.is_null() || operation.is_null() || path.is_null() || cJSON_IsArray(array) != TRUE {
            return;
        }
        let obj = cJSON_CreateObject();
        if obj.is_null() {
            return;
        }
        cJSON_AddItemToObject(obj, b"op\0".as_ptr().cast(), cJSON_CreateString(operation));
        cJSON_AddItemToObject(obj, b"path\0".as_ptr().cast(), cJSON_CreateString(path));
        if !value.is_null() {
            cJSON_AddItemToObject(obj, b"value\0".as_ptr().cast(), cJSON_Duplicate(value, TRUE));
        }
        cJSON_AddItemToArray(array, obj);
    })
}
