//! Minimal C memory intrinsics required by `core`/LLVM when linking a freestanding kernel.
//!
//! Our custom `x86_64-unknown-none-elf` target does not provide a libc, so we export
//! `memcpy`/`memmove`/`memset`/`memcmp` symbols from the kernel itself.

use core::ptr;

#[no_mangle]
pub unsafe extern "C" fn memcpy(dest: *mut u8, src: *const u8, n: usize) -> *mut u8 {
    if n != 0 {
        // SAFETY: caller promises valid non-overlapping regions for `n` bytes.
        unsafe { ptr::copy_nonoverlapping(src, dest, n) };
    }
    dest
}

#[no_mangle]
pub unsafe extern "C" fn memmove(dest: *mut u8, src: *const u8, n: usize) -> *mut u8 {
    if n != 0 {
        // SAFETY: caller promises valid regions for `n` bytes; overlapping is permitted.
        unsafe { ptr::copy(src, dest, n) };
    }
    dest
}

#[no_mangle]
pub unsafe extern "C" fn memset(dest: *mut u8, c: i32, n: usize) -> *mut u8 {
    if n != 0 {
        // SAFETY: caller promises `dest` is valid for `n` bytes.
        unsafe { ptr::write_bytes(dest, c as u8, n) };
    }
    dest
}

#[no_mangle]
pub unsafe extern "C" fn memcmp(lhs: *const u8, rhs: *const u8, n: usize) -> i32 {
    let mut i = 0;
    while i < n {
        // SAFETY: caller promises both regions are valid for `n` bytes.
        let a = unsafe { *lhs.add(i) };
        let b = unsafe { *rhs.add(i) };
        if a != b {
            return a as i32 - b as i32;
        }
        i += 1;
    }

    0
}
