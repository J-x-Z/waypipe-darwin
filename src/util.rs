/* SPDX-License-Identifier: GPL-3.0-or-later */
/*! Misc utilities and types */
use crate::platform::*;
use crate::wayland_gen::WlShmFormat;
use core::num::NonZeroU32;
use nix::{fcntl, libc, unistd};
use std::fmt;
use std::fmt::{Display, Formatter};
use std::fs::ReadDir;
use std::os::fd::OwnedFd;
use std::os::unix::ffi::OsStrExt;
use std::str::FromStr;

/** Like `format!`, but prepends file and line number.
 *
 * Example: `tag!("Failed to X: {} {}", arg1, arg2)` */
#[macro_export]
macro_rules! tag {
    ($x:tt) => {
        format!(concat!(std::file!(), ":", std::line!(), ": ", $x))
    };
    ($x:tt, $($arg:tt)+) => {
        format!(concat!(std::file!(), ":", std::line!(), ": ", $x), $($arg)+)
    };
}

/* Connection header constants. The original header layout is:
 *
 * 0: set iff reconnectable
 * 1: set iff update for reconnectable connection
 * 2: no dmabuf support (can be ignored as we lazily initialize)
 * 3-6: ignored
 * 7: fixed to 1
 * 8-10: compression type
 * 11-13: video type
 * 14-15: ignored
 * 16-30: version field, original Waypipe only accepts 0x1
 * 31: fixed to 0
 *
 * Waypipe's protocol does not use any interesting features early on;
 * the application side always starts by sending a Protocol-type message.
 *
 * To allow for a "silent" version upgrade, where a new version is
 * only used if acknowledged, the version field will now be interpreted as
 * follows:
 *
 * 3-6: lower bits of version
 * 16-23: upper bits of version
 *
 * All versions from 16 (=1) to 31 to should be able to interoperate
 * with original Waypipe.
 */
pub const MIN_PROTOCOL_VERSION: u32 = 0x10;
pub const WAYPIPE_PROTOCOL_VERSION: u32 = 0x11;
pub const CONN_FIXED_BIT: u32 = 0x1 << 7;
pub const CONN_UNSET_BIT: u32 = 0x1 << 31;
pub const _CONN_RECONNECTABLE_BIT: u32 = 0x1 << 0;
pub const _CONN_UPDATE_BIT: u32 = 0x1 << 1;
pub const CONN_NO_DMABUF_SUPPORT: u32 = 0x1 << 2;
pub const CONN_COMPRESSION_MASK: u32 = 0x7 << 8;
pub const CONN_NO_COMPRESSION: u32 = 0x1 << 8;
pub const CONN_LZ4_COMPRESSION: u32 = 0x2 << 8;
pub const CONN_ZSTD_COMPRESSION: u32 = 0x3 << 8;
pub const CONN_VIDEO_MASK: u32 = 0x7 << 11;
pub const CONN_NO_VIDEO: u32 = 0x1 << 11;
pub const CONN_VP9_VIDEO: u32 = 0x2 << 11;
pub const CONN_H264_VIDEO: u32 = 0x3 << 11;
pub const CONN_AV1_VIDEO: u32 = 0x4 << 11;

#[derive(Debug, PartialEq, Eq, Copy, Clone)]
pub enum WmsgType {
    /** Send over a set of Wayland protocol messages. Preceding messages
     * must create or update file descriptors and inject file descriptors
     * to the queue. */
    Protocol = 0, // header uint32_t, then protocol messages
    /** Inject file descriptors into the receiver's buffer, for use by the
     * protocol parser. */
    InjectRIDs = 1, // header uint32_t, then fds
    /** Create a new shared memory file of the given size.
     * Format: \ref wmsg_open_file */
    OpenFile = 2,
    /** Provide a new (larger) size for the file buffer.
     * Format: \ref wmsg_open_file */
    ExtendFile = 3,
    /** Create a new DMABUF with the given size and \ref dmabuf_slice_data.
     * Format: \ref wmsg_open_dmabuf */
    OpenDMABUF = 4,
    /** Fill the region of the file with the folllowing data. The data
     * should be compressed according to the global compression option.
     * Format: \ref wmsg_buffer_fill */
    BufferFill = 5,
    /** Apply a diff to the file. The diff contents may be compressed.
     * Format: \ref wmsg_buffer_diff */
    BufferDiff = 6,
    /** Create a new pipe, with the given remote R/W status */
    OpenIRPipe = 7, // wmsg_basic
    OpenIWPipe = 8, // wmsg_basic
    OpenRWPipe = 9, // wmsg_basic
    /** Transfer data to the pipe */
    PipeTransfer = 10, // wmsg_basic
    /** Shutdown the read end of the pipe that waypipe uses. */
    PipeShutdownR = 11, // wmsg_basic
    /** Shutdown the write end of the pipe that waypipe uses. */
    PipeShutdownW = 12, // wmsg_basic
    /** Create a DMABUF (with following data parameters) that will be used
     * to produce/consume video frames. Format: \ref wmsg_open_dmabuf.
     * Deprecated and may be disabled/removed in the future. */
    OpenDMAVidSrc = 13,
    OpenDMAVidDst = 14,
    /** Send a packet of video data to the destination */
    SendDMAVidPacket = 15, // wmsg_basic
    /** Acknowledge that a given number of messages has been received, so
     * that the sender of those messages no longer needs to store them
     * for replaying in case of reconnection. Format: \ref wmsg_ack */
    AckNblocks = 16,
    /** When restarting a connection, indicate the number of the message
     * which will be sent next. Format: \ref wmsg_restart */
    Restart = 17, // wmsg_restart
    /** When the remote program is closing. Format: only the header */
    Close = 18,
    /** Create a DMABUF (with following data parameters) that will be used
     * to produce/consume video frames. Format: \ref wmsg_open_dmavid */
    OpenDMAVidSrcV2 = 19,
    OpenDMAVidDstV2 = 20,
    /* Create a DRM syncobj timeline semaphore. Format: header, u64-le initial point */
    OpenTimeline = 21,
    /* Signal the indicated DRM syncobj timeline semaphore.  Format: header, u64-le initial point. */
    SignalTimeline = 22,
    /* Sent as the first message from the client to reveal the negotiated wire protocol
     * version. Format: header, u32 version field */
    Version = 23,
}

