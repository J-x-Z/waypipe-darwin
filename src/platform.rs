/* SPDX-License-Identifier: GPL-3.0-or-later */
/*! Platform-specific code.
 *
 * For platforms where Rust's `struct stat` is inaccurate, the key parameters can be
 * acquired using the following C script, to be compiled using `clang -std=c11`.
 *
 * ```c
 * #include <stdio.h>
 * #include <stddef.h>
 * #include <stdalign.h>
 * #include <sys/types.h> // for dev_t
 * #include <sys/stat.h> // for struct stat
 *
 * int main(int argc, char **argv) {
 *     (void)argc;
 *     (void)argv;
 *     printf("sizeof(dev_t) %zu\n", sizeof(dev_t));
 *     printf("alignof(dev_t) %zu\n", alignof(dev_t));
 *     printf("sizeof(struct stat) %zu\n", sizeof(struct stat));
 *     printf("alignof(struct stat) %zu\n", alignof(struct stat));
 *     printf("offsetof(struct stat, st_rdev) %zu\n", offsetof(struct stat, st_rdev));
 * }
 * ```
 */

use std::path::Path;

#[cfg(target_os = "freebsd")]
pub const SIZEOF_DEV_T: usize = 8;
#[cfg(not(target_os = "freebsd"))]
pub const SIZEOF_DEV_T: usize = std::mem::size_of::<nix::libc::dev_t>();

#[cfg(target_os = "freebsd")]
fn get_rdev_for_file_freebsd(path: &Path) -> Option<u64> {
    use core::ffi::c_char;
    use core::ffi::c_int;
    use std::ffi::CStr;
    use std::os::unix::ffi::OsStrExt;

    const SIZEOF_FREEBSD_STAT: usize = 224;
    const OFFSETOF_FREEBSD_ST_RDEV: usize = 40;

    #[repr(C, align(8))]
    struct FreeBSDStat([u8; SIZEOF_FREEBSD_STAT]);

    unsafe extern "C" {
        fn stat(path: *const c_char, buf: *mut FreeBSDStat) -> c_int;
    }

    let mut path_bytes = Vec::from(path.as_os_str().as_bytes());
    path_bytes.push(0);
    let path_str = CStr::from_bytes_with_nul(&path_bytes).unwrap();

    let mut output: FreeBSDStat = FreeBSDStat([0; SIZEOF_FREEBSD_STAT]);
    let ret: i32 = unsafe {
        /* SAFETY: requires FreeBSD >= 12.0 for the definition of the function
         * and struct stat to be correct. `stat()` does not hold onto references
         * to its input so there is no lifetime issue. */
        stat(path_str.as_ptr(), &mut output)
    };
    if ret != 0 {
        return None;
    }
    let st_rdev_bytes =
        &output.0[OFFSETOF_FREEBSD_ST_RDEV..OFFSETOF_FREEBSD_ST_RDEV + SIZEOF_DEV_T];
    let st_rdev = u64::from_ne_bytes(st_rdev_bytes.try_into().unwrap());
    Some(st_rdev)
}
#[cfg(not(target_os = "freebsd"))]
fn get_rdev_for_file_base(path: &Path) -> Option<u64> {
    use nix::sys::stat;

    let result = stat::stat(path).ok()?;
    /* st_rdev size varies by platform and may be 4 on old
     * architectures, but is typically 8 and always <= 8 */
    #[allow(clippy::useless_conversion)]
    Some(result.st_rdev as u64)
}

/** Get the (`st_rdev`) device id for the special file at `path`, cast to a u64. */
pub fn get_rdev_for_file(path: &Path) -> Option<u64> {
    #[cfg(target_os = "freebsd")]
    {
        get_rdev_for_file_freebsd(path)
    }
    #[cfg(not(target_os = "freebsd"))]
    {
        get_rdev_for_file_base(path)
    }
}