pub fn align(x: usize, y: usize) -> usize {
    y * ((x.checked_add(y - 1).unwrap()) / y)
}
pub fn align4(x: usize) -> usize {
    align(x, 4)
}
pub fn cat2x4(x: [u8; 4], y: [u8; 4]) -> [u8; 8] {
    [x[0], x[1], x[2], x[3], y[0], y[1], y[2], y[3]]
}
pub fn cat3x4(x: [u8; 4], y: [u8; 4], z: [u8; 4]) -> [u8; 12] {
    [
        x[0], x[1], x[2], x[3], y[0], y[1], y[2], y[3], z[0], z[1], z[2], z[3],
    ]
}
pub fn cat4x4(x: [u8; 4], y: [u8; 4], z: [u8; 4], a: [u8; 4]) -> [u8; 16] {
    [
        x[0], x[1], x[2], x[3], y[0], y[1], y[2], y[3], z[0], z[1], z[2], z[3], a[0], a[1], a[2],
        a[3],
    ]
}
pub fn split_interval(lo: u32, hi: u32, nparts: u32, index: u32) -> u32 {
    assert!(nparts < 1 << 15 && hi - lo < 1 << 31);
    lo + index * ((hi - lo) / nparts) + (index * ((hi - lo) % nparts)) / nparts
}
pub fn ceildiv(v: u32, u: u32) -> u32 {
    v.div_ceil(u)
}
/* Split u64 into high (32:63) and low (0:31) parts */
pub fn split_u64(x: u64) -> (u32, u32) {
    ((x >> 32) as u32, x as u32)
}
pub fn join_u64(hi: u32, lo: u32) -> u64 {
    ((hi as u64) << 32) | (lo as u64)
}

pub fn build_wmsg_header(typ: WmsgType, len: usize) -> u32 {
    u32::try_from(len).unwrap().checked_mul(1 << 5).unwrap() | (typ as u32)
}

/** The size excludes trailing padding (to multiple of 4). */
pub fn parse_wmsg_header(header: u32) -> Option<(usize, WmsgType)> {
    let code = header & ((1 << 5) - 1);
    let len = (header >> 5) as usize;
    let t = match code {
        0 => WmsgType::Protocol,
        1 => WmsgType::InjectRIDs,
        2 => WmsgType::OpenFile,
        3 => WmsgType::ExtendFile,
        4 => WmsgType::OpenDMABUF,
        5 => WmsgType::BufferFill,
        6 => WmsgType::BufferDiff,
        7 => WmsgType::OpenIRPipe,
        8 => WmsgType::OpenIWPipe,
        9 => WmsgType::OpenRWPipe,
        10 => WmsgType::PipeTransfer,
        11 => WmsgType::PipeShutdownR,
        12 => WmsgType::PipeShutdownW,
        13 => WmsgType::OpenDMAVidSrc,
        14 => WmsgType::OpenDMAVidDst,
        15 => WmsgType::SendDMAVidPacket,
        16 => WmsgType::AckNblocks,
        17 => WmsgType::Restart,
        18 => WmsgType::Close,
        19 => WmsgType::OpenDMAVidSrcV2,
        20 => WmsgType::OpenDMAVidDstV2,
        21 => WmsgType::OpenTimeline,
        22 => WmsgType::SignalTimeline,
        23 => WmsgType::Version,
        _ => {
            return None;
        }
    };
    Some((len, t))
}

pub fn retain_err<T, F, E>(x: &mut Vec<T>, mut f: F) -> Result<(), E>
where
    F: FnMut(&mut T) -> Result<bool, E>,
{
    let mut e: Result<(), E> = Ok(());
    x.retain_mut(|y| match f(y) {
        Ok(b) => b,
        Err(x) => {
            e = Err(x);
            /* It doesn't matter whether we keep or exit in this case */
            true
        }
    });
    e
}

/** Helper function to write into a finite length buffer. It does not propagate
 * errors and is most convenient if the write is certain to succeed. Example use:
 *
 * ```
 * let mut buf = [0u8; 256];
 * let slice = write_with_buffer(&mut buf, &|x: &mut &mut [u8]| {
 *     write!(x, "{}", number).expect("buffer should be long enough")
 * });
 * ```
 */
pub fn write_with_buffer<F: Fn(&mut &mut [u8])>(b: &mut [u8], f: F) -> &[u8] {
    let len = b.len();
    let mut tmp = &mut b[..];
    f(&mut tmp);
    let slice_len = len - tmp.len();
    &b[..slice_len]
}

/** A type to escape Wayland interface names, which should only consist of [a-zA-Z0-9_] */
pub struct EscapeWlName<'a>(pub &'a [u8]);
impl Display for EscapeWlName<'_> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        for c in self.0 {
            match *c {
                b'_' | b'a'..=b'z' | b'0'..=b'9' | b'A'..=b'Z' => {
                    write!(f, "{}", char::from_u32(*c as u32).unwrap())
                }
                _ => {
                    write!(f, "\\x{:02x}", *c)
                }
            }?
        }
        Ok(())
    }
}

/** A type to escape all non-ascii-printable characters when Displayed, to leave strings
 * somewhat legible but make it clear exactly what bytes they contain */
pub struct EscapeAsciiPrintable<'a>(pub &'a [u8]);
impl Display for EscapeAsciiPrintable<'_> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        for c in self.0 {
            match *c {
                b' '..=b'~' => write!(f, "{}", char::from_u32(*c as u32).unwrap()),
                _ => {
                    write!(f, "\\x{:02x}", *c)
                }
            }?
        }
        Ok(())
    }
}

/** Format a bool as 'T' or 'F' */
pub fn fmt_bool(x: bool) -> char {
    if x {
        'T'
    } else {
        'f'
    }
}

/** Return the string iff `x`, otherwise empty string. Can be efficient
 * for logging conditions that are rarely true. */
pub fn string_if_bool(x: bool, y: &str) -> &str {
    if x {
        y
    } else {
        ""
    }
}

/* A heap-allocated 64-aligned array */
pub struct AlignedArray {
    data: *mut u8,
    size: usize,
}
unsafe impl Send for AlignedArray {}
unsafe impl Sync for AlignedArray {}

impl AlignedArray {
    pub fn new(size: usize) -> AlignedArray {
        if size == 0 {
            AlignedArray {
                data: std::ptr::null_mut(),
                size: 0,
            }
        } else {
            let layout = std::alloc::Layout::from_size_align(size, 64).unwrap();

            unsafe {
                // SAFETY: layout size was checked to be > 0
                let mem = std::alloc::alloc_zeroed(layout).cast::<u8>();
                assert!(!mem.is_null());
                AlignedArray { data: mem, size }
            }
        }
    }
    /* Returns (ptr, len); ptr is promised to be 64 aligned */
    pub fn get_parts(&self) -> (*mut u8, usize) {
        (self.data, self.size)
    }
    pub fn get_mut(&mut self) -> &mut [u8] {
        if self.size == 0 {
            return &mut [];
        }
        unsafe {
            // SAFETY: self.data is not null since size > 0 was checked
            // data is 64-aligned, and only 1-alignment needed for u8
            // size matches allocated amount
            // &mut self argument ensures no other calls to get_mut() can
            // overlap in lifespan, so slice is not otherwise accessed; other unsafe
            // users of AlignedArray should enforce similar behavior
            &mut *std::ptr::slice_from_raw_parts_mut(self.data, self.size)
        }
    }
    pub fn get(&self) -> &[u8] {
        if self.size == 0 {
            return &[];
        }
        unsafe {
            // SAFETY: bounds OK else allocation would fail, todo
            &*std::ptr::slice_from_raw_parts(self.data, self.size)
        }
    }
}
impl Drop for AlignedArray {
    fn drop(&mut self) {
        if self.size > 0 {
            let layout = std::alloc::Layout::from_size_align(self.size, 64).unwrap();
            unsafe {
                // SAFETY: self.data is not null and was allocated with the same layout
                std::alloc::dealloc(self.data, layout);
            }
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Compression {
    None,
    Lz4(i8),
    Zstd(i8),
}
impl FromStr for Compression {
    type Err = &'static str;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        const FAILURE: &str = "Compression should have format: 'none', 'lz4[=#]', or 'zstd[=#]'";
        if s == "none" {
            Ok(Compression::None)
        } else if s.starts_with("lz4") {
            let lvl: i8;
            if s == "lz4" {
                lvl = 0;
            } else if let Some(suffix) = s.strip_prefix("lz4=") {
                lvl = suffix.parse::<i8>().map_err(|_| FAILURE)?;
            } else {
                return Err(FAILURE);
            }

            Ok(Compression::Lz4(lvl))
        } else if s.starts_with("zstd") {
            let lvl: i8;
            if s == "zstd" {
                lvl = 0;
            } else if let Some(suffix) = s.strip_prefix("zstd=") {
                lvl = suffix.parse::<i8>().map_err(|_| FAILURE)?;
            } else {
                return Err(FAILURE);
            }

            Ok(Compression::Zstd(lvl))
        } else {
            Err(FAILURE)
        }
    }
}
impl fmt::Display for Compression {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Compression::None => write!(f, "none"),
            Compression::Lz4(i) => {
                if *i == 0 {
                    write!(f, "lz4")
                } else {
                    write!(f, "lz4={}", i)
                }
            }
            Compression::Zstd(i) => {
                if *i == 0 {
                    write!(f, "zstd")
                } else {
                    write!(f, "zstd={}", i)
                }
            }
        }
    }
}
#[test]
fn compression_enum_roundtrip() {
    assert_eq!(
        Compression::from_str(&Compression::None.to_string()),
        Ok(Compression::None)
    );
    for i in i8::MIN..=i8::MAX {
        assert_eq!(
            Compression::from_str(&Compression::Lz4(i).to_string()),
            Ok(Compression::Lz4(i))
        );
        assert_eq!(
            Compression::from_str(&Compression::Zstd(i).to_string()),
            Ok(Compression::Zstd(i))
        );
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum VideoFormat {
    /* Values are used in wire protocol */
    H264 = 0,
    VP9 = 1,
    AV1 = 2,
}
/** Whether to prefer software or hardware encoding, when available */
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum CodecPreference {
    SW = 0,
    HW = 1,
}

/** Configuration for video encoding/decoding */
#[derive(Debug, Copy, Clone, PartialEq, Default)]
pub struct VideoSetting {
    /* If not set, no video encoding done */
    pub format: Option<VideoFormat>,
    /* If not set, default */
    pub bits_per_frame: Option<f32>,
    pub enc_pref: Option<CodecPreference>,
    pub dec_pref: Option<CodecPreference>,
}

impl FromStr for VideoSetting {
    type Err = &'static str;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        const FAILURE: &str =
            "Video spec should be comma-separated list containing any of: 'none', 'h264', 'vp9', 'av1', 'sw', 'hw', 'hwenc', 'swenc', 'hwdec', 'swdec', 'bpf=<real>'";
        let mut f = VideoSetting {
            format: None,
            bits_per_frame: None,
            enc_pref: None,
            dec_pref: None,
        };

        for chunk in s.split_terminator(',') {
            if chunk == "none" {
                f.format = None;
            } else if chunk == "hw" {
                f.enc_pref = Some(CodecPreference::HW);
                f.dec_pref = Some(CodecPreference::HW);
            } else if chunk == "sw" {
                f.enc_pref = Some(CodecPreference::SW);
                f.dec_pref = Some(CodecPreference::SW);
            } else if chunk == "swenc" {
                f.enc_pref = Some(CodecPreference::SW);
            } else if chunk == "hwenc" {
                f.enc_pref = Some(CodecPreference::HW);
            } else if chunk == "swdec" {
                f.dec_pref = Some(CodecPreference::SW);
            } else if chunk == "hwdec" {
                f.dec_pref = Some(CodecPreference::HW);
            } else if chunk == "h264" {
                f.format = Some(VideoFormat::H264);
            } else if chunk == "vp9" {
                f.format = Some(VideoFormat::VP9);
            } else if chunk == "av1" {
                f.format = Some(VideoFormat::AV1);
            } else if let Some(suffix) = chunk.strip_prefix("bpf=") {
                f.bits_per_frame = Some(suffix.parse::<f32>().map_err(|_| FAILURE)?);
            } else {
                return Err(FAILURE);
            }
        }
        Ok(f)
    }
}
impl fmt::Display for VideoSetting {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if let Some(fmt) = &self.format {
            match fmt {
                VideoFormat::H264 => write!(f, "h264")?,
                VideoFormat::AV1 => write!(f, "av1")?,
                VideoFormat::VP9 => write!(f, "vp9")?,
            };
        } else {
            write!(f, "none")?;
        }

        if self.enc_pref == Some(CodecPreference::SW) && self.dec_pref == Some(CodecPreference::SW)
        {
            write!(f, ",sw")?;
        } else if self.enc_pref == Some(CodecPreference::HW)
            && self.dec_pref == Some(CodecPreference::HW)
        {
            write!(f, ",hw")?;
        } else {
            if let Some(p) = self.enc_pref {
                match p {
                    CodecPreference::SW => write!(f, ",swenc")?,
                    CodecPreference::HW => write!(f, ",hwenc")?,
                }
            }
            if let Some(p) = self.dec_pref {
                match p {
                    CodecPreference::SW => write!(f, ",swdec")?,
                    CodecPreference::HW => write!(f, ",hwdec")?,
                }
            }
        }

        if let Some(bpf) = self.bits_per_frame {
            write!(f, ",bpf={}", bpf)?;
        }

        Ok(())
    }
}
#[test]
fn video_setting_roundtrip() {
    let examples = [
        VideoSetting {
            format: None,
            bits_per_frame: None,
            enc_pref: None,
            dec_pref: None,
        },
        VideoSetting {
            format: None,
            bits_per_frame: Some(1e9),
            enc_pref: Some(CodecPreference::SW),
            dec_pref: None,
        },
        VideoSetting {
            format: Some(VideoFormat::H264),
            bits_per_frame: Some(100.0),
            enc_pref: None,
            dec_pref: Some(CodecPreference::HW),
        },
        VideoSetting {
            format: Some(VideoFormat::VP9),
            bits_per_frame: Some(4321.0),
            enc_pref: Some(CodecPreference::SW),
            dec_pref: Some(CodecPreference::HW),
        },
        VideoSetting {
            format: Some(VideoFormat::H264),
            bits_per_frame: None,
            enc_pref: Some(CodecPreference::SW),
            dec_pref: Some(CodecPreference::SW),
        },
        VideoSetting {
            format: Some(VideoFormat::AV1),
            bits_per_frame: None,
            enc_pref: Some(CodecPreference::HW),
            dec_pref: Some(CodecPreference::HW),
        },
    ];
    for v in examples {
        println!("{}", VideoSetting::to_string(&v));
        assert_eq!(VideoSetting::from_str(&VideoSetting::to_string(&v)), Ok(v));
    }
}

#[derive(Debug)]
pub struct AddDmabufPlane {
    pub fd: OwnedFd,
    pub plane_idx: u32,
    pub offset: u32,
    pub stride: u32,
    pub modifier: u64,
}

/** Construct a fourcc code from the component letters */
pub const fn fourcc(a: char, b: char, c: char, d: char) -> u32 {
    u32::from_le_bytes([(a as u8), (b as u8), (c as u8), (d as u8)])
}

pub fn list_render_device_ids() -> Vec<u64> {
    use std::os::unix::ffi::OsStrExt;

    let mut dev_ids = Vec::new();
    let Ok(dir_iter) = std::fs::read_dir("/dev/dri") else {
        /* On failure, assume Vulkan is not available */
        return dev_ids;
    };

    for r in dir_iter {
        let std::io::Result::Ok(entry) = r else {
            continue;
        };
        if !entry.file_name().as_bytes().starts_with(b"renderD") {
            continue;
        }
        let Some(rdev) = get_rdev_for_file(&entry.path()) else {
            continue;
        };
        dev_ids.push(rdev);
    }
    dev_ids
}

/** Open the render node with specified device id*/
pub fn drm_open_render(dev_id: u64, rdrw: bool) -> Result<OwnedFd, String> {
    /* On Linux, the render node is usually /dev/dri/renderD$X where $X is the
     * minor value/lowest 8 bits, but this may not be the case on all platforms */
    let rd: ReadDir =
        std::fs::read_dir("/dev/dri").map_err(|x| tag!("Failed to read /dev/dri/: {}", x))?;
    for entry in rd {
        let e = entry.map_err(|x| tag!("Failed to read entry in /dev/dri: {}", x))?;

        /* Restrict to render nodes */
        if e.file_name().as_bytes().starts_with(b"renderD") {
            let path = e.path();
            let Some(rdev) = get_rdev_for_file(&path) else {
                continue;
            };
            /* Note: technically there is a check-vs-open race condition here, but
             * render nodes rarely change. It could be avoided using `fstat`. */
            if rdev == dev_id {
                let mut flags = fcntl::OFlag::O_CLOEXEC | fcntl::OFlag::O_NOCTTY;
                if rdrw {
                    flags |= fcntl::OFlag::O_RDWR;
                }
                return fcntl::open(&path, flags, nix::sys::stat::Mode::empty())
                    .map_err(|x| tag!("Failed to open drm node fd at '{:?}': {}", path, x));
            }
        }
    }
    Err(tag!(
        "Failed to find render node with device id 0x{:x}",
        dev_id,
    ))
}

/** Provide contents of dmabuf_slice_data, pretending the buffer has a linear modifier
 * and is tightly packed. */
pub fn dmabuf_slice_make_ideal(drm_format: u32, width: u32, height: u32, bpp: u32) -> [u8; 64] {
    let mut out = [0; 64];
    out[0..4].copy_from_slice(&width.to_le_bytes());
    out[4..8].copy_from_slice(&height.to_le_bytes());
    out[8..12].copy_from_slice(&drm_format.to_le_bytes());
    out[12..16].copy_from_slice(&1u32.to_le_bytes());

    let offset = 0_u32;
    out[16..20].copy_from_slice(&offset.to_le_bytes());
    let stride = width.checked_mul(bpp).unwrap();
    out[32..36].copy_from_slice(&stride.to_le_bytes());

    /* This modifier is only ever used by waypipe-c to decide what buffer type to create */
    out[48..56].copy_from_slice(&0_u64.to_le_bytes());
    /* Link plane to dmabuf */
    out[56] = 1;

    out
}

/** Get the stride from a dmabuf_slice_data; waypipe-c will interpret this as the nominal stride. */
pub fn dmabuf_slice_get_first_stride(data: [u8; 64]) -> u32 {
    u32::from_le_bytes(data[32..36].try_into().unwrap())
}

/** Set the close-on-exec flag for a file descriptor */
pub fn set_cloexec(fd: &OwnedFd, cloexec: bool) -> Result<(), String> {
    fcntl::fcntl(
        fd,
        fcntl::FcntlArg::F_SETFD(if cloexec {
            fcntl::FdFlag::FD_CLOEXEC
        } else {
            fcntl::FdFlag::empty()
        }),
    )
    .map_err(|x| tag!("Failed to set cloexec flag: {:?}", x))?;
    Ok(())
}

/** Set the O_NONBLOCK flag for the file description, clearing all other flags. */
pub fn set_nonblock(fd: &OwnedFd) -> Result<(), String> {
    fcntl::fcntl(fd, fcntl::FcntlArg::F_SETFL(nix::fcntl::OFlag::O_NONBLOCK))
        .map_err(|x| tag!("Failed to set nonblocking: {:?}", x))?;
    Ok(())
}

/** Unset the O_NONBLOCK flag for the file description, clearing all other flags. */
pub fn set_blocking(fd: &OwnedFd) -> Result<(), String> {
    fcntl::fcntl(fd, fcntl::FcntlArg::F_SETFL(nix::fcntl::OFlag::empty()))
        .map_err(|x| tag!("Failed to set blocking: {:?}", x))?;
    Ok(())
}

/** Given blocking `fd`, read exactly enough to fill `data`, or return error.
 * This blocks until completion or error. Returns Err(None) on EOF. */
pub fn read_exact(fd: &OwnedFd, data: &mut [u8]) -> Result<(), Option<nix::Error>> {
    let mut offset = 0;
    while offset < data.len() {
        match unistd::read(fd, &mut data[offset..]) {
            Ok(s) => {
                if s == 0 {
                    return Err(None);
                }
                offset += s;
            }
            Err(nix::errno::Errno::EINTR) => {
                continue;
            }
            Err(code) => {
                /* Note: since `fd` should not have O_NONBLOCK, EAGAIN is unexpected */
                return Err(Some(code));
            }
        }
    }
    Ok(())
}

/** Given blocking `fd`, write `data`, or return error if not everything was written.
 * This blocks until completion or error. */
pub fn write_exact(fd: &OwnedFd, data: &[u8]) -> nix::Result<()> {
    let mut offset = 0;
    while offset < data.len() {
        match unistd::write(fd, &data[offset..]) {
            Ok(s) => {
                offset += s;
            }
            Err(nix::errno::Errno::EINTR) => {
                continue;
            }
            Err(code) => {
                /* Note: since `fd` should not have O_NONBLOCK, EAGAIN is unexpected */
                return Err(code);
            }
        }
    }
    Ok(())
}

/** A very simple and fast pseudorandom generator; output is only hard to
 * predict for very restricted; this should be enough to fool a branch
 * predictor or general-purpose compression algorithm, but should not be
 * used outside test or benchmarking code. */
pub struct BadRng {
    pub state: u64,
}

impl BadRng {
    /** Get a new u64 value */
    pub fn next(&mut self) -> u64 {
        // Xorshift RNG, see Marsaglia 2003
        self.state ^= self.state << 13;
        self.state ^= self.state >> 7;
        self.state ^= self.state << 17;
        self.state
    }
    /** Get a new value, in the range 0..maxval; this is only approximately uniform */
    pub fn next_usize(&mut self, maxval: usize) -> usize {
        (self.next() % maxval as u64) as usize
    }
}

/** Basic layout parameters of a Wayland/drm_fourcc.h linear layout format plane; these are
 * sufficient to describe where data for a pixel is, within a plane, but do not describe
 * the exact way the data is encoded. */
#[derive(Clone, Copy)]
pub struct PlaneLayout {
    /** Bytes per (subsampled) texel block */
    pub bpt: NonZeroU32,
    /** Horizontal subsampling ratio (width / texel block count) */
    pub hsub: NonZeroU32,
    /** Vertical subsampling ratio (height / texel block count) */
    pub vsub: NonZeroU32,
    /** Width of a texel block in pixels; this should divide hsub. */
    pub htex: NonZeroU32,
    /** Height of a texel block in pixels; this should divide vsub. */
    pub vtex: NonZeroU32,
}

/** Basic layout parameters for a (possibly) multiplanar Wayland/drm_fourcc.h linear layout format.
 * These are sufficient to determine, given width/height/offset/stride parameters, where the data
 * corresponding to a given pixel is, but do not determine the exact way the data is encoded.
 */
pub struct FormatLayout {
    pub planes: &'static [PlaneLayout],
}

/** Convert a DRM fourcc format code to a Wayland format code.
 *
 * Wayland and DRM differ in encodings for Argb8888 and Xrgb8888 only.
 *
 * Both names for Argb8888/Xrgb8888 can safely be used to specify wl_shm formats if
 * the compositor advertised both, but only DRM formats are permitted for linux-dmabuf.
 */
pub fn drm_to_wayland(drm_format: u32) -> u32 {
    if drm_format == fourcc('A', 'R', '2', '4') {
        WlShmFormat::Argb8888 as u32
    } else if drm_format == fourcc('X', 'R', '2', '4') {
        WlShmFormat::Xrgb8888 as u32
    } else {
        drm_format
    }
}

/** Get the layout for a wl_shm/drm_fourcc format. Returns None if the format is unsupported
 * (either being entirely invalid, or a format lacking any linear layout.) */
pub fn get_shm_format_layout(format: u32) -> Option<FormatLayout> {
    use crate::wayland_gen::WlShmFormat::*;

    /* Safety: values are not zero */
    const N1: NonZeroU32 = unsafe { NonZeroU32::new_unchecked(1) };
    const N2: NonZeroU32 = unsafe { NonZeroU32::new_unchecked(2) };
    const N3: NonZeroU32 = unsafe { NonZeroU32::new_unchecked(3) };
    const N4: NonZeroU32 = unsafe { NonZeroU32::new_unchecked(4) };
    const N5: NonZeroU32 = unsafe { NonZeroU32::new_unchecked(5) };
    const N6: NonZeroU32 = unsafe { NonZeroU32::new_unchecked(6) };
    const N8: NonZeroU32 = unsafe { NonZeroU32::new_unchecked(8) };

    const SIMPLE_1: PlaneLayout = PlaneLayout {
        bpt: N1,
        hsub: N1,
        vsub: N1,
        htex: N1,
        vtex: N1,
    };
    const SIMPLE_2: PlaneLayout = PlaneLayout {
        bpt: N2,
        hsub: N1,
        vsub: N1,
        htex: N1,
        vtex: N1,
    };

    /* Formats which do not have a wl_shm name yet */
    const NV20: u32 = fourcc('N', 'V', '2', '0');
    const NV30: u32 = fourcc('N', 'V', '3', '0');
    match format {
        NV20 => {
            return Some(FormatLayout {
                planes: &[
                    PlaneLayout {
                        bpt: N5,
                        hsub: N4,
                        vsub: N1,
                        htex: N4,
                        vtex: N1,
                    },
                    PlaneLayout {
                        bpt: N5,
                        hsub: N4,
                        vsub: N1,
                        htex: N2,
                        vtex: N1,
                    },
                ],
            })
        }
        NV30 => {
            return Some(FormatLayout {
                planes: &[
                    PlaneLayout {
                        bpt: N5,
                        hsub: N4,
                        vsub: N1,
                        htex: N4,
                        vtex: N1,
                    },
                    PlaneLayout {
                        // ?
                        bpt: N5,
                        hsub: N2,
                        vsub: N1,
                        htex: N2,
                        vtex: N1,
                    },
                ],
            });
        }
        _ => (),
    }

    let f: WlShmFormat = (drm_to_wayland(format)).try_into().ok()?;

    Some(match f {
        R8 | C8 | D8 | Rgb332 | Bgr233 => FormatLayout {
            planes: &[SIMPLE_1],
        },

        Xrgb4444 | Xbgr4444 | Rgbx4444 | Bgrx4444 | Argb4444 | Abgr4444 | Rgba4444 | Bgra4444
        | Xrgb1555 | Xbgr1555 | Rgbx5551 | Bgrx5551 | Argb1555 | Abgr1555 | Rgba5551 | Bgra5551
        | Rgb565 | Bgr565 | R10 | R12 | R16 | Rg88 | Gr88 => FormatLayout {
            planes: &[SIMPLE_2],
        },

        Rgb888 | Bgr888 | Vuy888 => FormatLayout {
            planes: &[PlaneLayout {
                bpt: N3,
                hsub: N1,
                vsub: N1,
                htex: N1,
                vtex: N1,
            }],
        },

        Argb8888 | Xrgb8888 | Xbgr8888 | Rgbx8888 | Bgrx8888 | Abgr8888 | Rgba8888 | Bgra8888
        | Xrgb2101010 | Xbgr2101010 | Rgbx1010102 | Bgrx1010102 | Argb2101010 | Abgr2101010
        | Rgba1010102 | Bgra1010102 | Ayuv | Avuy8888 | Xvuy8888 | Xyuv8888 | Xvyu2101010
        | Rg1616 | Gr1616 | Y410 => FormatLayout {
            planes: &[PlaneLayout {
                bpt: N4,
                hsub: N1,
                vsub: N1,
                htex: N1,
                vtex: N1,
            }],
        },

        Xrgb16161616f | Xbgr16161616f | Argb16161616f | Abgr16161616f | Xrgb16161616
        | Xbgr16161616 | Argb16161616 | Abgr16161616 | Axbxgxrx106106106106 | Xvyu1216161616
        | Xvyu16161616 | Y412 | Y416 => FormatLayout {
            planes: &[PlaneLayout {
                bpt: N8,
                hsub: N1,
                vsub: N1,
                htex: N1,
                vtex: N1,
            }],
        },

        R1 | C1 | D1 => FormatLayout {
            planes: &[PlaneLayout {
                bpt: N1,
                hsub: N8,
                vsub: N1,
                htex: N8,
                vtex: N1,
            }],
        },
        R2 | C2 | D2 => FormatLayout {
            planes: &[PlaneLayout {
                bpt: N1,
                hsub: N4,
                vsub: N1,
                htex: N4,
                vtex: N1,
            }],
        },
        R4 | C4 | D4 => FormatLayout {
            planes: &[PlaneLayout {
                bpt: N1,
                hsub: N2,
                vsub: N1,
                htex: N2,
                vtex: N1,
            }],
        },
        Yuyv | Yvyu | Uyvy | Vyuy => FormatLayout {
            planes: &[PlaneLayout {
                bpt: N4,
                hsub: N2,
                vsub: N1,
                htex: N2,
                vtex: N1,
            }],
        },
        Y210 | Y212 | Y216 => FormatLayout {
            planes: &[PlaneLayout {
                bpt: N8,
                hsub: N2,
                vsub: N1,
                htex: N2,
                vtex: N1,
            }],
        },
        Y0l0 | X0l0 | Y0l2 | X0l2 => FormatLayout {
            planes: &[PlaneLayout {
                bpt: N8,
                hsub: N2,
                vsub: N2,
                htex: N2,
                vtex: N2,
            }],
        },

        Rgb565A8 | Bgr565A8 => FormatLayout {
            planes: &[SIMPLE_2, SIMPLE_1],
        },

        Rgb888A8 | Bgr888A8 => FormatLayout {
            planes: &[
                PlaneLayout {
                    bpt: N3,
                    hsub: N1,
                    vsub: N1,
                    htex: N1,
                    vtex: N1,
                },
                SIMPLE_1,
            ],
        },

        Xrgb8888A8 | Xbgr8888A8 | Rgbx8888A8 | Bgrx8888A8 => FormatLayout {
            planes: &[
                PlaneLayout {
                    bpt: N4,
                    hsub: N1,
                    vsub: N1,
                    htex: N1,
                    vtex: N1,
                },
                SIMPLE_1,
            ],
        },

        Nv12 | Nv21 => FormatLayout {
            planes: &[
                SIMPLE_1,
                PlaneLayout {
                    bpt: N2,
                    hsub: N2,
                    vsub: N2,
                    htex: N1,
                    vtex: N1,
                },
            ],
        },
        Nv16 | Nv61 => FormatLayout {
            planes: &[
                SIMPLE_1,
                PlaneLayout {
                    bpt: N2,
                    hsub: N2,
                    vsub: N1,
                    htex: N1,
                    vtex: N1,
                },
            ],
        },
        Nv24 | Nv42 => FormatLayout {
            planes: &[SIMPLE_1, SIMPLE_2],
        },
        Nv15 => FormatLayout {
            planes: &[
                PlaneLayout {
                    bpt: N5,
                    hsub: N4,
                    vsub: N1,
                    htex: N4,
                    vtex: N1,
                },
                PlaneLayout {
                    bpt: N5,
                    hsub: N4,
                    vsub: N2,
                    htex: N2,
                    vtex: N1,
                },
            ],
        },

        P210 | P010 | P012 | P016 => FormatLayout {
            planes: &[
                SIMPLE_2,
                PlaneLayout {
                    bpt: N4,
                    hsub: N2,
                    vsub: N2,
                    htex: N1,
                    vtex: N1,
                },
            ],
        },
        P030 => FormatLayout {
            planes: &[
                PlaneLayout {
                    bpt: N4,
                    hsub: N3,
                    vsub: N1,
                    htex: N3,
                    vtex: N1,
                },
                PlaneLayout {
                    bpt: N8,
                    hsub: N6,
                    vsub: N2,
                    htex: N3,
                    vtex: N1,
                },
            ],
        },

        Yuv410 | Yvu410 => FormatLayout {
            planes: &[
                SIMPLE_1,
                PlaneLayout {
                    bpt: N1,
                    hsub: N4,
                    vsub: N4,
                    htex: N1,
                    vtex: N1,
                },
                PlaneLayout {
                    bpt: N1,
                    hsub: N4,
                    vsub: N4,
                    htex: N1,
                    vtex: N1,
                },
            ],
        },
        Yuv411 | Yvu411 => FormatLayout {
            planes: &[
                SIMPLE_1,
                PlaneLayout {
                    bpt: N1,
                    hsub: N4,
                    vsub: N1,
                    htex: N1,
                    vtex: N1,
                },
                PlaneLayout {
                    bpt: N1,
                    hsub: N4,
                    vsub: N1,
                    htex: N1,
                    vtex: N1,
                },
            ],
        },
        Yuv420 | Yvu420 => FormatLayout {
            planes: &[
                SIMPLE_1,
                PlaneLayout {
                    bpt: N1,
                    hsub: N2,
                    vsub: N2,
                    htex: N1,
                    vtex: N1,
                },
                PlaneLayout {
                    bpt: N1,
                    hsub: N2,
                    vsub: N2,
                    htex: N1,
                    vtex: N1,
                },
            ],
        },
        Yuv422 | Yvu422 => FormatLayout {
            planes: &[
                SIMPLE_1,
                PlaneLayout {
                    bpt: N1,
                    hsub: N2,
                    vsub: N1,
                    htex: N1,
                    vtex: N1,
                },
                PlaneLayout {
                    bpt: N1,
                    hsub: N2,
                    vsub: N1,
                    htex: N1,
                    vtex: N1,
                },
            ],
        },
        Yuv444 | Yvu444 => FormatLayout {
            planes: &[SIMPLE_1, SIMPLE_1, SIMPLE_1],
        },

        Q401 | Q410 => FormatLayout {
            planes: &[SIMPLE_2, SIMPLE_2, SIMPLE_2],
        },

        Yuv4208bit | Yuv42010bit | Vuy101010 => {
            return None;
        }
    })
}

/** Build a libc::timespec from seconds and nanoseconds.
 *
 * libc's issue 3223 [1] means that libc::timespec cannot be constructed as a
 * struct literal on some 32 bit platforms that use 64 bit time. This includes
 * Debian's armhf after the t64 transition. This function can then be used as a
 * workaround constructor.
 *
 * [1] https://github.com/rust-lang/libc/issues/3223
 */
pub fn timespec_from_sec_nsec(sec: libc::time_t, nsec: libc::c_long) -> libc::timespec {
    let mut ret: libc::timespec = unsafe { std::mem::zeroed() };
    ret.tv_sec = sec;
    ret.tv_nsec = nsec;
    ret
}

/** Create an anonymous shared-memory file suitable for passing over Wayland. */
#[cfg(target_os = "linux")]
pub fn create_anon_file() -> Result<OwnedFd, String> {
    use nix::sys::memfd;

    memfd::memfd_create(
        c"waypipe",
        memfd::MFdFlags::MFD_CLOEXEC | memfd::MFdFlags::MFD_ALLOW_SEALING,
    )
    .map_err(|x| format!("memfd_create failed: {}", x))
}

/** Darwin has no memfd_create; unlink a securely-created temporary file instead. */
#[cfg(not(target_os = "linux"))]
pub fn create_anon_file() -> Result<OwnedFd, String> {
    use std::fs::OpenOptions;
    use std::os::unix::fs::OpenOptionsExt;

    for _ in 0..16 {
        let mut random = [0_u8; 16];
        getrandom::getrandom(&mut random)
            .map_err(|x| format!("failed to generate temporary file name: {}", x))?;
        let name = random
            .iter()
            .fold(String::with_capacity(32), |mut out, byte| {
                use std::fmt::Write;
                let _ = write!(out, "{byte:02x}");
                out
            });
        let path = std::env::temp_dir().join(format!("waypipe-{name}"));
        match OpenOptions::new()
            .read(true)
            .write(true)
            .create_new(true)
            .mode(0o600)
            .custom_flags(libc::O_CLOEXEC)
            .open(&path)
        {
            Ok(file) => {
                std::fs::remove_file(&path)
                    .map_err(|x| format!("failed to unlink anonymous file {:?}: {}", path, x))?;
                return Ok(file.into());
            }
            Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => continue,
            Err(error) => return Err(format!("failed to create anonymous file: {}", error)),
        }
    }
    Err("failed to allocate a unique anonymous file name".into())
}

/** Create a pipe with pipe2 semantics on platforms that only expose pipe. */
pub fn create_pipe(flags: fcntl::OFlag) -> nix::Result<(OwnedFd, OwnedFd)> {
    #[cfg(target_os = "linux")]
    {
        unistd::pipe2(flags)
    }
    #[cfg(not(target_os = "linux"))]
    {
        let supported = fcntl::OFlag::O_CLOEXEC | fcntl::OFlag::O_NONBLOCK;
        if flags.intersects(!supported) {
            return Err(nix::errno::Errno::EINVAL);
        }
        let (read_fd, write_fd) = unistd::pipe()?;
        for fd in [&read_fd, &write_fd] {
            if flags.contains(fcntl::OFlag::O_CLOEXEC) {
                fcntl::fcntl(fd, fcntl::FcntlArg::F_SETFD(fcntl::FdFlag::FD_CLOEXEC))?;
            }
            if flags.contains(fcntl::OFlag::O_NONBLOCK) {
                fcntl::fcntl(fd, fcntl::FcntlArg::F_SETFL(fcntl::OFlag::O_NONBLOCK))?;
            }
        }
        Ok((read_fd, write_fd))
    }
}

/** Create a Unix stream socketpair with portable CLOEXEC/NONBLOCK handling. */
pub fn create_socketpair(flags: fcntl::OFlag) -> nix::Result<(OwnedFd, OwnedFd)> {
    use nix::sys::socket;

    let supported = fcntl::OFlag::O_CLOEXEC | fcntl::OFlag::O_NONBLOCK;
    if flags.intersects(!supported) {
        return Err(nix::errno::Errno::EINVAL);
    }
    #[cfg(target_os = "linux")]
    let socket_flags = {
        let mut socket_flags = socket::SockFlag::empty();
        if flags.contains(fcntl::OFlag::O_CLOEXEC) {
            socket_flags |= socket::SockFlag::SOCK_CLOEXEC;
        }
        if flags.contains(fcntl::OFlag::O_NONBLOCK) {
            socket_flags |= socket::SockFlag::SOCK_NONBLOCK;
        }
        socket_flags
    };
    #[cfg(not(target_os = "linux"))]
    let socket_flags = socket::SockFlag::empty();

    let pair = socket::socketpair(
        socket::AddressFamily::Unix,
        socket::SockType::Stream,
        None,
        socket_flags,
    )?;
    #[cfg(not(target_os = "linux"))]
    for fd in [&pair.0, &pair.1] {
        if flags.contains(fcntl::OFlag::O_CLOEXEC) {
            fcntl::fcntl(fd, fcntl::FcntlArg::F_SETFD(fcntl::FdFlag::FD_CLOEXEC))?;
        }
        if flags.contains(fcntl::OFlag::O_NONBLOCK) {
            fcntl::fcntl(fd, fcntl::FcntlArg::F_SETFL(fcntl::OFlag::O_NONBLOCK))?;
        }
    }
    Ok(pair)
}

/** Use ppoll where available and poll on Darwin, which has no ppoll. */
pub fn system_poll(
    fds: &mut [nix::poll::PollFd<'_>],
    timeout: Option<nix::sys::time::TimeSpec>,
    signal_mask: Option<nix::sys::signal::SigSet>,
) -> nix::Result<libc::c_int> {
    #[cfg(target_os = "linux")]
    {
        nix::poll::ppoll(fds, timeout, signal_mask)
    }
    #[cfg(not(target_os = "linux"))]
    {
        use nix::poll::PollTimeout;
        use nix::sys::time::TimeValLike;

        let _ = signal_mask;
        let timeout = match timeout {
            Some(value) => {
                PollTimeout::try_from(value.num_milliseconds()).unwrap_or(PollTimeout::MAX)
            }
            None => PollTimeout::NONE,
        };
        nix::poll::poll(fds, timeout)
    }
}
