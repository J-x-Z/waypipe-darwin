/* SPDX-License-Identifier: GPL-3.0-or-later */
/*! Wayland protocol message handling and translation logic */
use crate::damage::*;
#[cfg(feature = "dmabuf")]
use crate::dmabuf::*;
#[cfg(feature = "gbmfallback")]
use crate::gbm::*;
use crate::kernel::*;
use crate::platform::*;
#[cfg(any(not(feature = "video"), not(feature = "gbmfallback")))]
use crate::stub::*;
use crate::tag;
use crate::util::*;
use crate::wayland::*;
use crate::wayland_gen::*;
use crate::*;

use core::str;
use log::{debug, error};
use nix::libc;
use nix::unistd;
use std::cell::RefCell;
use std::collections::{BTreeMap, BTreeSet, VecDeque};
use std::fmt::{Display, Formatter};
use std::os::fd::OwnedFd;
use std::rc::Rc;

/** Structure storing information for a specific Wayland object */
pub struct WpObject {
    /** Wayland interface associated with the object */
    obj_type: WaylandInterface,
    /** Extra data associated with the object; in practice this enum either matches
     * WaylandInterface or is WpExtra::None */
    extra: WpExtra,
}

/** Damage rectangle, of the form directly provided by wl_surface.damage and wl_surface.damage_buffer */
#[derive(Clone, Copy, Debug)]
struct WlRect {
    x: i32,
    y: i32,
    width: i32,
    height: i32,
}

/** Properties of a buffer's attachment to the surface, which are required to interpret how
 * surface coordinates and buffer coordinates interact. */
#[derive(Eq, PartialEq, Clone)]
struct BufferAttachment {
    /** Buffer scale */
    scale: u32,
    /** Transform (from buffer to surface) */
    transform: WlOutputTransform,
    /** Viewport source x-offset,y-offset,width,height, all 24.8 fixed point and >=,>=,>,> 0 */
    viewport_src: Option<(i32, i32, i32, i32)>,
    /** Viewport destination width/height, should both be > 0 */
    viewport_dst: Option<(i32, i32)>,
    /** The buffer attached at the time the damage was committed (or an arbitrary value if
     * the batch was not yet committed.) */
    buffer_uid: u64,
    /** Dimensions of the attached buffer; these are needed to properly evaluate transforms. */
    buffer_size: (i32, i32),
}

/** Damage recorded for a surface during a commit interval, plus information
 * needed to interpret the damage and record the associated buffer. */
#[derive(Clone)]
struct DamageBatch {
    /** Information about which buffer is used in this batch and how it is attached to the surface */
    attachment: BufferAttachment,
    /** wl_surface::damage directly applies in the surface coordinate space, and the conversion to buffer
     * coordinate space can only be done at commit time, so these must be cached instead of immediately
     * converted. */
    damage: Vec<WlRect>,
    /** Most clients use wl_surface::damage_buffer, which is easier to use and more precise than wl_surface::damage.
     * For Waypipe, it is usually easier, except in the rare case of accumulating damage over different
     * buffers whose attachment parameters differ (and thus whose buffer coordinate spaces differ); then
     * one must convert between spaces. */
    damage_buffer: Vec<WlRect>,
}

/** Additional information for wl_surface */
struct ObjWlSurface {
    attached_buffer_id: Option<ObjId>,
    /* The total damage for a buffer since the last time it was committed is given
     * by the accumulateed damage committed. The pending state is at index 0,
     * last commit at index 1, etc. */
    damage_history: [DamageBatch; 7],
    /* acquire/release timline points for explicit sync */
    acquire_pt: Option<(u64, Rc<RefCell<ShadowFd>>)>,
    release_pt: Option<(u64, Rc<RefCell<ShadowFd>>)>,

    /* unique wp_viewport object associated with this surface */
    viewport_id: Option<ObjId>,
}

/** Additional information for wp_viewport */
struct ObjWpViewport {
    /** Surface to which this wp_viewport is attached. Is None when wl_surface has been destroyed. */
    wl_surface: Option<ObjId>,
}

/** Additional information for wl_shm_pool */
struct ObjWlShmPool {
    buffer: Rc<RefCell<ShadowFd>>,
}

/** Metadata indicating how a wl_buffer created from a wl_shm pool maps
 * onto the underlying pool */
#[derive(Clone, Copy)]
struct ObjWlBufferShm {
    width: i32,
    height: i32,
    format: u32,
    offset: i32,
    stride: i32,
}

/** Additional information for wl_buffer */
struct ObjWlBuffer {
    sfd: Rc<RefCell<ShadowFd>>,
    /* Metadata explaining how a wl_buffer relates to its underlying wl_shm_pool.'
     * DMABUFs do not need this metadata because the shadowfd stores this information. */
    shm_info: Option<ObjWlBufferShm>,

    unique_id: u64,
}

/** Data of a format tranche from zwp_linux_dmabuf_feedback_v1 */
struct DmabufTranche {
    flags: u32,
    /** Format table entries; these are interpreted _immediately_ when the
     * tranche is provided, using the last format table to arrive. */
    values: Vec<(u32, u64)>,
    /** When translating tranche, cache output indices here */
    indices: Vec<u8>,
    device: u64,
}

/** Additional information for zwp_linux_dmabuf_v1 */
struct ObjZwpLinuxDmabuf {
    /** Set of formats seen in .modifier events. This makes it possible to
     * replace the first .modifier received with a full list of modifiers events
     * for that format, and then drop all subsequent .modifier events with same
     * format. (Alternatively, one could wait for "a roundtrip after binding" to
     * determine when all format events have arrived, but this approach will never
     * introduce any delay. */
    formats_seen: BTreeSet<u32>,
}

/** Additional information for zwp_linux_dmabuf_feedback_v1 */
struct ObjZwpLinuxDmabufFeedback {
    /** Last format table received from the Wayland compositor */
    input_format_table: Option<Vec<(u32, u64)>>,
    /** Contents of last format table sent (or which will be sent to) to the Wayland client */
    output_format_table: Option<Vec<u8>>,

    main_device: Option<u64>,
    tranches: Vec<DmabufTranche>,
    current: DmabufTranche,
    /* If true, have already processed tranches */
    processed: bool,
    /* If true, should send format table when processing ::done */
    queued_format_table: Option<(Rc<RefCell<ShadowFd>>, u32)>,
}

/** Additional information for zwp_linux_buffer_params_v1 */
struct ObjZwpLinuxDmabufParams {
    /* if using 'create', output dmabuf reference stored here before transferring to wl_buffer */
    dmabuf: Option<Rc<RefCell<ShadowFd>>>,
    // A bunch of zwp_linux_buffer_params_v1.add calls will be followed by create or create_immed,
    // which provide the dimensions and format
    planes: Vec<AddDmabufPlane>,
    // todo: the size is limited in practice, so instead of Vec one could use an array;
    // however, this would require some way of handling drop (e.g., using Option<OwnedFd>)
    // which might be awkward
}

/** Additional information for wp_linux_drm_syncobj_surface_v1 */
struct ObjWpDrmSyncobjSurface {
    /* Corresponding wl_surface object */
    surface: ObjId,
}

/** Additional information for wp_linux_drm_syncobj_timeline_v1 */
struct ObjWpDrmSyncobjTimeline {
    timeline: Rc<RefCell<ShadowFd>>,
}

/** Additional information for zwlr_screencopy_frame_v1 */
struct ObjZwlrScreencopyFrame {
    /* Store sfd and shm metadata of target buffer in case the wl_buffer is destroyed early */
    buffer: Option<(Rc<RefCell<ShadowFd>>, Option<ObjWlBufferShm>)>,
}

/** Additional information for ext_image_copy_capture_session_v1.
 *
 * The dmabuf device and formats need to be processed together in order to
 * restrict to what the upstream device and Waypipe both support. shm formats
 * are safe to pass through unmodified since, in the worst case, any shm format
 * can be handled by replicating the entire shm pool.
 */
struct ObjExtImageCopyCaptureSession {
    dmabuf_device: Option<u64>,
    dmabuf_formats: Vec<(u32, Vec<u64>)>,
    /* The sorted list of format/modifier pairs from the last batch of buffer constraint events */
    last_format_mod_list: Vec<(u32, u64)>,
    frame_list: Vec<ObjId>, /* At most one frame _should_ exist at a time */
}

/** Additional information for ext_image_copy_capture_frame_v1 */
struct ObjExtImageCopyCaptureFrame {
    /* Store sfd and shm metadata of target buffer in case the wl_buffer is destroyed early */
    buffer: Option<(Rc<RefCell<ShadowFd>>, Option<ObjWlBufferShm>)>,

    /** This is reset to None when the parent session is destroyed. */
    capture_session: Option<ObjId>,

    /** If the parent session is destroyed, retain the last list of suppported modifiers here. */
    supported_modifiers: Vec<(u32, u64)>,
}

/** Additional information for zwlr_gamma_control_v1 */
struct ObjZwlrGammaControl {
    gamma_size: Option<u32>,
}

/** Additional information for wl_registry */
struct ObjWlRegistry {
    /** Store global advertisements for wp_linux_drm_syncobj_manager_v1
     * until zwp_linux_dmabuf_v1 arrives and it is known whether Waypipe
     * is able to handle DMABUFs and timeline semaphores. */
    syncobj_manager_replay: Vec<(u32, u32)>,
}

/** Additional information attached to specific Wayland objects */
enum WpExtra {
    WlSurface(Box<ObjWlSurface>),
    WlBuffer(Box<ObjWlBuffer>),
    WlRegistry(Box<ObjWlRegistry>),
    WlShmPool(Box<ObjWlShmPool>),
    WpViewport(Box<ObjWpViewport>),
    ZwpDmabuf(Box<ObjZwpLinuxDmabuf>),
    ZwpDmabufFeedback(Box<ObjZwpLinuxDmabufFeedback>),
    ZwpDmabufParams(Box<ObjZwpLinuxDmabufParams>),
    ZwlrScreencopyFrame(Box<ObjZwlrScreencopyFrame>),
    ExtImageCopyCaptureSession(Box<ObjExtImageCopyCaptureSession>),
    ExtImageCopyCaptureFrame(Box<ObjExtImageCopyCaptureFrame>),
    ZwlrGammaControl(Box<ObjZwlrGammaControl>),
    WpDrmSyncobjSurface(Box<ObjWpDrmSyncobjSurface>),
    WpDrmSyncobjTimeline(Box<ObjWpDrmSyncobjTimeline>),
    None,
}

/** This enum indicates in which direction messages are being processed, and
 * provides queues for ShadowFds or file descriptors to receive or send. */
pub enum TranslationInfo<'a> {
    // RID -> SFD/FD
    FromChannel(
        (
            &'a mut VecDeque<Rc<RefCell<ShadowFd>>>,
            &'a mut VecDeque<Rc<RefCell<ShadowFd>>>,
        ),
    ),
    // FD to SFD/RID
    FromWayland(
        (
            &'a mut VecDeque<OwnedFd>,
            &'a mut Vec<Rc<RefCell<ShadowFd>>>,
        ),
    ),
}

/** Given a Wayland rectangle, clip it with the \[0,0,w,h\] rectangle and return
 * the result, if nonempty */
fn clip_wlrect_to_buffer(a: &WlRect, w: i32, h: i32) -> Option<Rect> {
    let x1 = a.x;
    let x2 = a.x.saturating_add(a.width);
    let y1 = a.y;
    let y2 = a.y.saturating_add(a.height);
    let nx1 = x1.clamp(0, w);
    let nx2 = x2.clamp(0, w);
    let ny1 = y1.clamp(0, h);
    let ny2 = y2.clamp(0, h);
    if nx2 > nx1 && ny2 > ny1 {
        Some(Rect {
            x1: nx1 as u32,
            x2: nx2 as u32,
            y1: ny1 as u32,
            y2: ny2 as u32,
        })
    } else {
        None
    }
}

/** Apply the viewport crop and scale to a rectangle in surface-local coordinates. After this,
 * apply buffer scale/transforms.
 *
 * `buffer_size` is the size of the buffer _after_ scale/transform operations.
 *
 * This rounds boundaries outward by up to a pixel, under the assumption that "linear" scaling
 * is used, not nearest-neighbor, cubic, fft, etc.
 *
 * view_src/view_dst should be valid for wp_viewporter
 *
 * This function saturates on overflow.
 */
fn apply_viewport_transform(
    a: &WlRect,
    buffer_size: (i32, i32),
    view_src: Option<(i32, i32, i32, i32)>,
    view_dst: Option<(i32, i32)>,
) -> Option<Rect> {
    assert!(a.width >= 0 && a.height >= 0);

    let dst: (i32, i32) = if let Some(x) = view_dst {
        (x.0, x.1)
    } else if let Some(x) = view_src {
        /* in crop-only case, the crop rectangle size should be an integer */
        ((x.2 as u32).div_ceil(256) as i32, (x.3 as u32).div_ceil(256) as i32)
    } else {
        (buffer_size.0, buffer_size.1)
    };

    /* 1: clip rectangle to 'dst' */
    let x1 = a.x.clamp(0, dst.0);
    let x2 = a.x.saturating_add(a.width).clamp(0, dst.0);
    let y1 = a.y.clamp(0, dst.1);
    let y2 = a.y.saturating_add(a.height).clamp(0, dst.1);
    if x2 <= x1 || y2 <= y1 {
        /* Rectangle intersection with 'dst' region is empty. */
        return None;
    }

    /* Fixed point. Expand to i64 to avoid early clipping. */
    if let Some(v) = view_src {
        assert!(v.0 >= 0 && v.1 >= 0 && v.2 > 0 && v.3 > 0);

        fn source_floor(v: i32, dst: i32, src_sz_fixed: i32, src_offset_fixed: i32) -> u32 {
            /* Fixed point calculation: floor(x1 * src.width / dst.width + src.x1), where src.width, src.x1 are in 24.8 fixed point.
             * Worst case: may use 62 bits for multiplication result and 32 for addition. */
            (((v as u64) * (src_sz_fixed as u64) / (dst as u64) + (src_offset_fixed as u64)) / 256)
                as u32
        }
        fn source_ceil(v: i32, dst: i32, src_sz_fixed: i32, src_offset_fixed: i32) -> u32 {
            ((((v as u64) * (src_sz_fixed as u64)).div_ceil(dst as u64)
                + (src_offset_fixed as u64))
                .div_ceil(256)) as u32
        }

        /* The rectangle should either map into the rectangle given by buffer_transformed_size,
         * or else there should be a protocol error because the viewport src is out of bounds;
         * however, it is safe for Waypipe to just clip the region and not raise an error. */
        let sx1 = source_floor(x1, dst.0, v.2, v.0).min(buffer_size.0 as u32);
        let sx2 = source_ceil(x2, dst.0, v.2, v.0).min(buffer_size.0 as u32);
        let sy1 = source_floor(y1, dst.1, v.3, v.1).min(buffer_size.1 as u32);
        let sy2 = source_ceil(y2, dst.1, v.3, v.1).min(buffer_size.1 as u32);
        if sx1 >= sx2 || sy1 >= sy2 {
            return None;
        }
        Some(Rect {
            x1: sx1,
            x2: sx2,
            y1: sy1,
            y2: sy2,
        })
    } else {
        /* When view_dst=None, buffer_size = dst and this scaling has no effect. */
        let sx1 = (((x1 as u64) * (buffer_size.0 as u64)) / (dst.0 as u64)) as u32;
        let sx2 = (((x2 as u64) * (buffer_size.0 as u64)).div_ceil(dst.0 as u64)) as u32;
        let sy1 = (((y1 as u64) * (buffer_size.1 as u64)) / (dst.1 as u64)) as u32;
        let sy2 = (((y2 as u64) * (buffer_size.1 as u64)).div_ceil(dst.1 as u64)) as u32;
        /* Only clip by transformed size */
        Some(Rect {
            x1: sx1,
            x2: sx2,
            y1: sy1,
            y2: sy2,
        })
    }
}

/** Transform a rectangle indicating damage on a surface to a rectangle indicating
 * damage on a buffer.
 *
 * `scale` and `transform` are those of the surface; `width` and `height` those of the buffer.
 *
 * `view_src`/`view_dst` are validated viewport source and destination parameters.
 *
 * This clips the damage to the surface and returns None if the result is nonempty. */
fn apply_damage_rect_transform(
    a: &WlRect,
    scale: u32,
    transform: WlOutputTransform,
    view_src: Option<(i32, i32, i32, i32)>,
    view_dst: Option<(i32, i32)>,
    width: i32,
    height: i32,
) -> Option<Rect> {
    /* Each of the eight transformations corresponds to a
     * unique set of reflections: X<->Y | Xflip | Yflip */
    let seq = [0b000, 0b011, 0b110, 0b101, 0b010, 0b001, 0b100, 0b111];
    let code = seq[transform as u32 as usize];
    let swap_xy = code & 0x1 != 0;
    let flip_x = code & 0x2 != 0;
    let flip_y = code & 0x4 != 0;

    let pre_vp_size = if swap_xy {
        (height / scale as i32, width / scale as i32)
    } else {
        (width / scale as i32, height / scale as i32)
    };

    let b = apply_viewport_transform(a, pre_vp_size, view_src, view_dst)?;

    // These should not overflow, since b should be clipped to `pre_vp_size`.
    let mut xl = b.x1.checked_mul(scale).unwrap();
    let mut yl = b.y1.checked_mul(scale).unwrap();
    let mut xh = b.x2.checked_mul(scale).unwrap();
    let mut yh = b.y2.checked_mul(scale).unwrap();

    let end_w = if swap_xy { height } else { width } as u32;
    let end_h = if swap_xy { width } else { height } as u32;

    if flip_x {
        (xh, xl) = (end_w - xl, end_w - xh);
    }
    if flip_y {
        (yh, yl) = (end_h - yl, end_h - yh);
    }
    if swap_xy {
        (xl, xh, yl, yh) = (yl, yh, xl, xh);
    }
    Some(Rect {
        x1: xl,
        x2: xh,
        y1: yl,
        y2: yh,
    })
}

/** Invert a viewport transform, rounding damage rectangle sizes up.
 *
 * The input is a nondegenerate rectangle _contained in_ the [0..buffer_size] rectangle,
 * or None if there is no overlap.
 */
fn inverse_viewport_transform(
    a: &Rect,
    buffer_size: (i32, i32),
    view_src: Option<(i32, i32, i32, i32)>,
    view_dst: Option<(i32, i32)>,
) -> Option<WlRect> {
    assert!(buffer_size.0 > 0 && buffer_size.1 > 0);
    assert!(
        a.x1 < a.x2 && a.y1 < a.y2 && a.x2 <= buffer_size.0 as u32 && a.y2 <= buffer_size.1 as u32
    );

    /* 1: convert to .8 fixed point, and clip rectangle to 'src' */
    let (mut x1, mut x2, mut y1, mut y2) = (
        (a.x1 as u64) * 256,
        (a.x2 as u64) * 256,
        (a.y1 as u64) * 256,
        (a.y2 as u64) * 256,
    );
    if let Some((sx, sy, sw, sh)) = view_src {
        assert!(sx >= 0 && sy >= 0 && sw > 0 && sh > 0);
        let e = (sx as u64 + sw as u64, sy as u64 + sh as u64);
        x1 = x1.clamp(sx as u64, e.0) - (sx as u64);
        x2 = x2.clamp(sx as u64, e.0) - (sx as u64);
        y1 = y1.clamp(sy as u64, e.1) - (sy as u64);
        y2 = y2.clamp(sy as u64, e.1) - (sy as u64);

        /* Per protocol, parts of damage outside surface dimensions are ignored */
        if x2 <= x1 || y2 <= y1 {
            return None;
        }
    };
    /* src size, in .8 fixed point */
    let src: (u64, u64) = if let Some(x) = view_src {
        (x.2 as u64, x.3 as u64)
    } else {
        (buffer_size.0 as u64 * 256, buffer_size.1 as u64 * 256)
    };

    /* 2: scale to 'dst' and round result to integer */
    let dst: (u32, u32) = if let Some((dw, dh)) = view_dst {
        assert!(dw > 0 && dh > 0);
        (dw as u32, dh as u32)
    } else if let Some((_sx, _sy, sw, sh)) = view_src {
        /* In crop-only case, the crop rectangle size should be an integer;
         * the error will be logged at commit time, but relax the handling
         * here in case clients are broken and compositors don't catch it */
        ((sw as u32).div_ceil(256), (sh as u32).div_ceil(256))
    } else {
        (buffer_size.0 as u32, buffer_size.1 as u32)
    };

    /* Rectangle coordinates, in fixed point, are up to 31+8 bits,
     * dst is at most 31 bits, and src.0 is at most 31 bits. Do operations
     * in u128 to ensure 31+31+8 bit intermediate products are not clipped.
     * (Note: it is possible to reduce the intermediate product size, since
     * if view_src is not None, coordinates are clipped to 32 bits, and
     * otherwise are divisible by 256.) */
    let xl = ((x1 as u128) * (dst.0 as u128)) / (src.0 as u128);
    let xh = ((x2 as u128) * (dst.0 as u128)).div_ceil(src.0 as u128);
    let yl = ((y1 as u128) * (dst.1 as u128)) / (src.1 as u128);
    let yh = ((y2 as u128) * (dst.1 as u128)).div_ceil(src.1 as u128);
    assert!(xh > xl && yh > yl);
    /* Results should fall within [0,i32::MAX] since dst is so bounded, and coordinates are <= src*256. */
    assert!(xh <= i32::MAX as _ && yh <= i32::MAX as _);
    Some(WlRect {
        x: xl as i32,
        y: yl as i32,
        width: (xh - xl) as i32,
        height: (yh - yl) as i32,
    })
}

/** Transform a rectangle indicating damage on a buffer to a rectangle
 * indicating damage on a surface.
 *
 * The rectangle processed may be clipped to the buffer size and to the viewport size;
 * None will be returned if it has no overlap with the surface extents.
 */
fn inverse_damage_rect_transform(
    a: &WlRect,
    scale: u32,
    transform: WlOutputTransform,
    view_src: Option<(i32, i32, i32, i32)>,
    view_dst: Option<(i32, i32)>,
    buf_width: i32,
    buf_height: i32,
) -> Option<WlRect> {
    assert!(buf_width > 0 && buf_height > 0 && scale > 0);
    assert!(a.width > 0 && a.height > 0);

    /* Each of the eight transformations corresponds to a
     * unique set of reflections: X<->Y | Xflip | Yflip */
    let seq = [0b000, 0b011, 0b110, 0b101, 0b010, 0b001, 0b100, 0b111];
    let code = seq[transform as u32 as usize];
    let swap_xy = code & 0x1 != 0;
    let flip_x = code & 0x2 != 0;
    let flip_y = code & 0x4 != 0;

    let mut xl = a.x.clamp(0, buf_width) as u32;
    let mut xh = a.x.saturating_add(a.width).clamp(0, buf_width) as u32;
    let mut yl = a.y.clamp(0, buf_height) as u32;
    let mut yh = a.y.saturating_add(a.height).clamp(0, buf_height) as u32;
    if xh <= xl || yh <= yl {
        /* Rectangle is degenerate after clipping */
        return None;
    }
    let (end_w, end_h) = if swap_xy {
        (buf_height as u32, buf_width as u32)
    } else {
        (buf_width as u32, buf_height as u32)
    };

    if swap_xy {
        (xl, xh, yl, yh) = (yl, yh, xl, xh);
    }
    if flip_y {
        (yh, yl) = (end_h - yl, end_h - yh);
    }
    if flip_x {
        (xh, xl) = (end_w - xl, end_w - xh);
    }
    (xl, xh) = (xl / scale, xh.div_ceil(scale));
    (yl, yh) = (yl / scale, yh.div_ceil(scale));

    let post_vp_size = if swap_xy {
        (
            (buf_height as u32).div_ceil(scale) as i32,
            (buf_width as u32).div_ceil(scale) as i32,
        )
    } else {
        (
            (buf_width as u32).div_ceil(scale) as i32,
            (buf_height as u32).div_ceil(scale) as i32,
        )
    };

    let b = Rect {
        x1: xl,
        x2: xh,
        y1: yl,
        y2: yh,
    };
    inverse_viewport_transform(&b, post_vp_size, view_src, view_dst)
}

/** Get the memory region corresponding to the buffer with the specific format,
 * or 'None' if there was any overflow. */
fn extent_for_linear_buffer(
    buffer: &ObjWlBufferShm,
    layout: &FormatLayout,
) -> Option<(usize, usize)> {
    let start: usize = buffer.offset.try_into().expect("offset nonnegative");
    let mut end = start;
    assert!(buffer.stride >= 0 && buffer.width > 0 && buffer.height > 0);

    let ext_stride = (buffer.stride as usize).checked_mul(layout.planes[0].hsub.get() as usize)?;
    let mut max_row_len: usize = 0;

    for plane in layout.planes {
        /* In practice, divisions should always be exact/with no remainder;
         * in case odd sizes are for some reason used, the most sensible thing
         * to do is round up. (Exactly how to calculate these parameters is left
         * entirely unspecified by the Wayland protocol as of writing.) */
        let plane_stride = (ext_stride.checked_mul(plane.bpt.get() as usize)?).div_ceil(
            layout.planes[0]
                .bpt
                .get()
                .checked_mul(plane.hsub.get())
                .expect("factors small") as usize,
        );

        let plane_height = (buffer.height as usize).div_ceil(plane.vsub.get() as usize);

        max_row_len = max_row_len.max(
            (buffer.width as usize)
                .checked_mul(plane.bpt.get() as usize)?
                .div_ceil(plane.hsub.get() as usize),
        );

        end = end.checked_add(plane_height.checked_mul(plane_stride)? as usize)?;
    }
    Some((start, end.max(start.checked_add(max_row_len)?)))
}

/** Get an interval containing the entire memory region corresponding to the buffer */
fn damage_for_entire_buffer(buffer: &ObjWlBufferShm) -> (usize, usize) {
    let max_endpoint = 64 * (usize::MAX / 64);
    if let Some(layout) = get_shm_format_layout(buffer.format) {
        if let Some((start, end)) = extent_for_linear_buffer(buffer, &layout) {
            (64 * (start / 64), align(end.min(max_endpoint), 64))
        } else {
            (64 * (buffer.offset as usize / 64), max_endpoint)
        }
    } else {
        debug!(
            "Likely overestimating buffer size for unknown format {}",
            buffer.format
        );
        (64 * (buffer.offset as usize / 64), max_endpoint)
    }
}

/** Return a list of rectangular areas damaged on a surface since the last time the given
 * buffer was committed; the rectangles are not guaranteed to be disjoint. */
fn get_damage_rects(surface: &ObjWlSurface, attachment: &BufferAttachment) -> Vec<Rect> {
    let (width, height) = attachment.buffer_size;
    let mut rects = Vec::<Rect>::new();
    assert!(width > 0 && height > 0);

    let full_damage = Rect {
        x1: 0,
        x2: width.try_into().unwrap(),
        y1: 0,
        y2: height.try_into().unwrap(),
    };

    /* The first slot will later be updated to match `attachment`. */
    let Some(first_idx_offset) = surface
        .damage_history
        .iter()
        .skip(1)
        .position(|x| x.attachment.buffer_uid == attachment.buffer_uid)
    else {
        /* First time this buffer was seen at all; mark the entire buffer as damaged */
        rects.push(full_damage);
        return rects;
    };
    let first_idx = first_idx_offset + 1;
    if surface.damage_history[first_idx].attachment != *attachment {
        /* Require an exact match: it is posssible that the surface->buffer coordinate
         * transform and the buffer contents have changed in concert so that the surface
         * pixels remain the same, even though all buffer pixels are different. In this
         * scenario no damage will be reported. However, Waypipe must still update all
         * pixels, because the compositor may read from and rerender the surface at any
         * time. It is only when the visible surface contents have no damage and the
         * coordinate transform stays the same that the contents of the buffer are pinned.
         *
         * Note: in theory, one could special-case viewport-resize operations which preserve
         * the mapping for all retained surface pixels, and then automatically damage the
         * buffer pixels that are newly made visible. (Clients may do this to enable
         * high-performance smooth resizing.)
         */
        rects.push(full_damage);
        return rects;
    }

    /* Scan all recorded damage from previous commits */
    for (i, batch) in surface.damage_history[..first_idx].iter().enumerate() {
        if i == 0 {
            /* Special case: damage to the target buffer needs no conversion */
            for w in &batch.damage_buffer {
                if let Some(r) = clip_wlrect_to_buffer(w, width, height) {
                    rects.push(r);
                }
            }
        } else {
            for w in &batch.damage_buffer {
                /* Translate damage from this buffer's coordinate space to the current
                 * buffer's coordinate space, through the surface coordinate space.
                 *
                 * Note: buffer damage can be more precise than surface damage when the
                 * scale is > 1 or wp_viewporter is involved, so this can round up buffer
                 * sizes more than strictly necessary. */
                let Some(s) = inverse_damage_rect_transform(
                    w,
                    batch.attachment.scale,
                    batch.attachment.transform,
                    batch.attachment.viewport_src,
                    batch.attachment.viewport_dst,
                    batch.attachment.buffer_size.0,
                    batch.attachment.buffer_size.1,
                ) else {
                    continue;
                };
                if let Some(r) = apply_damage_rect_transform(
                    &s,
                    attachment.scale,
                    attachment.transform,
                    attachment.viewport_src,
                    attachment.viewport_dst,
                    width,
                    height,
                ) {
                    rects.push(r);
                }
            }
        }

        for w in &batch.damage {
            /* Convert from surface space to local space, using the _current_ transform.
             *
             * Note: the damage is clipped according to the current surface size;
             * technically one could clip damage at each commit according to the then-current
             * surface size, and then reclip it for this one; however, such an optimization
             * would be complicated and only benefits weird clients, so is not worth doing. */
            if let Some(r) = apply_damage_rect_transform(
                w,
                attachment.scale,
                attachment.transform,
                attachment.viewport_src,
                attachment.viewport_dst,
                width,
                height,
            ) {
                rects.push(r);
            }
        }
    }
    rects
}

/** Compute the (disjoint) damage intervals for a shared memory pool used by the buffer,
 * using damage accumulated since the last time the buffer was committed */
fn get_damage_for_shm(
    buffer: &ObjWlBuffer,
    surface: &ObjWlSurface,
    attachment: &BufferAttachment,
) -> Vec<(usize, usize)> {
    let Some(shm_info) = &buffer.shm_info else {
        panic!();
    };

    let Some(layout) = get_shm_format_layout(shm_info.format) else {
        debug!("Format without known linear layout {}", shm_info.format);
        return vec![damage_for_entire_buffer(shm_info)];
    };
    let [p0] = layout.planes else {
        debug!(
            "Format {} has {} planes",
            shm_info.format,
            layout.planes.len()
        );
        return vec![damage_for_entire_buffer(shm_info)];
    };
    if p0.hsub.get() != 1 || p0.vsub.get() != 1 || p0.htex.get() != 1 || p0.vtex.get() != 1 {
        debug!(
            "Format {} has nontrivial texels or subsampling",
            shm_info.format
        );
        return vec![damage_for_entire_buffer(shm_info)];
    }
    let bpp = p0.bpt.get();

    let mut rects = get_damage_rects(surface, attachment);
    compute_damaged_segments(
        &mut rects[..],
        6,
        128,
        shm_info.offset.try_into().unwrap(),
        shm_info.stride.try_into().unwrap(),
        bpp as usize,
    )
}

/** Compute the (disjoint) damage intervals for the linear view of a DMABUF's contents,
 * using damage accumulated since the last time the buffer was committed */
fn get_damage_for_dmabuf(
    sfdd: &ShadowFdDmabuf,
    surface: &ObjWlSurface,
    attachment: &BufferAttachment,
) -> Vec<(usize, usize)> {
    // TODO: deduplicate implementations
    let (nom_len, width) = match sfdd.buf {
        DmabufImpl::Vulkan(ref buf) => (buf.nominal_size(sfdd.view_row_stride), buf.width),
        DmabufImpl::Gbm(ref buf) => (buf.nominal_size(sfdd.view_row_stride), buf.width),
    };

    assert!(
        sfdd.drm_format != WlShmFormat::Xrgb8888 as u32
            && sfdd.drm_format != WlShmFormat::Argb8888 as u32
    );
    let wayl_format = drm_to_wayland(sfdd.drm_format);
    let Some(layout) = get_shm_format_layout(wayl_format) else {
        debug!("Format without known bpp {}", sfdd.drm_format);
        return vec![(0, align(nom_len, 64))];
    };
    let [p0] = layout.planes else {
        debug!(
            "Format {} has {} planes",
            sfdd.drm_format,
            layout.planes.len()
        );
        return vec![(0, align(nom_len, 64))];
    };
    if p0.hsub.get() != 1 || p0.vsub.get() != 1 || p0.htex.get() != 1 || p0.vtex.get() != 1 {
        debug!(
            "Format {} has nontrivial texels or subsampling",
            sfdd.drm_format
        );
        return vec![(0, align(nom_len, 64))];
    }
    let bpp = p0.bpt.get();

    let mut rects = get_damage_rects(surface, attachment);
    /* Stride: tightly packed. */
    // except: possibly gcd(4,bpp) aligned ?
    let stride = sfdd.view_row_stride.unwrap_or(width * bpp);
    compute_damaged_segments(&mut rects[..], 6, 128, 0, stride as usize, bpp as usize)
}

/** Construct a format table for use by the zwp_linux_dmabuf_v1 protocol,
 * from the tranches */
fn process_dmabuf_feedback(feedback: &mut ObjZwpLinuxDmabufFeedback) -> Result<Vec<u8>, String> {
    let mut index: BTreeMap<(u32, u64), u16> = BTreeMap::new();
    for t in feedback.tranches.iter() {
        for f in t.values.iter() {
            index.insert(*f, u16::MAX);
        }
    }

    if index.len() > u16::MAX as usize {
        return Err(tag!(
            "Format table is too large ({} > {})",
            index.len(),
            u16::MAX
        ));
    }

    let mut table = Vec::new();
    for (i, (f, v)) in index.iter_mut().enumerate() {
        table.extend_from_slice(&f.0.to_le_bytes());
        table.extend_from_slice(&0u32.to_le_bytes());
        table.extend_from_slice(&f.1.to_le_bytes());
        *v = i as u16;
    }

    for t in feedback.tranches.iter_mut() {
        let mut indices = Vec::new();
        for f in t.values.iter() {
            let idx = index.get(f).expect("Inserted key should still be present");
            indices.extend_from_slice(&idx.to_le_bytes());
        }
        t.indices = indices;
    }

    Ok(table)
}

/** Construct a format table for use by the zwp_linux_dmabuf_v1 protocol */
fn rebuild_format_table(
    dmabuf_dev: &DmabufDevice,
    feedback: &mut ObjZwpLinuxDmabufFeedback,
) -> Result<(), String> {
    /* Identify the remotely supported formats */
    let mut remote_formats = BTreeSet::<u32>::new();
    for t in feedback.tranches.iter() {
        if t.device != feedback.main_device.unwrap() {
            /* Only use main device of compositor; at least one tranche will use it. */
            continue;
        }
        for (fmt, _modifier) in t.values.iter() {
            remote_formats.insert(*fmt);
        }
    }

    /* Regenerate tranches, roughly matching original _format_ preferences */
    let mut new_tranches = Vec::<DmabufTranche>::new();
    for t in feedback.tranches.iter() {
        if t.device != feedback.main_device.unwrap() {
            continue;
        }

        let mut n = DmabufTranche {
            device: feedback.main_device.unwrap(),
            flags: 0,
            values: Vec::new(),
            indices: Vec::new(),
        };

        for (fmt, _modifier) in t.values.iter() {
            /* Record each format in exactly one tranche, preferring earlier tranches;
             * all modifiers for a format are put in the same tranche. */
            if remote_formats.remove(fmt) {
                let mods = dmabuf_dev_modifier_list(dmabuf_dev, *fmt);
                for m in mods {
                    n.values.push((*fmt, *m));
                }
            }
        }
        if !n.values.is_empty() {
            new_tranches.push(n);
        }
    }
    if new_tranches.is_empty() {
        return Err(tag!(
            "Failed to build new format tranches: no formats with common support"
        ));
    }
    feedback.tranches = new_tranches;

    Ok(())
}

/** Add new modifiers to the format-to-modifier-list table.
 *
 * Modifiers added are not deduplicated; if `modifiers` is empty nothing will be done.
 *
 * This can theoretically lead to quadratic runtime, but should only be used with
 * modifiers that are a subset of what Waypipe supports, so list lengths will
 * never be long.
 */
fn add_advertised_modifiers(map: &mut BTreeMap<u32, Vec<u64>>, format: u32, modifiers: &[u64]) {
    if modifiers.is_empty() {
        return;
    }
    let entries: &mut Vec<u64> = map.entry(format).or_default();
    for m in modifiers {
        if !entries.contains(m) {
            entries.push(*m);
        }
    }
}

/** Record a new Wayland object, checking that its ID was not already used */
fn insert_new_object(
    objects: &mut BTreeMap<ObjId, WpObject>,
    id: ObjId,
    obj: WpObject,
) -> Result<(), String> {
    let t = obj.obj_type;
    if let Some(old) = objects.insert(id, obj) {
        return Err(tag!(
            "Creating object of type {:?} with id {}, but object of type {:?} with same id already exists",
            t, id, old.obj_type,
        ));
    }
    Ok(())
}

/** Register any new objects created by a message */
fn register_generic_new_ids(
    msg: &[u8],
    meth: &WaylandMethod,
    glob: &mut Globals,
) -> Result<(), String> {
    let mut tail = &msg[8..];

    for op in meth.sig {
        match op {
            WaylandArgument::Uint | WaylandArgument::Int | WaylandArgument::Fixed => {
                let _ = parse_u32(&mut tail);
            }
            WaylandArgument::Fd => {
                // do nothing
            }
            WaylandArgument::Object(_) | WaylandArgument::GenericObject => {
                let _ = parse_obj(&mut tail)?;
            }
            WaylandArgument::NewId(new_intf) => {
                let id = parse_obj(&mut tail)?;

                insert_new_object(
                    &mut glob.objects,
                    id,
                    WpObject {
                        obj_type: *new_intf,
                        extra: WpExtra::None,
                    },
                )?;
            }
            WaylandArgument::GenericNewId => {
                // order: (string, version, new_id)
                let string = parse_string(&mut tail)?
                    .ok_or_else(|| tag!("New id string should not be null"))?;
                let _version = parse_u32(&mut tail)?;
                let id = parse_obj(&mut tail)?;

                if glob.objects.contains_key(&id) {
                    return Err(tag!("Creating object with id not detected as deleted"));
                }

                if let Some(new_intf) = lookup_intf_by_name(string) {
                    /* Only track recognized object types; messages for
                     * unrecognized types will be ignored. */
                    glob.objects.insert(
                        id,
                        WpObject {
                            obj_type: new_intf,
                            extra: WpExtra::None,
                        },
                    );
                }
            }
            WaylandArgument::String => {
                let x = parse_string(&mut tail)?;
                if x.is_none() {
                    return Err(tag!("Received null string where none allowed"));
                }
            }
            WaylandArgument::OptionalString => {
                let _ = parse_string(&mut tail)?;
            }
            WaylandArgument::Array => {
                let _ = parse_array(&mut tail)?;
            }
        }
    }
    Ok(())
}

/** Copy a message to the destination buffer */
fn copy_msg(msg: &[u8], dst: &mut &mut [u8]) {
    dst[..msg.len()].copy_from_slice(msg);
    *dst = &mut std::mem::take(dst)[msg.len()..];
}
/** Copy a message, stripping or adding the Waypipe-specific fd count field */
fn copy_msg_tag_fd(msg: &[u8], dst: &mut &mut [u8], from_channel: bool) -> Result<(), String> {
    dst[..msg.len()].copy_from_slice(msg);
    let mut h2 = u32::from_le_bytes(dst[4..8].try_into().unwrap());

    if from_channel {
        // drop the upper bits of the opcode entirely
        if h2 & (1 << 11) == 0 {
            return Err(tag!("header part {:x} missing fd tag", h2));
        }
        h2 &= !0xff00;
    } else {
        // tag message as using one file descriptor
        h2 = (h2 & !0xff00) | (1 << 11);
    }
    dst[4..8].copy_from_slice(&u32::to_le_bytes(h2));

    *dst = &mut std::mem::take(dst)[msg.len()..];
    Ok(())
}

/** Return true iff `id` in the Wayland server allocation range */
fn is_server_object(id: ObjId) -> bool {
    id.0 >= 0xff000000
}

/** Default processing for an identified Wayland message */
fn default_proc_way_msg(
    msg: &[u8],
    dst: &mut &mut [u8],
    meth: &WaylandMethod,
    is_req: bool,
    object_id: ObjId,
    glob: &mut Globals,
) -> Result<ProcMsg, String> {
    if dst.len() < msg.len() {
        // Not enough space to store the message. Then edit the
        // protocol message in place, and stop processing.
        return Ok(ProcMsg::NeedsSpace((msg.len(), 0)));
    }

    register_generic_new_ids(msg, meth, glob)?;

    if meth.destructor {
        if !is_req {
            // Deletion events; process immediately; object is already in map
            glob.objects.remove(&object_id).unwrap();
        } else if is_server_object(object_id) {
            // TODO: is zombie handling necessary, when clients delete server generated objects?
            glob.objects.remove(&object_id).unwrap();
        } else {
            // client object, destructor request: wait until: wl_display.delete_id
        }
    }
    copy_msg(msg, dst);
    Ok(ProcMsg::Done)
}

/** Process an unidentified Wayland message */
fn proc_unknown_way_msg(
    msg: &[u8],
    dst: &mut &mut [u8],
    transl: TranslationInfo,
) -> Result<ProcMsg, String> {
    /* Untracked, unidentified object with unknown message */
    if let TranslationInfo::FromChannel((x, y)) = transl {
        let mut header2 = u32::from_le_bytes(msg[4..8].try_into().unwrap());
        let ntagfds = ((header2 & ((1 << 16) - 1)) >> 11) as usize;
        // This is usually safe for simple file transfers (like keymaps),
        // where as long as the sending side recognizes the file it can
        // be replicated OK.
        if ntagfds > 0 {
            error!("Unidentified message has {} fds attached according to other Waypipe instance; blindly transferring them", ntagfds);
        }

        if dst.len() < msg.len() || y.len() + ntagfds > MAX_OUTGOING_FDS {
            // Not enough space to store the message.
            return Ok(ProcMsg::NeedsSpace((msg.len(), ntagfds)));
        }

        if x.len() < ntagfds {
            return Err(tag!("Missing sfd"));
        }
        for sfd in x.iter().take(ntagfds) {
            let b = sfd.borrow();
            if let ShadowFdVariant::File(data) = &b.data {
                if data.pending_apply_tasks > 0 {
                    return Ok(ProcMsg::WaitFor(b.remote_id));
                }
            }
        }

        for _ in 0..ntagfds {
            let sfd = x.pop_front().unwrap();
            y.push_back(sfd);
        }

        /* Copy message to wayland, removing fd tags */
        dst[..msg.len()].copy_from_slice(msg);
        header2 &= !0xf800;
        dst[4..8].copy_from_slice(&u32::to_le_bytes(header2));
    } else {
        if dst.len() < msg.len() {
            // Not enough space to store the message.
            return Ok(ProcMsg::NeedsSpace((msg.len(), 0)));
        }
        /* Copy message to channel, assuming no fds are attached */
        dst[..msg.len()].copy_from_slice(msg);
    }

    *dst = &mut std::mem::take(dst)[msg.len()..];
    Ok(ProcMsg::Done)
}

/** Parse and return the entries of a zwp_linux_dmabuf_v1 format table. Any
 * trailing bytes after the last multiple of 16 will be ignored */
fn parse_format_table(data: &[u8]) -> Vec<(u32, u64)> {
    let mut t = Vec::new();
    for chunk in data.chunks_exact(16) {
        let format: u32 = u32::from_le_bytes(chunk[..4].try_into().unwrap());
        let modifier: u64 = u64::from_le_bytes(chunk[8..16].try_into().unwrap());
        t.push((format, modifier));
    }
    t
}

/** Parse a `dev_t` array into a u64 */
fn parse_dev_array(arr: &[u8]) -> Option<u64> {
    /* dev_t may be smaller on some old systems, but detection of this is complicated, so
     * accept both reasonable options */
    if arr.len() == 4 {
        Some(u32::from_le_bytes(arr.try_into().unwrap()) as u64)
    } else if arr.len() == 8 {
        Some(u64::from_le_bytes(arr.try_into().unwrap()))
    } else {
        None
    }
}

/** Convert a `u64` into an array of size `dev_t`. On systems where dev_t
 * is u32, the high bits are expected to be zero. */
fn write_dev_array(dev: u64) -> [u8; SIZEOF_DEV_T] {
    let b = dev.to_le_bytes();
    let (dev, leftover) = b.split_at(SIZEOF_DEV_T);
    assert!(leftover.iter().all(|x| *x == 0));
    dev.try_into().unwrap()
}

/** Assuming the ShadowFd has file type, return whether has pending apply tasks? */
fn file_has_pending_apply_tasks(sfd: &RefCell<ShadowFd>) -> Result<bool, String> {
    let b = sfd.borrow();
    let ShadowFdVariant::File(data) = &b.data else {
        // TODO: make this a helper function
        return Err(tag!("ShadowFd is not of file type"));
    };
    Ok(data.pending_apply_tasks > 0)
}

/** Compute the midpoint of two timestamps, rounding to -∞ */
fn timespec_midpoint(stamp_1: libc::timespec, stamp_3: libc::timespec) -> libc::timespec {
    let mut mid_nsec = if stamp_1.tv_nsec < stamp_3.tv_nsec {
        stamp_1.tv_nsec + (stamp_3.tv_nsec - stamp_1.tv_nsec) / 2
    } else {
        stamp_3.tv_nsec + (stamp_1.tv_nsec - stamp_3.tv_nsec) / 2
    };
    let mut mid_sec = if stamp_1.tv_sec < stamp_3.tv_sec {
        stamp_1.tv_sec + (stamp_3.tv_sec - stamp_1.tv_sec) / 2
    } else {
        stamp_3.tv_sec + (stamp_1.tv_sec - stamp_3.tv_sec) / 2
    };
    if stamp_3.tv_sec % 2 != stamp_1.tv_sec % 2 {
        mid_nsec += 500_000_000;
    }
    if mid_nsec > 1_000_000_000 {
        mid_sec += 1;
        mid_nsec -= 1_000_000_000;
    }

    timespec_from_sec_nsec(mid_sec, mid_nsec)
}

/** Estimate the time difference between two clocks.
 *
 * Return value is: (sec_diff, nsec_diff) where nsec_diff >= 0 */
fn clock_sub(clock_a: u32, clock_b: u32) -> Result<(i64, u32), String> {
    let mut stamp_1 = timespec_from_sec_nsec(0, 0);
    let mut stamp_2 = stamp_1;
    let mut stamp_3 = stamp_1;
    let ca: libc::clockid_t = clock_a.try_into().unwrap();
    let cb: libc::clockid_t = clock_b.try_into().unwrap();
    unsafe {
        // SAFETY: clock_gettime only writes into second argument, which is repr(C)
        // and thus properly aligned
        let ret1 = libc::clock_gettime(ca, &mut stamp_1);
        let ret2 = libc::clock_gettime(cb, &mut stamp_2);
        let ret3 = libc::clock_gettime(ca, &mut stamp_3);
        if ret1 != 0 || ret2 != 0 || ret3 != 0 {
            return Err(tag!(
                "clock_gettime failed for clock {} or {}",
                clock_a,
                clock_b
            ));
        }
    }

    let stamp_avg = timespec_midpoint(stamp_1, stamp_3);
    /* tv_sec is i32 on pre-Y2K38 systems */
    #[allow(clippy::unnecessary_cast)]
    let mut tv_sec = stamp_avg
        .tv_sec
        .checked_sub(stamp_2.tv_sec)
        .ok_or_else(|| tag!("overflow"))? as i64;
    let mut tv_nsec = stamp_avg
        .tv_nsec
        .checked_sub(stamp_2.tv_nsec)
        .ok_or_else(|| tag!("overflow"))? as i32;

    if tv_nsec < 0 {
        tv_sec -= 1;
        tv_nsec += 1_000_000_000;
    }
    assert!((0..1_000_000_000).contains(&tv_nsec));

    Ok((tv_sec, tv_nsec as u32))
}

/** Add a signed time offset to a time point, returning None on overflow. */
fn time_add(mut a: (u64, u32), b: (i64, u32)) -> Option<(u64, u32)> {
    assert!(a.1 < 1_000_000_000 && b.1 < 1_000_000_000);
    let mut nsec = a.1.checked_add(b.1)?;
    if nsec > 1_000_000_000 {
        nsec -= 1_000_000_000;
        a.0 = a.0.checked_add(1)?;
    }

    // 64-bit time overflow should never happen in practice
    Some((a.0.checked_add_signed(b.0)?, nsec))
}

/** Convert the given timestamp from/to the specified clock ID to/from the realtime
 * clock, depending on whether message is going toward the channel or not.  */
fn translate_timestamp(
    tv_sec_hi: u32,
    tv_sec_lo: u32,
    tv_nsec: u32,
    clock_id: u32,
    to_channel: bool,
) -> Result<(u32, u32, u32), String> {
    if tv_nsec >= 1_000_000_000 {
        return Err(tag!("tv_nsec should be in [0,10^9), got {}", tv_nsec));
    }

    let tv_sec = join_u64(tv_sec_hi, tv_sec_lo);
    let realtime = libc::CLOCK_REALTIME as u32;
    let (new_sec, new_nsec) = if to_channel {
        /* Convert from clock_id to CLOCK_REALTIME */
        let (diff_sec, diff_nsec) = clock_sub(realtime, clock_id)?;
        time_add((tv_sec, tv_nsec), (diff_sec, diff_nsec)).ok_or_else(|| tag!("overflow"))?
    } else {
        /* Convert from CLOCK_REALTIME to clock_id */
        let (diff_sec, diff_nsec) = clock_sub(clock_id, realtime)?;
        time_add((tv_sec, tv_nsec), (diff_sec, diff_nsec)).ok_or_else(|| tag!("overflow"))?
    };
    let (new_sec_hi, new_sec_lo) = split_u64(new_sec);
    Ok((new_sec_hi, new_sec_lo, new_nsec))
}

/** Handle a readonly file whose contents need exact replication.
 *
 * When the message comes from Wayland, translate it; when the message comes from the
 * channel, return Ok(Some(ProcMsg::WaitFor(...))) if the file is not yet ready to export.
 */
fn translate_or_wait_for_fixed_file(
    transl: TranslationInfo,
    glob: &mut Globals,
    file_sz: u32,
) -> Result<Option<ProcMsg>, String> {
    match transl {
        TranslationInfo::FromChannel((x, y)) => {
            let sfd = &x.front().ok_or_else(|| tag!("Missing fd"))?;
            let rid = sfd.borrow().remote_id;
            if file_has_pending_apply_tasks(sfd)? {
                return Ok(Some(ProcMsg::WaitFor(rid)));
            }
            y.push_back(x.pop_front().unwrap());
        }
        TranslationInfo::FromWayland((x, y)) => {
            let v = translate_shm_fd(
                x.pop_front().ok_or_else(|| tag!("Missing fd"))?,
                file_sz.try_into().unwrap(),
                &mut glob.map,
                &mut glob.max_local_id,
                true,
                true,
                false,
            )?;
            y.push(v);
        }
    };
    Ok(None)
}

/** A helper structure to print a Wayland method's arguments, on demand */
struct MethodArguments<'a> {
    meth: &'a WaylandMethod,
    msg: &'a [u8],
}
/** Display a Wayland method's arguments; error if parsing failed, return true if formatter error */
fn fmt_method(arg: &MethodArguments, f: &mut Formatter<'_>) -> Result<bool, &'static str> {
    assert!(arg.msg.len() >= 8);
    let mut tail: &[u8] = &arg.msg[8..];

    let mut first = true;
    for op in arg.meth.sig {
        if !first {
            if write!(f, ", ").is_err() {
                return Ok(true);
            }
        } else {
            first = false;
        }
        match op {
            WaylandArgument::Uint => {
                let v = parse_u32(&mut tail)?;
                if write!(f, "{}:u", v).is_err() {
                    return Ok(true);
                }
            }
            WaylandArgument::Int => {
                let v = parse_i32(&mut tail)?;
                if write!(f, "{}:i", v).is_err() {
                    return Ok(true);
                }
            }
            WaylandArgument::Fixed => {
                let v = parse_i32(&mut tail)?;
                if write!(f, "{:.8}:f", (v as f64) * 0.00390625).is_err() {
                    return Ok(true);
                }
            }
            WaylandArgument::Fd => {
                // do nothing
                if write!(f, "fd").is_err() {
                    return Ok(true);
                }
            }
            WaylandArgument::Object(t) => {
                let id = parse_u32(&mut tail)?;
                if write!(f, "{}#{}:obj", INTERFACE_TABLE[*t as usize].name, id).is_err() {
                    return Ok(true);
                }
            }
            WaylandArgument::NewId(t) => {
                let id = parse_u32(&mut tail)?;
                if write!(f, "{}#{}:new_id", INTERFACE_TABLE[*t as usize].name, id).is_err() {
                    return Ok(true);
                }
            }
            WaylandArgument::GenericObject => {
                let id = parse_u32(&mut tail)?;
                if write!(f, "{}:gobj", id).is_err() {
                    return Ok(true);
                }
            }
            WaylandArgument::GenericNewId => {
                // order: (string, version, new_id)
                let ostring = parse_string(&mut tail)?;
                let version = parse_u32(&mut tail)?;
                let id = parse_u32(&mut tail)?;
                if (if let Some(string) = ostring {
                    write!(
                        f,
                        "(\"{}\", {}:u, {}:new_id)",
                        EscapeAsciiPrintable(string),
                        version,
                        id
                    )
                } else {
                    write!(f, "(null_str, {}:u, {}:new_id)", version, id)
                })
                .is_err()
                {
                    return Ok(true);
                }
            }
            WaylandArgument::String | WaylandArgument::OptionalString => {
                let ostring = parse_string(&mut tail)?;
                if (if let Some(string) = ostring {
                    write!(f, "\"{}\"", EscapeAsciiPrintable(string))
                } else {
                    write!(f, "null_str")
                })
                .is_err()
                {
                    return Ok(true);
                }
            }
            WaylandArgument::Array => {
                let a = parse_array(&mut tail)?;
                if write!(f, "{:?}", a).is_err() {
                    return Ok(true);
                }
            }
        }
    }
    if !tail.is_empty() {
        return Err(PARSE_ERROR);
    }
    Ok(false)
}

impl Display for MethodArguments<'_> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match fmt_method(self, f) {
            Err(e) => write!(f, "...format error: {}", e),
            Ok(eof) => {
                if eof {
                    Err(std::fmt::Error)
                } else {
                    Ok(())
                }
            }
        }
    }
}

/** Result of a message processing attempt */
#[derive(Debug, Eq, PartialEq)]
pub enum ProcMsg {
    /** Message successfully processed */
    Done,
    /* Not enough bytes or fds in the output queue to process the message.
     *
     * The argument is the (number of bytes, number of fds) needed */
    NeedsSpace((usize, usize)),
    /** Need to wait for all operations on a given RID to complete.
     *
     * This is only useful for messages coming from the channel whose associated
     * processing might still be in progress. */
    WaitFor(Rid),
}

/** Returns true iff `x` is <= `y` in both coordinates */
fn space_le(x: (usize, usize), y: (usize, usize)) -> bool {
    x.0 <= y.0 && x.1 <= y.1
}

/** Macro to check if the required space (`x` bytes, `y` fds) is more than
 * the available space (`r.0` and `r.1`) and if so, return the required space. */
macro_rules! check_space {
    ($x:expr, $y:expr, $r:expr) => {
        let space: (usize, usize) = ($x, $y);
        if !space_le(space, $r) {
            return Ok(ProcMsg::NeedsSpace(space));
        }
    };
}

/** Process a Wayland message; typically this just copies the message from the source
 * to the destination buffer, but sometimes messages are dropped and inserted, and file
 * descriptor processing is done or queued.
 *
 * The function returns ProcMsg depending on whether the message was processed or
 * if some condition must be met first. */
pub fn process_way_msg(
    msg: &[u8],
    dst: &mut &mut [u8],
    transl: TranslationInfo,
    glob: &mut Globals,
) -> Result<ProcMsg, String> {
    let object_id = ObjId(u32::from_le_bytes(msg[0..4].try_into().unwrap()));
    let header2 = u32::from_le_bytes(msg[4..8].try_into().unwrap());
    let length = (header2 >> 16) as usize;
    assert!(msg.len() == length);
    // drop bits 11-16 from the opcode, as these may encode fds
    let opcode = (header2 & ((1 << 11) - 1)) as usize;

    let (from_channel, outgoing_fds): (bool, usize) = match &transl {
        TranslationInfo::FromChannel((_x, y)) => (true, y.len()),
        TranslationInfo::FromWayland((_x, y)) => (false, y.len()),
    };

    let is_req = glob.on_display_side == from_channel;
    let Some(ref mut obj) = glob.objects.get_mut(&object_id) else {
        debug!(
            "Processing {} on unknown object {}; opcode {} length {}",
            if is_req { "request" } else { "event" },
            object_id,
            opcode,
            length
        );
        return proc_unknown_way_msg(msg, dst, transl);
    };

    let opt_meth: Option<&WaylandMethod> = if is_req {
        INTERFACE_TABLE[obj.obj_type as usize].reqs.get(opcode)
    } else {
        INTERFACE_TABLE[obj.obj_type as usize].evts.get(opcode)
    };
    if opt_meth.is_none() {
        debug!(
            "Method out of range: {}#{}, opcode {}",
            INTERFACE_TABLE[obj.obj_type as usize].name, object_id, opcode
        );
        return proc_unknown_way_msg(msg, dst, transl);
    }

    let meth = opt_meth.unwrap();
    /* note: this may fail */
    if log::log_enabled!(log::Level::Debug) {
        debug!(
            "Processing {}: {}#{}.{}({})",
            if is_req { "request" } else { "event" },
            INTERFACE_TABLE[obj.obj_type as usize].name,
            object_id,
            meth.name,
            MethodArguments { meth, msg }
        );
    }

    assert!(opcode <= u8::MAX as usize);
    let mod_opcode = if is_req {
        MethodId::Request(opcode as u8)
    } else {
        MethodId::Event(opcode as u8)
    };

    let remaining_space = (dst.len(), MAX_OUTGOING_FDS - outgoing_fds);

    match (obj.obj_type, mod_opcode) {
        (WaylandInterface::WlDisplay, OPCODE_WL_DISPLAY_DELETE_ID) => {
            check_space!(msg.len(), 0, remaining_space);

            let object_id = ObjId(parse_evt_wl_display_delete_id(msg)?);
            if object_id == ObjId(1) {
                return Err(tag!("Tried to delete wl_display object"));
            }

            if let Some(_removed) = glob.objects.remove(&object_id) {
                /* Cleanup .extra state: currently nothing to do */
            } else {
                debug!("Deleted untracked object");
            }

            copy_msg(msg, dst);

            Ok(ProcMsg::Done)
        }
        (WaylandInterface::WlDisplay, OPCODE_WL_DISPLAY_GET_REGISTRY) => {
            check_space!(msg.len(), 0, remaining_space);

            let registry_id = parse_req_wl_display_get_registry(msg)?;
            insert_new_object(
                &mut glob.objects,
                registry_id,
                WpObject {
                    obj_type: WaylandInterface::WlRegistry,
                    extra: WpExtra::WlRegistry(Box::new(ObjWlRegistry {
                        syncobj_manager_replay: Vec::new(),
                    })),
                },
            )?;
            copy_msg(msg, dst);
            Ok(ProcMsg::Done)
        }
        (WaylandInterface::WlCallback, OPCODE_WL_CALLBACK_DONE) => {
            check_space!(msg.len(), 0, remaining_space);
            copy_msg(msg, dst);
            /* This object has a destructor event, and no methods, so
             * it is considered deleted immediately on receipt of the message */
            glob.objects.remove(&object_id);

            // TODO: handle all 'destructor-events' like this, including those for compositor-created objects?
            Ok(ProcMsg::Done)
        }
        (WaylandInterface::WlShm, OPCODE_WL_SHM_CREATE_POOL) => {
            check_space!(msg.len(), 1, remaining_space);

            let (pool_id, pool_size) = parse_req_wl_shm_create_pool(msg)?;
            let pos_size = pool_size
                .try_into()
                .map_err(|_| tag!("Need nonnegative shm pool size, given {}", pool_size))?;

            let buffer: Rc<RefCell<ShadowFd>> = match transl {
                TranslationInfo::FromChannel((x, y)) => {
                    let sfd = x.pop_front().ok_or_else(|| tag!("Missing fd"))?;
                    y.push_back(sfd.clone());
                    sfd
                }
                TranslationInfo::FromWayland((x, y)) => {
                    // Note: actual file size provided may be larger than pool_size,
                    // since there may be following wl_shm_pool::resize calls

                    let v = translate_shm_fd(
                        x.pop_front().ok_or_else(|| tag!("Missing fd"))?,
                        pos_size,
                        &mut glob.map,
                        &mut glob.max_local_id,
                        false,
                        false,
                        false,
                    )?;
                    y.push(v.clone());
                    v
                }
            };

            insert_new_object(
                &mut glob.objects,
                pool_id,
                WpObject {
                    obj_type: WaylandInterface::WlShmPool,
                    extra: WpExtra::WlShmPool(Box::new(ObjWlShmPool { buffer })),
                },
            )?;

            copy_msg_tag_fd(msg, dst, from_channel)?;

            Ok(ProcMsg::Done)
        }
        (WaylandInterface::WlShmPool, OPCODE_WL_SHM_POOL_CREATE_BUFFER) => {
            check_space!(msg.len(), 0, remaining_space);
            copy_msg(msg, dst);

            let (buffer_id, offset, width, height, stride, format) =
                parse_req_wl_shm_pool_create_buffer(msg)?;

            if width <= 0 || height <= 0 {
                return Err(tag!(
                    "wl_shm buffer width, height should be positive, not {} x {}",
                    width,
                    height
                ));
            }
            if offset < 0 {
                return Err(tag!("wl_shm buffer offsets must be nonnegative"));
            }
            if stride < 0 {
                /* While negative strides have a possible interpretation
                 * for single plane formats, the protocol documentation is
                 * vague and libwayland rejects them */
                return Err(tag!("wl_shm stride should be nonnegative, not {}", stride));
            }

            let info = ObjWlBufferShm {
                width: width.max(0),
                height: height.max(0),
                format,
                offset,
                stride,
            };

            let sfd = if let WpExtra::WlShmPool(ref x) = &obj.extra {
                x.buffer.clone()
            } else {
                return Err(tag!("wl_shm_pool object has invalid extra type"));
            };

            let b = sfd.borrow();
            let ShadowFdVariant::File(f) = &b.data else {
                panic!("WpExtra::WlShmPool should only link to a shm fd");
            };
            let buf_size = f.buffer_size;
            drop(b);

            if let Some(layout) = get_shm_format_layout(info.format) {
                /* If the format is something for which damage could feasibly be calculated,
                 * also confirm the submitted buffer is actually large enough */
                let too_small =
                    if let Some(end) = extent_for_linear_buffer(&info, &layout).map(|i| i.1) {
                        buf_size < end
                    } else {
                        true
                    };
                if too_small {
                    return Err(tag!(
                        "wl_shm_pool is too small to contain provided wl_buffer"
                    ));
                }
            }

            insert_new_object(
                &mut glob.objects,
                buffer_id,
                WpObject {
                    obj_type: WaylandInterface::WlBuffer,
                    extra: WpExtra::WlBuffer(Box::new(ObjWlBuffer {
                        sfd,
                        shm_info: Some(info),
                        unique_id: glob.max_buffer_uid,
                    })),
                },
            )?;
            glob.max_buffer_uid += 1;

            Ok(ProcMsg::Done)
        }

        (WaylandInterface::WlShmPool, OPCODE_WL_SHM_POOL_RESIZE) => {
            check_space!(msg.len(), 0, remaining_space);

            let WpExtra::WlShmPool(ref x) = &obj.extra else {
                return Err(tag!("wl_shm_pool object has invalid extra type"));
            };

            if file_has_pending_apply_tasks(&x.buffer)? {
                let b = x.buffer.borrow();
                return Ok(ProcMsg::WaitFor(b.remote_id));
            }

            copy_msg(msg, dst);

            if glob.on_display_side {
                /* The application side is responsible for notifying the display
                 * side of the updated buffer size and its contents. */
                return Ok(ProcMsg::Done);
            }

            let size = parse_req_wl_shm_pool_resize(msg)?;
            let new_size: usize = size
                .try_into()
                .map_err(|_| tag!("Invalid buffer size: {}", size))?;

            let x: &mut ShadowFd = &mut x.buffer.borrow_mut();
            if let ShadowFdVariant::File(ref mut y) = x.data {
                if new_size < y.buffer_size {
                    return Err(tag!(
                        "Invalid decrease in size ({} -> {}) for wl_shm_pool::resize",
                        y.buffer_size,
                        new_size
                    ));
                }
                y.buffer_size = new_size;

                // extend the mirror and initialize a new mapping
                // requires that no other operations be in progress on mapping
                update_core_for_new_size(&y.fd, new_size, &mut y.core)?;
            }

            Ok(ProcMsg::Done)
        }
        (WaylandInterface::WlCompositor, OPCODE_WL_COMPOSITOR_CREATE_SURFACE) => {
            check_space!(msg.len(), 0, remaining_space);

            let surf_id = parse_req_wl_compositor_create_surface(msg)?;

            let d = DamageBatch {
                damage: Vec::new(),
                damage_buffer: Vec::new(),
                attachment: BufferAttachment {
                    scale: 1,
                    transform: WlOutputTransform::Normal,
                    viewport_src: None,
                    viewport_dst: None,
                    buffer_uid: 0,
                    buffer_size: (0, 0),
                },
            };

            insert_new_object(
                &mut glob.objects,
                surf_id,
                WpObject {
                    obj_type: WaylandInterface::WlSurface,
                    extra: WpExtra::WlSurface(Box::new(ObjWlSurface {
                        attached_buffer_id: None,
                        damage_history: [
                            d.clone(),
                            d.clone(),
                            d.clone(),
                            d.clone(),
                            d.clone(),
                            d.clone(),
                            d.clone(),
                        ],
                        acquire_pt: None,
                        release_pt: None,
                        viewport_id: None,
                    })),
                },
            )?;

            copy_msg(msg, dst);

            Ok(ProcMsg::Done)
        }
        (WaylandInterface::WlSurface, OPCODE_WL_SURFACE_DESTROY) => {
            check_space!(msg.len(), 0, remaining_space);
            copy_msg(msg, dst);

            let WpExtra::WlSurface(ref mut surf) = obj.extra else {
                return Err(tag!("Surface object has invalid extra type"));
            };
            let mut tmp = None;
            std::mem::swap(&mut tmp, &mut surf.viewport_id);
            if let Some(vp_id) = tmp {
                if let Some(ref mut object) = glob.objects.get_mut(&vp_id) {
                    let WpExtra::WpViewport(ref mut viewport) = object.extra else {
                        return Err(tag!("Viewport object has invalid extra type"));
                    };
                    viewport.wl_surface = None;
                }
            }
            Ok(ProcMsg::Done)
        }
        (WaylandInterface::WlSurface, OPCODE_WL_SURFACE_ATTACH) => {
            check_space!(msg.len(), 0, remaining_space);

            let (buf_id, _x, _y) = parse_req_wl_surface_attach(msg)?;

            if let Some(ref mut object) = glob.objects.get_mut(&object_id) {
                if let WpExtra::WlSurface(ref mut x) = &mut object.extra {
                    x.attached_buffer_id = if buf_id != ObjId(0) {
                        Some(buf_id)
                    } else {
                        None
                    };
                } else {
                    return Err(tag!("Surface object has invalid extra type"));
                }
            } else {
                return Err(tag!("Attaching to nonexistant object"));
            }

            copy_msg(msg, dst);

            Ok(ProcMsg::Done)
        }
        (WaylandInterface::WlSurface, OPCODE_WL_SURFACE_SET_BUFFER_SCALE) => {
            check_space!(msg.len(), 0, remaining_space);
            copy_msg(msg, dst);

            let s = parse_req_wl_surface_set_buffer_scale(msg)?;

            if s <= 0 {
                return Err(tag!("wl_surface.set_buffer_scale used nonpositive scale"));
            }

            let WpExtra::WlSurface(ref mut surf) = &mut obj.extra else {
                return Err(tag!("Surface object has invalid extra type"));
            };

            surf.damage_history[0].attachment.scale = s as u32;

            Ok(ProcMsg::Done)
        }
        (WaylandInterface::WlSurface, OPCODE_WL_SURFACE_SET_BUFFER_TRANSFORM) => {
            check_space!(msg.len(), 0, remaining_space);
            copy_msg(msg, dst);

            let t = parse_req_wl_surface_set_buffer_transform(msg)?;

            let WpExtra::WlSurface(ref mut surf) = &mut obj.extra else {
                return Err(tag!("Surface object has invalid extra type"));
            };
            if t < 0 {
                return Err(tag!("Buffer transform value should be nonnegative"));
            }
            surf.damage_history[0].attachment.transform = (t as u32)
                .try_into()
                .map_err(|()| "Not a valid transform type")?;

            Ok(ProcMsg::Done)
        }
        (WaylandInterface::WlSurface, OPCODE_WL_SURFACE_DAMAGE) => {
            check_space!(msg.len(), 0, remaining_space);
            copy_msg(msg, dst);

            if glob.on_display_side {
                /* Only the buffers on the application side will be dirty */
                return Ok(ProcMsg::Done);
            }

            let WpExtra::WlSurface(ref mut surf) = &mut obj.extra else {
                return Err(tag!("Surface object has invalid extra type"));
            };

            let (x, y, width, height) = parse_req_wl_surface_damage(msg)?;
            if width <= 0 || height <= 0 {
                /* This doesn't appear to be a protocol error; still, filter out degenerate rectangles */
                error!(
                    "Received degenerate damage rectangle: x={} y={} w={} h={}",
                    x, y, width, height
                );
            } else {
                surf.damage_history[0].damage.push(WlRect {
                    x,
                    y,
                    width,
                    height,
                });
            }

            Ok(ProcMsg::Done)
        }
        (WaylandInterface::WlSurface, OPCODE_WL_SURFACE_DAMAGE_BUFFER) => {
            check_space!(msg.len(), 0, remaining_space);
            copy_msg(msg, dst);

            if glob.on_display_side {
                /* Only the buffers on the application side will be dirty */
                return Ok(ProcMsg::Done);
            }

            let WpExtra::WlSurface(ref mut surf) = &mut obj.extra else {
                return Err(tag!("Surface object has invalid extra type"));
            };

            let (x, y, width, height) = parse_req_wl_surface_damage_buffer(msg)?;
            if width <= 0 || height <= 0 {
                /* This doesn't appear to be a protocol error; still, filter out degenerate rectangles */
                error!(
                    "Received degenerate damage rectangle: x={} y={} w={} h={}",
                    x, y, width, height
                );
            } else {
                surf.damage_history[0].damage_buffer.push(WlRect {
                    x,
                    y,
                    width,
                    height,
                });
            }

            Ok(ProcMsg::Done)
        }
        (WaylandInterface::WlSurface, OPCODE_WL_SURFACE_COMMIT) => {
            check_space!(msg.len(), 0, remaining_space);

            let () = parse_req_wl_surface_commit(msg)?;
            let WpExtra::WlSurface(ref x) = obj.extra else {
                return Err(tag!("Surface object has invalid extra type"));
            };
            let opt_buf_id: Option<ObjId> = x.attached_buffer_id;
            if x.acquire_pt.is_some() != x.release_pt.is_some() {
                return Err(tag!("Acquire/release points must both be set"));
            }
            let has_timelines = x.acquire_pt.is_some() || x.release_pt.is_some();

            if from_channel {
                if let Some(buf_id) = opt_buf_id {
                    if let Some(buf) = glob.objects.get(&buf_id) {
                        if let WpExtra::WlBuffer(ref buf_data) = buf.extra {
                            let b = buf_data.sfd.borrow();
                            let apply_count = if let ShadowFdVariant::File(data) = &b.data {
                                data.pending_apply_tasks
                            } else if let ShadowFdVariant::Dmabuf(data) = &b.data {
                                /* Note: there must be a wait _somewhere_ when using timelines, otherwise diffs may be received faster
                                 * than they can be processed. */
                                if has_timelines {
                                    0 /* do not wait for tasks; will signal acquire semaphore on last task completion */
                                } else {
                                    data.pending_apply_tasks
                                }
                            } else {
                                return Err(tag!("Attached buffer is not of file or dmabuf type"));
                            };
                            if apply_count > 0 {
                                return Ok(ProcMsg::WaitFor(b.remote_id));
                            }
                        }
                    }
                }
            }

            copy_msg(msg, dst);

            if glob.on_display_side {
                /* Only the buffers on the application side will be dirty */
                let obj = &mut glob.objects.get_mut(&object_id).unwrap();
                let WpExtra::WlSurface(ref mut x) = &mut obj.extra else {
                    return Err(tag!("Surface object has invalid extra type"));
                };

                if x.acquire_pt.is_some() != x.release_pt.is_some() {
                    return Err(tag!("Acquire/release points must both be set"));
                }
                /* These must be set every swap cycle, so take them */
                let mut acq_pt = None;
                let mut rel_pt = None;
                std::mem::swap(&mut x.acquire_pt, &mut acq_pt);
                std::mem::swap(&mut x.release_pt, &mut rel_pt);

                let opt_buf_id: Option<ObjId> = x.attached_buffer_id;
                if let Some(buf_id) = opt_buf_id {
                    if let Some(buf) = glob.objects.get(&buf_id) {
                        if let WpExtra::WlBuffer(ref buf_data) = buf.extra {
                            let mut sfd = buf_data.sfd.borrow_mut();
                            if let ShadowFdVariant::Dmabuf(ref mut y) = &mut sfd.data {
                                dmabuf_post_apply_task_operations(y)?;

                                if let Some((pt, timeline)) = acq_pt {
                                    y.acquires.push((pt, timeline));
                                }
                                if let Some((pt, timeline)) = rel_pt {
                                    let mut tsfd = timeline.borrow_mut();
                                    let ShadowFdVariant::Timeline(ref mut timeline_data) =
                                        tsfd.data
                                    else {
                                        panic!("Expected timeline sfd");
                                    };
                                    timeline_data.releases.push((pt, buf_data.sfd.clone()));
                                    let trid = tsfd.remote_id;
                                    drop(tsfd);

                                    y.releases.insert((trid, pt), timeline);
                                }
                                if y.pending_apply_tasks == 0 {
                                    /* All tasks completed before this was processed, so signal immediately */
                                    debug!("Tasks already done, signalling acquires");
                                    signal_timeline_acquires(&mut y.acquires)?;
                                }
                            }
                        }
                    }
                }

                return Ok(ProcMsg::Done);
            }

            let obj = &mut glob.objects.get_mut(&object_id).unwrap();
            let WpExtra::WlSurface(ref mut x) = &mut obj.extra else {
                return Err(tag!("Surface object has invalid extra type"));
            };

            if x.acquire_pt.is_some() != x.release_pt.is_some() {
                return Err(tag!("Acquire/release points must both be set"));
            }
            /* These must be set every swap cycle, so take them */
            let mut acq_pt = None;
            let mut rel_pt = None;
            std::mem::swap(&mut x.acquire_pt, &mut acq_pt);
            std::mem::swap(&mut x.release_pt, &mut rel_pt);

            let mut current_attachment = x.damage_history[0].attachment.clone();

            /* This shifts all entries of x.damage_history right by one */
            // mutable wl_surface reference dropped; now reading from wl_buffer and wl_surface objects

            let mut found_buffer = false;
            if let Some(buf_id) = opt_buf_id {
                /* Note: this is vulnerable to ABA-type problems when the buffer is
                 * destroyed and a new one with the same id is recreated; however,
                 * this is client misbehavior and Waypipe can silently ignore it,
                 * apply extra damage to the new buffer, and let the compositor
                 * handle it. */
                if let Some(buf) = glob.objects.get(&buf_id) {
                    let obj = &glob.objects.get(&object_id).unwrap();
                    let WpExtra::WlSurface(ref x) = &obj.extra else {
                        return Err(tag!(
                            "Object produced through unexpected method, missing associated state"
                        ));
                    };
                    if let WpExtra::WlBuffer(ref buf_data) = buf.extra {
                        let mut sfd = buf_data.sfd.borrow_mut();
                        let buffer_size: (i32, i32) = if let ShadowFdVariant::File(_) = sfd.data {
                            let Some(shm_info) = buf_data.shm_info else {
                                return Err(tag!(
                                    "Expected shm info for wl_buffer with File-type ShadowFd"
                                ));
                            };
                            (shm_info.width, shm_info.height)
                        } else if let ShadowFdVariant::Dmabuf(ref y) = sfd.data {
                            match y.buf {
                                DmabufImpl::Vulkan(ref buf) => (
                                    buf.width.try_into().unwrap(),
                                    buf.height.try_into().unwrap(),
                                ),
                                DmabufImpl::Gbm(ref buf) => (
                                    buf.width.try_into().unwrap(),
                                    buf.height.try_into().unwrap(),
                                ),
                            }
                        } else {
                            return Err(tag!("Expected buffer shadowfd to be of file type"));
                        };

                        current_attachment.buffer_uid = buf_data.unique_id;
                        current_attachment.buffer_size = buffer_size;
                        found_buffer = true;

                        if current_attachment.buffer_size.0 % current_attachment.scale as i32 != 0
                            || current_attachment.buffer_size.1 % current_attachment.scale as i32
                                != 0
                        {
                            /* This is required as per wl_surface::attach documentation; compositors
                             * should send an error shortly after this occurs. */
                            error!("wl_buffer size at commit time is not divisible by surface scale, likely client protocol error");
                        }

                        if let Some((_vx, _vy, vw, vh)) = current_attachment.viewport_src {
                            if current_attachment.viewport_dst.is_none()
                                && (vw % 256 != 0 || vh % 256 != 0)
                            {
                                error!("viewport dst size is unset while src size is not integral, likely client protocol error");
                            }
                        }

                        if let ShadowFdVariant::File(ref mut y) = &mut sfd.data {
                            match &y.damage {
                                Damage::Everything => {}
                                Damage::Intervals(old) => {
                                    let dmg = get_damage_for_shm(buf_data, x, &current_attachment);
                                    y.damage =
                                        Damage::Intervals(union_damage(&old[..], &dmg[..], 128));
                                }
                            }
                        } else if let ShadowFdVariant::Dmabuf(ref mut y) = &mut sfd.data {
                            match &y.damage {
                                Damage::Everything => {}
                                Damage::Intervals(old) => {
                                    let dmg = get_damage_for_dmabuf(y, x, &current_attachment);
                                    y.damage =
                                        Damage::Intervals(union_damage(&old[..], &dmg[..], 128));
                                }
                            }
                            if acq_pt.is_none() {
                                y.using_implicit_sync = true;
                            }

                            /* todo: in theory these should still work with shm buffers */
                            if let Some((pt, timeline)) = acq_pt {
                                y.acquires.push((pt, timeline));
                            }
                            if let Some((pt, timeline)) = rel_pt {
                                let mut tsfd = timeline.borrow_mut();
                                let ShadowFdVariant::Timeline(ref mut timeline_data) = tsfd.data
                                else {
                                    panic!("Expected timeline sfd");
                                };
                                timeline_data.releases.push((pt, buf_data.sfd.clone()));
                                let trid = tsfd.remote_id;
                                drop(tsfd);

                                y.releases.insert((trid, pt), timeline);
                            }
                        } else {
                            unreachable!();
                        }
                    }
                } else {
                    debug!("Attached wl_buffer {} for wl_surface {} destroyed before commit: the result of this is not specified and compositors may do anything. Interpreting as null attachment.", buf_id, object_id);
                }
            }

            /* acquire mutable reference again */
            let obj = &mut glob.objects.get_mut(&object_id).unwrap();

            let WpExtra::WlSurface(ref mut x) = &mut obj.extra else {
                return Err(tag!("Surface object has invalid extra type"));
            };

            if found_buffer {
                /* Have not yet updated properties for the current buffer attachment, so do so now */
                x.damage_history[0].attachment = current_attachment.clone();
                /* Rotate the damage log */
                let mut fresh = DamageBatch {
                    attachment: current_attachment.clone(),
                    damage: Vec::new(),
                    damage_buffer: Vec::new(),
                };
                std::mem::swap(&mut x.damage_history[6], &mut fresh);
                x.damage_history.swap(5, 6);
                x.damage_history.swap(4, 5);
                x.damage_history.swap(3, 4);
                x.damage_history.swap(2, 3);
                x.damage_history.swap(1, 2);
                x.damage_history.swap(0, 1);
            } else {
                /* Null attachment (or buffer of unknown properties and size), wipe history */
                current_attachment.buffer_uid = 0;
                current_attachment.buffer_size = (0, 0);
                for i in 0..7 {
                    x.damage_history[i] = DamageBatch {
                        attachment: current_attachment.clone(),
                        damage: Vec::new(),
                        damage_buffer: Vec::new(),
                    };
                }
            }

            Ok(ProcMsg::Done)
        }
        (WaylandInterface::WpViewporter, OPCODE_WP_VIEWPORTER_GET_VIEWPORT) => {
            check_space!(msg.len(), 0, remaining_space);
            copy_msg(msg, dst);

            let (new_id, surface) = parse_req_wp_viewporter_get_viewport(msg)?;
            insert_new_object(
                &mut glob.objects,
                new_id,
                WpObject {
                    obj_type: WaylandInterface::WpViewport,
                    extra: WpExtra::WpViewport(Box::new(ObjWpViewport {
                        wl_surface: Some(surface),
                    })),
                },
            )?;

            Ok(ProcMsg::Done)
        }
        (WaylandInterface::WpViewport, OPCODE_WP_VIEWPORT_DESTROY) => {
            check_space!(msg.len(), 0, remaining_space);
            copy_msg(msg, dst);

            let WpExtra::WpViewport(ref mut viewport) = obj.extra else {
                return Err(tag!("Viewport object has invalid extra type"));
            };
            let mut tmp = None;
            std::mem::swap(&mut tmp, &mut viewport.wl_surface);
            if let Some(surf_id) = tmp {
                if let Some(ref mut object) = glob.objects.get_mut(&surf_id) {
                    let WpExtra::WlSurface(ref mut surface) = object.extra else {
                        return Err(tag!("Surface object has invalid extra type"));
                    };
                    surface.damage_history[0].attachment.viewport_src = None;
                    surface.damage_history[0].attachment.viewport_dst = None;
                    surface.viewport_id = None;
                }
            }
            Ok(ProcMsg::Done)
        }
        (WaylandInterface::WpViewport, OPCODE_WP_VIEWPORT_SET_DESTINATION) => {
            check_space!(msg.len(), 0, remaining_space);
            copy_msg(msg, dst);

            let (w, h) = parse_req_wp_viewport_set_destination(msg)?;
            let destination: Option<(i32, i32)> = if w == -1 && h == -1 {
                None
            } else if w <= 0 || h <= 0 {
                return Err(tag!("invalid wp_viewport destination ({},{})", w, h));
            } else {
                Some((w, h))
            };

            let WpExtra::WpViewport(ref mut viewport) = obj.extra else {
                return Err(tag!("Viewport object has invalid extra type"));
            };
            if let Some(surf_id) = viewport.wl_surface {
                if let Some(ref mut object) = glob.objects.get_mut(&surf_id) {
                    let WpExtra::WlSurface(ref mut surface) = object.extra else {
                        return Err(tag!("Surface object has invalid extra type"));
                    };

                    surface.damage_history[0].attachment.viewport_dst = destination;
                }
            }
            Ok(ProcMsg::Done)
        }
        (WaylandInterface::WpViewport, OPCODE_WP_VIEWPORT_SET_SOURCE) => {
            check_space!(msg.len(), 0, remaining_space);
            copy_msg(msg, dst);

            let (x, y, w, h) = parse_req_wp_viewport_set_source(msg)?;
            let source: Option<(i32, i32, i32, i32)> =
                if x == -256 && y == -256 && w == -256 && h == -256 {
                    None
                } else if x < 0 || y < 0 || w <= 0 || h <= 0 {
                    return Err(tag!("invalid wp_viewport source ({},{},{},{})", x, y, w, h));
                } else {
                    Some((x, y, w, h))
                };

            let WpExtra::WpViewport(ref mut viewport) = obj.extra else {
                return Err(tag!("Viewport object has invalid extra type"));
            };
            if let Some(surf_id) = viewport.wl_surface {
                if let Some(ref mut object) = glob.objects.get_mut(&surf_id) {
                    let WpExtra::WlSurface(ref mut surface) = object.extra else {
                        return Err(tag!("Surface object has invalid extra type"));
                    };

                    surface.damage_history[0].attachment.viewport_src = source;
                }
            }
            Ok(ProcMsg::Done)
        }
        (
            WaylandInterface::WpLinuxDrmSyncobjManagerV1,
            OPCODE_WP_LINUX_DRM_SYNCOBJ_MANAGER_V1_GET_SURFACE,
        ) => {
            check_space!(msg.len(), 0, remaining_space);
            copy_msg(msg, dst);

            let (new_id, surf_id) = parse_req_wp_linux_drm_syncobj_manager_v1_get_surface(msg)?;

            // Currently unclear how drm_syncobj interacts with wl_shm-type buffers; in theory,
            // with GPU access to CPU memory, it _should_ be possible.

            insert_new_object(
                &mut glob.objects,
                new_id,
                WpObject {
                    obj_type: WaylandInterface::WpLinuxDrmSyncobjSurfaceV1,
                    extra: WpExtra::WpDrmSyncobjSurface(Box::new(ObjWpDrmSyncobjSurface {
                        // TODO: check if ABA problem applies
                        surface: surf_id,
                    })),
                },
            )?;

            Ok(ProcMsg::Done)
        }
        (
            WaylandInterface::WpLinuxDrmSyncobjManagerV1,
            OPCODE_WP_LINUX_DRM_SYNCOBJ_MANAGER_V1_IMPORT_TIMELINE,
        ) => {
            check_space!(msg.len(), 1, remaining_space);

            let new_id = parse_req_wp_linux_drm_syncobj_manager_v1_import_timeline(msg)?;

            let sfd = match transl {
                TranslationInfo::FromChannel((x, y)) => {
                    let sfd = x.pop_front().ok_or_else(|| tag!("Missing sfd"))?;
                    let mut b = sfd.borrow_mut();
                    if let ShadowFdVariant::Timeline(t) = &mut b.data {
                        t.debug_wayland_id = new_id;
                    } else {
                        return Err(tag!("Expected timeline fd"));
                    }
                    drop(b);

                    y.push_back(sfd.clone());
                    sfd
                }
                TranslationInfo::FromWayland((x, y)) => {
                    let v = translate_timeline(
                        x.pop_front().ok_or_else(|| tag!("Missing fd"))?,
                        glob,
                        new_id,
                    )?;
                    y.push(v.clone());
                    v
                }
            };

            insert_new_object(
                &mut glob.objects,
                new_id,
                WpObject {
                    obj_type: WaylandInterface::WpLinuxDrmSyncobjTimelineV1,
                    extra: WpExtra::WpDrmSyncobjTimeline(Box::new(ObjWpDrmSyncobjTimeline {
                        // TODO: check if ABA problem applies
                        timeline: sfd,
                    })),
                },
            )?;

            copy_msg_tag_fd(msg, dst, from_channel)?;
            Ok(ProcMsg::Done)
        }
        (
            WaylandInterface::WpLinuxDrmSyncobjSurfaceV1,
            OPCODE_WP_LINUX_DRM_SYNCOBJ_SURFACE_V1_SET_ACQUIRE_POINT,
        )
        | (
            WaylandInterface::WpLinuxDrmSyncobjSurfaceV1,
            OPCODE_WP_LINUX_DRM_SYNCOBJ_SURFACE_V1_SET_RELEASE_POINT,
        ) => {
            check_space!(msg.len(), 0, remaining_space);

            let acquire = mod_opcode == OPCODE_WP_LINUX_DRM_SYNCOBJ_SURFACE_V1_SET_ACQUIRE_POINT;

            let (timeline_id, pt_hi, pt_lo) = if acquire {
                parse_req_wp_linux_drm_syncobj_surface_v1_set_acquire_point(msg)?
            } else {
                parse_req_wp_linux_drm_syncobj_surface_v1_set_release_point(msg)?
            };
            let pt = join_u64(pt_hi, pt_lo);

            let WpExtra::WpDrmSyncobjSurface(s) = &obj.extra else {
                return Err(tag!(
                    "Object produced through unexpected method, missing associated state"
                ));
            };
            let surf_id = s.surface;

            let timeline_obj = glob.objects.get(&timeline_id).ok_or("")?;
            let WpExtra::WpDrmSyncobjTimeline(t) = &timeline_obj.extra else {
                return Err(tag!("Incorrect extra type, expected timeline"));
            };
            let sfd = t.timeline.clone();

            // Currently unclear how drm_syncobj interacts with wl_shm-type buffers; in theory,
            // with GPU access to CPU memory, it _should_ be possible.

            let surface_obj = glob.objects.get_mut(&surf_id).ok_or("")?;
            let WpExtra::WlSurface(ref mut surf) = &mut surface_obj.extra else {
                return Err(tag!("Incorrect extra type, expected surface"));
            };

            if acquire {
                surf.acquire_pt = Some((pt, sfd));
            } else {
                surf.release_pt = Some((pt, sfd));
            }

            copy_msg(msg, dst);
            Ok(ProcMsg::Done)
        }
        (WaylandInterface::ZwpLinuxDmabufV1, OPCODE_ZWP_LINUX_DMABUF_V1_FORMAT) => {
            let format = parse_evt_zwp_linux_dmabuf_v1_format(msg)?;

            match glob.dmabuf_device {
                DmabufDevice::Unknown | DmabufDevice::Unavailable => {
                    unreachable!();
                }
                DmabufDevice::VulkanSetup(_) => {
                    /* Device not fully initialized; drop the format event. This is safe
                     * because VulkanSetup only occurs if there are only v4+ bindings (where
                     * format negotiation happens through the feedback mechanism instead). */
                    debug!("Dropping unexpected zwp_linux_dmabuf_v1::format event, likely compositor error");
                    return Ok(ProcMsg::Done);
                }
                DmabufDevice::Vulkan((_, ref vulk)) => {
                    let mod_linear = 0;
                    if !vulk.supports_format(format, mod_linear) {
                        /* Drop message, format not supported in standard scenario (linear modifier);
                         * and this event cannot communicate any modifiers. */
                        return Ok(ProcMsg::Done);
                    }
                }
                DmabufDevice::Gbm(ref gbm) => {
                    if gbm_supported_modifiers(gbm, format).is_empty() {
                        /* Format not supported */
                        return Ok(ProcMsg::Done);
                    }
                }
            }
            check_space!(msg.len(), 0, remaining_space);
            copy_msg(msg, dst);
            Ok(ProcMsg::Done)
        }
        (WaylandInterface::ZwpLinuxDmabufV1, OPCODE_ZWP_LINUX_DMABUF_V1_MODIFIER) => {
            let (format, mod_hi, mod_lo) = parse_evt_zwp_linux_dmabuf_v1_modifier(msg)?;
            let modifier = join_u64(mod_hi, mod_lo);

            if glob.on_display_side {
                /* Restrict the format/modifier pairs to what this instance of Waypipe supports */
                if !dmabuf_dev_supports_format(&glob.dmabuf_device, format, modifier) {
                    return Ok(ProcMsg::Done);
                }
                check_space!(msg.len(), 0, remaining_space);
                add_advertised_modifiers(&mut glob.advertised_modifiers, format, &[modifier]);
                copy_msg(msg, dst);
                Ok(ProcMsg::Done)
            } else {
                /* For each format, replace the modifiers listed with what this instance of Waypipe accepts */
                let WpExtra::ZwpDmabuf(d) = &mut obj.extra else {
                    return Err(tag!(
                        "Object produced through unexpected method, missing associated state"
                    ));
                };
                if d.formats_seen.contains(&format) {
                    return Ok(ProcMsg::Done);
                }
                d.formats_seen.insert(format);

                let mods = dmabuf_dev_modifier_list(&glob.dmabuf_device, format);
                check_space!(
                    mods.len() * length_evt_zwp_linux_dmabuf_v1_modifier(),
                    0,
                    remaining_space
                );
                add_advertised_modifiers(&mut glob.advertised_modifiers, format, mods);

                for new_mod in mods {
                    let (nmod_hi, nmod_lo) = split_u64(*new_mod);
                    write_evt_zwp_linux_dmabuf_v1_modifier(
                        dst, object_id, format, nmod_hi, nmod_lo,
                    );
                }
                Ok(ProcMsg::Done)
            }
        }

        (WaylandInterface::ZwpLinuxDmabufV1, OPCODE_ZWP_LINUX_DMABUF_V1_CREATE_PARAMS) => {
            check_space!(msg.len(), 0, remaining_space);
            copy_msg(msg, dst);

            let params_id = parse_req_zwp_linux_dmabuf_v1_create_params(msg)?;

            insert_new_object(
                &mut glob.objects,
                params_id,
                WpObject {
                    obj_type: WaylandInterface::ZwpLinuxBufferParamsV1,
                    extra: WpExtra::ZwpDmabufParams(Box::new(ObjZwpLinuxDmabufParams {
                        planes: Vec::new(),
                        dmabuf: None,
                    })),
                },
            )?;

            Ok(ProcMsg::Done)
        }

        (WaylandInterface::ZwpLinuxBufferParamsV1, OPCODE_ZWP_LINUX_BUFFER_PARAMS_V1_ADD) => {
            /* This message will be consumed and recreated later once full information has arrived;
             * it can always be processed */

            let (plane_idx, offset, stride, modifier_hi, modifier_lo) =
                parse_req_zwp_linux_buffer_params_v1_add(msg)?;
            let modifier: u64 = join_u64(modifier_hi, modifier_lo);

            let WpExtra::ZwpDmabufParams(ref mut p) = &mut obj.extra else {
                return Err(tag!("Incorrect extra type")); /* TODO: make a helper method for this, with standardized error ? */
            };

            match transl {
                TranslationInfo::FromChannel((_, _)) => {
                    /* Do nothing: the OpenDMABUF message will be sent at time of ::create or ::create_immed ;
                     * there is no point in dequeing it earlier. Also, there will only be one message, even if
                     * there are multiple planes. */
                }
                TranslationInfo::FromWayland((x, _)) => {
                    let fd = x.pop_front().ok_or_else(|| tag!("Missing fd"))?;

                    p.planes.push(AddDmabufPlane {
                        fd,
                        plane_idx,
                        offset,
                        stride,
                        modifier,
                    });
                }
            };

            Ok(ProcMsg::Done)
        }

        (
            WaylandInterface::ZwpLinuxBufferParamsV1,
            OPCODE_ZWP_LINUX_BUFFER_PARAMS_V1_CREATE_IMMED,
        ) => {
            let WpExtra::ZwpDmabufParams(ref mut params) = &mut obj.extra else {
                return Err(tag!("Incorrect extra type")); /* TODO: make a helper method for this, with standardized error ? */
            };

            let (buffer_id, width, height, drm_format, flags) =
                parse_req_zwp_linux_buffer_params_v1_create_immed(msg)?;
            if width <= 0 || height <= 0 {
                return Err(tag!("DMABUF width or height should be positive"));
            }
            let (width, height) = (width as u32, height as u32);

            if flags != 0 {
                return Err(tag!("DMABUF flags not yet supported"));
            }

            let sfd: Rc<RefCell<ShadowFd>> = match transl {
                TranslationInfo::FromChannel((x, y)) => {
                    let sfd = &x.front().ok_or_else(|| tag!("Missing sfd"))?;
                    let mut b = sfd.borrow_mut();
                    let ShadowFdVariant::Dmabuf(ref mut data) = &mut b.data else {
                        return Err(tag!("Incorrect extra type"));
                    };

                    data.debug_wayland_id = buffer_id;

                    let estimated_msg_len = length_req_zwp_linux_buffer_params_v1_add()
                        * data.export_planes.len()
                        + length_req_zwp_linux_buffer_params_v1_create_immed();
                    check_space!(estimated_msg_len, data.export_planes.len(), remaining_space);

                    for plane in data.export_planes.iter() {
                        let (mod_hi, mod_lo) = split_u64(plane.modifier);
                        write_req_zwp_linux_buffer_params_v1_add(
                            dst,
                            object_id,
                            false,
                            plane.plane_idx,
                            plane.offset,
                            plane.stride,
                            mod_hi,
                            mod_lo,
                        );
                    }
                    copy_msg(msg, dst);

                    drop(b);

                    let sfd = x.pop_front().unwrap();
                    y.push_back(sfd.clone());
                    sfd
                }
                TranslationInfo::FromWayland((_, y)) => {
                    /* Finally process all the SFDs, introduce add messages */
                    let estimated_msg_len = length_req_zwp_linux_buffer_params_v1_add()
                    /* send add messages as-if the buffer were actually linear,
                     * for improved compatibility with older Waypipe */
                    + length_req_zwp_linux_buffer_params_v1_create_immed();
                    check_space!(estimated_msg_len, params.planes.len(), remaining_space);

                    let mut planes = Vec::new();
                    std::mem::swap(&mut params.planes, &mut planes);

                    let sfd = translate_dmabuf_fd(
                        width,
                        height,
                        drm_format,
                        planes,
                        &glob.opts,
                        &glob.dmabuf_device,
                        &mut glob.max_local_id,
                        &mut glob.map,
                        buffer_id,
                    )?;
                    y.push(sfd.clone());

                    /* Recreate all the 'add' messages at once. While this is technically pointless when Waypipe runs against itself,
                     * since those add messages get ignored and dropped, older versions may care. */
                    let mod_linear: u64 = 0;
                    /* Assuming format is single planar, instruct other end to create a linear buffer;
                     * older Waypipe will do this, and newer will ignore it. */
                    let (mod_hi, mod_lo) = split_u64(mod_linear);

                    let wayl_format = drm_to_wayland(drm_format);
                    // TODO: this will need to change for multiplanar or packed formats
                    let bpp = get_shm_format_layout(wayl_format).unwrap().planes[0].bpt;

                    write_req_zwp_linux_buffer_params_v1_add(
                        dst,
                        object_id,
                        true,
                        /* plane idx */ 0,
                        /* offset */ 0,
                        bpp.get().checked_mul(width).unwrap(), // stride as if tightly packed
                        mod_hi,
                        mod_lo,
                    );
                    write_req_zwp_linux_buffer_params_v1_create_immed(
                        dst,
                        object_id,
                        buffer_id,
                        width as i32,
                        height as i32,
                        drm_format,
                        flags,
                    );

                    sfd
                }
            };

            insert_new_object(
                &mut glob.objects,
                buffer_id,
                WpObject {
                    obj_type: WaylandInterface::WlBuffer,
                    extra: WpExtra::WlBuffer(Box::new(ObjWlBuffer {
                        sfd,
                        shm_info: None,
                        unique_id: glob.max_buffer_uid,
                    })),
                },
            )?;
            glob.max_buffer_uid += 1;

            Ok(ProcMsg::Done)
        }
        (WaylandInterface::ZwpLinuxBufferParamsV1, OPCODE_ZWP_LINUX_BUFFER_PARAMS_V1_CREATE) => {
            let WpExtra::ZwpDmabufParams(ref mut params) = &mut obj.extra else {
                return Err(tag!("Incorrect extra type")); /* TODO: make a helper method for this, with standardized error ? */
            };
            if params.dmabuf.is_some() {
                return Err(tag!("Can only create dmabuf from params once"));
            }

            let (width, height, drm_format, flags) =
                parse_req_zwp_linux_buffer_params_v1_create(msg)?;
            if width <= 0 || height <= 0 {
                return Err(tag!("DMABUF width or height should be positive"));
            }
            let (width, height) = (width as u32, height as u32);

            let sfd: Rc<RefCell<ShadowFd>> = match transl {
                TranslationInfo::FromChannel((x, y)) => {
                    let sfd = &x.front().ok_or_else(|| tag!("Missing sfd"))?;
                    let mut b = sfd.borrow_mut();
                    let ShadowFdVariant::Dmabuf(ref mut data) = &mut b.data else {
                        return Err(tag!("Incorrect extra type"));
                    };
                    let estimated_msg_len = length_req_zwp_linux_buffer_params_v1_add()
                        * data.export_planes.len()
                        + length_req_zwp_linux_buffer_params_v1_create_immed();
                    check_space!(estimated_msg_len, data.export_planes.len(), remaining_space);

                    for plane in data.export_planes.iter() {
                        let (mod_hi, mod_lo) = split_u64(plane.modifier);
                        write_req_zwp_linux_buffer_params_v1_add(
                            dst,
                            object_id,
                            false,
                            plane.plane_idx,
                            plane.offset,
                            plane.stride,
                            mod_hi,
                            mod_lo,
                        );
                    }
                    copy_msg(msg, dst);

                    drop(b);

                    let sfd = x.pop_front().unwrap();
                    y.push_back(sfd.clone());
                    sfd
                }
                TranslationInfo::FromWayland((_, y)) => {
                    /* Finally process all the SFDs, introduce add messages */
                    let estimated_msg_len = length_req_zwp_linux_buffer_params_v1_add()
                    /* send add messages as-if the buffer were actually linear,
                     * for improved compatibility with older Waypipe */
                    + length_req_zwp_linux_buffer_params_v1_create_immed();
                    check_space!(estimated_msg_len, params.planes.len(), remaining_space);

                    let mut planes = Vec::new();
                    std::mem::swap(&mut params.planes, &mut planes);

                    let sfd = translate_dmabuf_fd(
                        width,
                        height,
                        drm_format,
                        planes,
                        &glob.opts,
                        &glob.dmabuf_device,
                        &mut glob.max_local_id,
                        &mut glob.map,
                        ObjId(0),
                    )?;
                    y.push(sfd.clone());

                    /* Recreate all the 'add' messages at once. While this is technically pointless when Waypipe runs against itself,
                     * since those add messages get ignored and dropped, older versions may care. */
                    let mod_linear: u64 = 0;
                    /* Assuming format is single planar, instruct other end to create a linear buffer;
                     * older Waypipe will do this, and newer will ignore it. */
                    let (mod_hi, mod_lo) = split_u64(mod_linear);

                    let wayl_format = drm_to_wayland(drm_format);
                    // TODO: update for multiplanar/complicated pixel formats
                    let bpp = get_shm_format_layout(wayl_format).unwrap().planes[0].bpt;

                    write_req_zwp_linux_buffer_params_v1_add(
                        dst,
                        object_id,
                        true,
                        /* plane idx */ 0,
                        /* offset */ 0,
                        bpp.get().checked_mul(width).unwrap(), // stride as if tightly packed
                        mod_hi,
                        mod_lo,
                    );
                    write_req_zwp_linux_buffer_params_v1_create(
                        dst,
                        object_id,
                        width as i32,
                        height as i32,
                        drm_format,
                        flags,
                    );

                    sfd
                }
            };
            params.dmabuf = Some(sfd);
            Ok(ProcMsg::Done)
        }
        (WaylandInterface::ZwpLinuxBufferParamsV1, OPCODE_ZWP_LINUX_BUFFER_PARAMS_V1_FAILED) => {
            check_space!(msg.len(), 0, remaining_space);
            copy_msg(msg, dst);

            /* Discard reference to dmabuf now; compositor rejected it */
            let WpExtra::ZwpDmabufParams(ref mut params) = &mut obj.extra else {
                return Err(tag!("Incorrect extra type"));
            };
            params.dmabuf = None;
            Ok(ProcMsg::Done)
        }
        (WaylandInterface::ZwpLinuxBufferParamsV1, OPCODE_ZWP_LINUX_BUFFER_PARAMS_V1_CREATED) => {
            check_space!(msg.len(), 0, remaining_space);
            copy_msg(msg, dst);

            let obj_id = parse_evt_zwp_linux_buffer_params_v1_created(msg)?;
            let WpExtra::ZwpDmabufParams(ref mut params) = &mut obj.extra else {
                return Err(tag!("Incorrect extra type"));
            };

            let mut osfd = None;
            std::mem::swap(&mut params.dmabuf, &mut osfd);
            let Some(sfd) = osfd else {
                return Err(tag!(
                    "zwp_linux_buffer_params_v1::created event must follow ::create request and must occur at most once"
                ));
            };

            let mut b = sfd.borrow_mut();
            let ShadowFdVariant::Dmabuf(ref mut d) = b.data else {
                return Err(tag!("Incorrect shadowfd type"));
            };
            d.debug_wayland_id = obj_id;
            drop(b);

            insert_new_object(
                &mut glob.objects,
                obj_id,
                WpObject {
                    obj_type: WaylandInterface::WlBuffer,
                    extra: WpExtra::WlBuffer(Box::new(ObjWlBuffer {
                        sfd,
                        shm_info: None,
                        unique_id: glob.max_buffer_uid,
                    })),
                },
            )?;
            glob.max_buffer_uid += 1;

            Ok(ProcMsg::Done)
        }

        (WaylandInterface::ZwpLinuxDmabufV1, OPCODE_ZWP_LINUX_DMABUF_V1_GET_DEFAULT_FEEDBACK)
        | (WaylandInterface::ZwpLinuxDmabufV1, OPCODE_ZWP_LINUX_DMABUF_V1_GET_SURFACE_FEEDBACK) => {
            check_space!(msg.len(), 0, remaining_space);
            copy_msg(msg, dst);

            let feedback_id = if mod_opcode == OPCODE_ZWP_LINUX_DMABUF_V1_GET_DEFAULT_FEEDBACK {
                parse_req_zwp_linux_dmabuf_v1_get_default_feedback(msg)?
            } else {
                let (f, _surface) = parse_req_zwp_linux_dmabuf_v1_get_surface_feedback(msg)?;
                f
            };

            insert_new_object(
                &mut glob.objects,
                feedback_id,
                WpObject {
                    obj_type: WaylandInterface::ZwpLinuxDmabufFeedbackV1,
                    extra: WpExtra::ZwpDmabufFeedback(Box::new(ObjZwpLinuxDmabufFeedback {
                        input_format_table: None,
                        output_format_table: None,
                        main_device: None,
                        tranches: Vec::new(),
                        processed: false,
                        queued_format_table: None,
                        current: DmabufTranche {
                            flags: 0,
                            values: Vec::new(),
                            indices: Vec::new(),
                            device: 0,
                        },
                    })),
                },
            )?;

            Ok(ProcMsg::Done)
        }

        (
            WaylandInterface::ZwpLinuxDmabufFeedbackV1,
            OPCODE_ZWP_LINUX_DMABUF_FEEDBACK_V1_FORMAT_TABLE,
        ) => {
            let table_size = parse_evt_zwp_linux_dmabuf_feedback_v1_format_table(msg)?;
            let WpExtra::ZwpDmabufFeedback(ref mut f) = &mut obj.extra else {
                return Err(tag!("Expected object to have dmabuf_feedback data"));
            };

            /* Load the format table */
            let mut table_data = vec![0; table_size as usize];
            match transl {
                TranslationInfo::FromChannel((x, _)) => {
                    let sfd = x.front().ok_or_else(|| tag!("Missing sfd"))?;
                    if file_has_pending_apply_tasks(sfd)? {
                        let b = sfd.borrow();
                        return Ok(ProcMsg::WaitFor(b.remote_id));
                    }
                    let sfd = x.pop_front().unwrap();

                    let b = sfd.borrow_mut();
                    let ShadowFdVariant::File(ref f) = b.data else {
                        return Err(tag!("Received non-File ShadowFd for format table"));
                    };
                    if table_size as usize != f.buffer_size {
                        return Err(tag!(
                            "Wrong buffer size for format table: got {}, expected {}",
                            f.buffer_size,
                            table_size
                        ));
                    }

                    copy_from_mapping(&mut table_data, &f.core.as_ref().unwrap().mapping, 0);
                }
                TranslationInfo::FromWayland((x, _)) => {
                    let fd = x.pop_front().ok_or_else(|| tag!("Missing fd"))?;

                    let mapping = ExternalMapping::new(&fd, table_size as usize, true)?;
                    copy_from_mapping(&mut table_data, &mapping, 0);
                }
            };
            f.input_format_table = Some(parse_format_table(&table_data));
            Ok(ProcMsg::Done)
        }

        (
            WaylandInterface::ZwpLinuxDmabufFeedbackV1,
            OPCODE_ZWP_LINUX_DMABUF_FEEDBACK_V1_MAIN_DEVICE,
        ) => {
            let WpExtra::ZwpDmabufFeedback(ref mut feedback) = &mut obj.extra else {
                return Err(tag!("Unexpected object extra type"));
            };

            let dev = parse_evt_zwp_linux_dmabuf_feedback_v1_main_device(msg)?;
            let main_device = parse_dev_array(dev)
                .ok_or_else(|| tag!("Unexpected size for dev_t: {}", dev.len()))?;
            feedback.main_device = Some(main_device);

            if glob.on_display_side && matches!(glob.dmabuf_device, DmabufDevice::VulkanSetup(_)) {
                complete_dmabuf_setup(&glob.opts, Some(main_device), &mut glob.dmabuf_device)?;
            }

            Ok(ProcMsg::Done)
        }
        (
            WaylandInterface::ZwpLinuxDmabufFeedbackV1,
            OPCODE_ZWP_LINUX_DMABUF_FEEDBACK_V1_TRANCHE_FLAGS,
        ) => {
            let WpExtra::ZwpDmabufFeedback(ref mut feedback) = &mut obj.extra else {
                return Err(tag!("Unexpected object extra type"));
            };

            let flags = parse_evt_zwp_linux_dmabuf_feedback_v1_tranche_flags(msg)?;
            feedback.current.flags = flags;

            Ok(ProcMsg::Done)
        }
        (
            WaylandInterface::ZwpLinuxDmabufFeedbackV1,
            OPCODE_ZWP_LINUX_DMABUF_FEEDBACK_V1_TRANCHE_TARGET_DEVICE,
        ) => {
            let WpExtra::ZwpDmabufFeedback(ref mut feedback) = &mut obj.extra else {
                return Err(tag!("Unexpected object extra type"));
            };

            let dev = parse_evt_zwp_linux_dmabuf_feedback_v1_tranche_target_device(msg)?;
            feedback.current.device = parse_dev_array(dev)
                .ok_or_else(|| tag!("Unexpected size for dev_t: {}", dev.len()))?;

            Ok(ProcMsg::Done)
        }
        (
            WaylandInterface::ZwpLinuxDmabufFeedbackV1,
            OPCODE_ZWP_LINUX_DMABUF_FEEDBACK_V1_TRANCHE_FORMATS,
        ) => {
            let WpExtra::ZwpDmabufFeedback(ref mut feedback) = &mut obj.extra else {
                return Err(tag!("Unexpected object extra type"));
            };

            let fmts = parse_evt_zwp_linux_dmabuf_feedback_v1_tranche_formats(msg)?;
            if fmts.len() % 2 != 0 {
                return Err(tag!("Format array not of even length"));
            }
            if fmts.is_empty() {
                return Ok(ProcMsg::Done);
            }
            let Some(ref table) = feedback.input_format_table else {
                return Err(tag!(
                    "No format table provided before tranche_formats was received"
                ));
            };

            for chunk in fmts.chunks_exact(2) {
                let idx = u16::from_le_bytes(chunk.try_into().unwrap());

                let Some(pair) = table.get(idx as usize) else {
                    return Err(tag!(
                        "Tranche format index {} out of range for format table of length {}",
                        idx,
                        table.len()
                    ));
                };
                /* On display side, immediately filter out unsupported format-modifier pairs. */
                if glob.on_display_side
                    && !dmabuf_dev_supports_format(&glob.dmabuf_device, pair.0, pair.1)
                {
                    continue;
                }
                feedback.current.values.push(*pair);
            }

            Ok(ProcMsg::Done)
        }

        (
            WaylandInterface::ZwpLinuxDmabufFeedbackV1,
            OPCODE_ZWP_LINUX_DMABUF_FEEDBACK_V1_TRANCHE_DONE,
        ) => {
            let WpExtra::ZwpDmabufFeedback(ref mut feedback) = &mut obj.extra else {
                return Err(tag!("Unexpected object extra type"));
            };
            feedback.tranches.push(DmabufTranche {
                flags: 0,
                values: Vec::new(),
                indices: Vec::new(),
                device: 0,
            });
            std::mem::swap(feedback.tranches.last_mut().unwrap(), &mut feedback.current);
            Ok(ProcMsg::Done)
        }
        (WaylandInterface::ZwpLinuxDmabufFeedbackV1, OPCODE_ZWP_LINUX_DMABUF_FEEDBACK_V1_DONE) => {
            let WpExtra::ZwpDmabufFeedback(ref mut feedback) = &mut obj.extra else {
                return Err(tag!("Unexpected object extra type"));
            };

            let dev_len = write_dev_array(0).len();

            if !feedback.processed {
                /* Process feedback now, to determine how much space is needed for it. */
                feedback.processed = true;

                if !glob.on_display_side {
                    rebuild_format_table(&glob.dmabuf_device, feedback)?;
                }
                let new_table = process_dmabuf_feedback(feedback)?;
                if Some(&new_table) != feedback.output_format_table.as_ref() {
                    // Table has changed; send new fd
                    let local_fd = create_anon_file()
                        .map_err(|x| tag!("Failed to create shared-memory file: {:?}", x))?;
                    let sz: u32 = new_table.len().try_into().unwrap();
                    assert!(sz > 0);

                    unistd::ftruncate(&local_fd, sz as libc::off_t)
                        .map_err(|x| tag!("Failed to resize memfd: {:?}", x))?;
                    let mapping: ExternalMapping =
                        ExternalMapping::new(&local_fd, sz as usize, false).map_err(|x| {
                            tag!("Failed to mmap fd when building new format table: {}", x)
                        })?;
                    copy_onto_mapping(&new_table[..], &mapping, 0);

                    let sfd = translate_shm_fd(
                        local_fd,
                        sz as usize,
                        &mut glob.map,
                        &mut glob.max_local_id,
                        glob.on_display_side,
                        true,
                        !glob.on_display_side,
                    )?;

                    feedback.output_format_table = Some(new_table);
                    feedback.queued_format_table = Some((sfd, sz));
                }
            }

            let mut space_est = 0;
            if feedback.queued_format_table.is_some() {
                space_est += length_evt_zwp_linux_dmabuf_feedback_v1_format_table();
            }
            space_est += length_evt_zwp_linux_dmabuf_feedback_v1_main_device(dev_len);
            space_est += length_evt_zwp_linux_dmabuf_feedback_v1_done();

            /* note: this is now using the _converted_ tranche lengths and counts */
            for t in feedback.tranches.iter() {
                space_est += length_evt_zwp_linux_dmabuf_feedback_v1_tranche_done()
                    + length_evt_zwp_linux_dmabuf_feedback_v1_tranche_flags()
                    + length_evt_zwp_linux_dmabuf_feedback_v1_tranche_target_device(dev_len)
                    + length_evt_zwp_linux_dmabuf_feedback_v1_tranche_formats(t.indices.len());
            }

            check_space!(
                space_est,
                if feedback.queued_format_table.is_some() {
                    1
                } else {
                    0
                },
                remaining_space
            );

            let mut queued_table = None;
            std::mem::swap(&mut queued_table, &mut feedback.queued_format_table);
            if let Some((sfd, sz)) = queued_table {
                match transl {
                    TranslationInfo::FromChannel((_, y)) => {
                        write_evt_zwp_linux_dmabuf_feedback_v1_format_table(
                            dst, object_id, false, sz,
                        );
                        y.push_back(sfd);
                    }
                    TranslationInfo::FromWayland((_, y)) => {
                        write_evt_zwp_linux_dmabuf_feedback_v1_format_table(
                            dst, object_id, true, sz,
                        );
                        y.push(sfd);
                    }
                }
            }

            /* Write messages, filtering as necessary. */
            let dev_id = dmabuf_dev_get_id(&glob.dmabuf_device);
            write_evt_zwp_linux_dmabuf_feedback_v1_main_device(
                dst,
                object_id,
                write_dev_array(dev_id).as_slice(),
            );
            for t in feedback.tranches.iter() {
                for f in t.values.iter() {
                    add_advertised_modifiers(&mut glob.advertised_modifiers, f.0, &[f.1]);
                }
                write_evt_zwp_linux_dmabuf_feedback_v1_tranche_target_device(
                    dst,
                    object_id,
                    write_dev_array(dev_id).as_slice(),
                );
                write_evt_zwp_linux_dmabuf_feedback_v1_tranche_flags(dst, object_id, t.flags);
                write_evt_zwp_linux_dmabuf_feedback_v1_tranche_formats(
                    dst,
                    object_id,
                    &t.indices[..],
                );
                write_evt_zwp_linux_dmabuf_feedback_v1_tranche_done(dst, object_id);
            }
            write_evt_zwp_linux_dmabuf_feedback_v1_done(dst, object_id);

            /* Reset state for next batch */
            feedback.processed = false;
            feedback.tranches = Vec::new();
            // note: `feedback.current` _should_ already be reset in _tranche_done
            feedback.current = DmabufTranche {
                flags: 0,
                values: Vec::new(),
                indices: Vec::new(),
                device: 0,
            };

            Ok(ProcMsg::Done)
        }

        (WaylandInterface::WlKeyboard, OPCODE_WL_KEYBOARD_KEYMAP) => {
            check_space!(msg.len(), 1, remaining_space);

            let (_, keymap_size) = parse_evt_wl_keyboard_keymap(msg)?;
            if let Some(wait) = translate_or_wait_for_fixed_file(transl, glob, keymap_size)? {
                return Ok(wait);
            }
            copy_msg_tag_fd(msg, dst, from_channel)?;

            Ok(ProcMsg::Done)
        }
        (
            WaylandInterface::WpImageDescriptionInfoV1,
            OPCODE_WP_IMAGE_DESCRIPTION_INFO_V1_ICC_FILE,
        ) => {
            check_space!(msg.len(), 1, remaining_space);

            let file_size = parse_evt_wp_image_description_info_v1_icc_file(msg)?;
            if let Some(wait) = translate_or_wait_for_fixed_file(transl, glob, file_size)? {
                return Ok(wait);
            }
            copy_msg_tag_fd(msg, dst, from_channel)?;
            Ok(ProcMsg::Done)
        }
        (
            WaylandInterface::WpImageDescriptionCreatorIccV1,
            OPCODE_WP_IMAGE_DESCRIPTION_CREATOR_ICC_V1_SET_ICC_FILE,
        ) => {
            check_space!(msg.len(), 1, remaining_space);

            // TODO: only damage the portion of the file between 'offset' and 'offset+length'.
            // Or, to save remote resources: reduce the offset to zero and have Waypipe
            // convert between coordinates.
            // Also: mmap is not required to be supported, only seek+read, so this file may
            // need different handling anyway.
            let (offset, length) = parse_req_wp_image_description_creator_icc_v1_set_icc_file(msg)?;
            if length == 0 {
                return Err(tag!("File length for wp_image_description_creator_icc_v1::set_icc_file should not be zero"));
            }
            let Some(file_sz) = offset.checked_add(length) else {
                return Err(tag!("File offset+length={}+{} overflow for wp_image_description_creator_icc_v1::set_icc_file", offset, length));
            };

            match transl {
                TranslationInfo::FromChannel((x, y)) => {
                    let sfd = &x.front().ok_or_else(|| tag!("Missing fd"))?;
                    let rid = sfd.borrow().remote_id;
                    if file_has_pending_apply_tasks(sfd)? {
                        return Ok(ProcMsg::WaitFor(rid));
                    }
                    y.push_back(x.pop_front().unwrap());
                }
                TranslationInfo::FromWayland((x, y)) => {
                    let v = translate_shm_fd(
                        x.pop_front().ok_or_else(|| tag!("Missing fd"))?,
                        file_sz.try_into().unwrap(),
                        &mut glob.map,
                        &mut glob.max_local_id,
                        true,
                        true,
                        false,
                    )?;
                    y.push(v);
                }
            };

            copy_msg_tag_fd(msg, dst, from_channel)?;
            Ok(ProcMsg::Done)
        }
        (
            WaylandInterface::ZwlrGammaControlManagerV1,
            OPCODE_ZWLR_GAMMA_CONTROL_MANAGER_V1_GET_GAMMA_CONTROL,
        ) => {
            check_space!(msg.len(), 0, remaining_space);

            let (gamma, _output) = parse_req_zwlr_gamma_control_manager_v1_get_gamma_control(msg)?;
            insert_new_object(
                &mut glob.objects,
                gamma,
                WpObject {
                    obj_type: WaylandInterface::ZwlrGammaControlV1,
                    extra: WpExtra::ZwlrGammaControl(Box::new(ObjZwlrGammaControl {
                        gamma_size: None,
                    })),
                },
            )?;

            copy_msg(msg, dst);
            Ok(ProcMsg::Done)
        }
        (WaylandInterface::ZwlrGammaControlV1, OPCODE_ZWLR_GAMMA_CONTROL_V1_GAMMA_SIZE) => {
            check_space!(msg.len(), 0, remaining_space);
            let WpExtra::ZwlrGammaControl(ref mut gamma) = obj.extra else {
                return Err(tag!(
                    "Object produced through unexpected method, missing associated state"
                ));
            };
            let gamma_size = parse_evt_zwlr_gamma_control_v1_gamma_size(msg)?;
            if gamma_size > u32::MAX / 6 {
                return Err(tag!(
                    "Gamma size too large (ramps would use >u32::MAX bytes)"
                ));
            }
            gamma.gamma_size = Some(gamma_size);

            copy_msg(msg, dst);
            Ok(ProcMsg::Done)
        }
        (WaylandInterface::ZwlrGammaControlV1, OPCODE_ZWLR_GAMMA_CONTROL_V1_SET_GAMMA) => {
            check_space!(msg.len(), 1, remaining_space);
            let WpExtra::ZwlrGammaControl(ref gamma) = obj.extra else {
                return Err(tag!(
                    "Object produced through unexpected method, missing associated state"
                ));
            };
            let Some(gamma_size) = gamma.gamma_size else {
                return Err(tag!(
                    "zwlr_gamma_control_v1::set_gamma called before gamma size provided"
                ));
            };

            if let Some(wait) =
                translate_or_wait_for_fixed_file(transl, glob, gamma_size.checked_mul(6).unwrap())?
            {
                return Ok(wait);
            }
            copy_msg_tag_fd(msg, dst, from_channel)?;

            Ok(ProcMsg::Done)
        }
        (
            WaylandInterface::ZwpPrimarySelectionSourceV1,
            OPCODE_ZWP_PRIMARY_SELECTION_SOURCE_V1_SEND,
        )
        | (
            WaylandInterface::ZwpPrimarySelectionOfferV1,
            OPCODE_ZWP_PRIMARY_SELECTION_OFFER_V1_RECEIVE,
        )
        | (WaylandInterface::GtkPrimarySelectionSource, OPCODE_GTK_PRIMARY_SELECTION_SOURCE_SEND)
        | (
            WaylandInterface::GtkPrimarySelectionOffer,
            OPCODE_GTK_PRIMARY_SELECTION_OFFER_RECEIVE,
        )
        | (WaylandInterface::ExtDataControlSourceV1, OPCODE_EXT_DATA_CONTROL_SOURCE_V1_SEND)
        | (WaylandInterface::ExtDataControlOfferV1, OPCODE_EXT_DATA_CONTROL_OFFER_V1_RECEIVE)
        | (WaylandInterface::ZwlrDataControlSourceV1, OPCODE_ZWLR_DATA_CONTROL_SOURCE_V1_SEND)
        | (WaylandInterface::ZwlrDataControlOfferV1, OPCODE_ZWLR_DATA_CONTROL_OFFER_V1_RECEIVE)
        | (WaylandInterface::WlDataSource, OPCODE_WL_DATA_SOURCE_SEND)
        | (WaylandInterface::WlDataOffer, OPCODE_WL_DATA_OFFER_RECEIVE) => {
            check_space!(msg.len(), 1, remaining_space);

            /* Message format is mimetype + fd, and the mimetype doesn't matter
             * for Waypipe */

            match transl {
                TranslationInfo::FromChannel((x, y)) => {
                    let sfd = x.pop_front().ok_or_else(|| tag!("Missing sfd"))?;
                    y.push_back(sfd);
                }
                TranslationInfo::FromWayland((x, y)) => {
                    let v = translate_pipe_fd(
                        x.pop_front().ok_or_else(|| tag!("Missing fd"))?,
                        glob,
                        true, // reading from channel
                    )?;
                    y.push(v);
                }
            };

            copy_msg_tag_fd(msg, dst, from_channel)?;

            Ok(ProcMsg::Done)
        }
        (
            WaylandInterface::ZwlrScreencopyManagerV1,
            OPCODE_ZWLR_SCREENCOPY_MANAGER_V1_CAPTURE_OUTPUT_REGION,
        ) => {
            check_space!(msg.len(), 0, remaining_space);
            let (frame, _overlay_cursor, _output, _x, _y, _w, _h) =
                parse_req_zwlr_screencopy_manager_v1_capture_output_region(msg)?;
            insert_new_object(
                &mut glob.objects,
                frame,
                WpObject {
                    obj_type: WaylandInterface::ZwlrScreencopyFrameV1,
                    extra: WpExtra::ZwlrScreencopyFrame(Box::new(ObjZwlrScreencopyFrame {
                        buffer: None,
                    })),
                },
            )?;
            copy_msg(msg, dst);
            Ok(ProcMsg::Done)
        }
        (
            WaylandInterface::ZwlrScreencopyManagerV1,
            OPCODE_ZWLR_SCREENCOPY_MANAGER_V1_CAPTURE_OUTPUT,
        ) => {
            check_space!(msg.len(), 0, remaining_space);
            let (frame, _overlay_cursor, _output) =
                parse_req_zwlr_screencopy_manager_v1_capture_output(msg)?;
            insert_new_object(
                &mut glob.objects,
                frame,
                WpObject {
                    obj_type: WaylandInterface::ZwlrScreencopyFrameV1,
                    extra: WpExtra::ZwlrScreencopyFrame(Box::new(ObjZwlrScreencopyFrame {
                        buffer: None,
                    })),
                },
            )?;
            copy_msg(msg, dst);
            Ok(ProcMsg::Done)
        }
        (
            WaylandInterface::ExtImageCopyCaptureManagerV1,
            OPCODE_EXT_IMAGE_COPY_CAPTURE_MANAGER_V1_CREATE_SESSION,
        ) => {
            check_space!(msg.len(), 0, remaining_space);
            let (frame, _output, _options) =
                parse_req_ext_image_copy_capture_manager_v1_create_session(msg)?;
            insert_new_object(
                &mut glob.objects,
                frame,
                WpObject {
                    obj_type: WaylandInterface::ExtImageCopyCaptureSessionV1,
                    extra: WpExtra::ExtImageCopyCaptureSession(Box::new(
                        ObjExtImageCopyCaptureSession {
                            dmabuf_device: None,
                            dmabuf_formats: Vec::new(),
                            last_format_mod_list: Vec::new(),
                            frame_list: Vec::new(),
                        },
                    )),
                },
            )?;
            copy_msg(msg, dst);
            Ok(ProcMsg::Done)
        }
        (
            WaylandInterface::ExtImageCopyCaptureCursorSessionV1,
            OPCODE_EXT_IMAGE_COPY_CAPTURE_CURSOR_SESSION_V1_GET_CAPTURE_SESSION,
        ) => {
            check_space!(msg.len(), 0, remaining_space);
            let frame =
                parse_req_ext_image_copy_capture_cursor_session_v1_get_capture_session(msg)?;
            insert_new_object(
                &mut glob.objects,
                frame,
                WpObject {
                    obj_type: WaylandInterface::ExtImageCopyCaptureSessionV1,
                    extra: WpExtra::ExtImageCopyCaptureSession(Box::new(
                        ObjExtImageCopyCaptureSession {
                            dmabuf_device: None,
                            dmabuf_formats: Vec::new(),
                            last_format_mod_list: Vec::new(),
                            frame_list: Vec::new(),
                        },
                    )),
                },
            )?;
            copy_msg(msg, dst);
            Ok(ProcMsg::Done)
        }
        (
            WaylandInterface::ExtImageCopyCaptureSessionV1,
            OPCODE_EXT_IMAGE_COPY_CAPTURE_SESSION_V1_DMABUF_DEVICE,
        ) => {
            check_space!(msg.len(), 0, remaining_space);
            if glob.opts.no_gpu {
                return Ok(ProcMsg::Done);
            }
            let WpExtra::ExtImageCopyCaptureSession(ref mut session) = obj.extra else {
                return Err(tag!(
                    "Object produced through unexpected method, missing associated state"
                ));
            };

            let dev = parse_evt_ext_image_copy_capture_session_v1_dmabuf_device(msg)?;
            let main_device = parse_dev_array(dev)
                .ok_or_else(|| tag!("Unexpected size for dev_t: {}", dev.len()))?;
            session.dmabuf_device = Some(main_device);

            /* Drop message; will be recreated when ::done arrives */
            Ok(ProcMsg::Done)
        }
        (
            WaylandInterface::ExtImageCopyCaptureSessionV1,
            OPCODE_EXT_IMAGE_COPY_CAPTURE_SESSION_V1_DMABUF_FORMAT,
        ) => {
            check_space!(msg.len(), 0, remaining_space);
            if glob.opts.no_gpu {
                return Ok(ProcMsg::Done);
            }
            let WpExtra::ExtImageCopyCaptureSession(ref mut session) = obj.extra else {
                return Err(tag!(
                    "Object produced through unexpected method, missing associated state"
                ));
            };
            let (fmt, modifiers) = parse_evt_ext_image_copy_capture_session_v1_dmabuf_format(msg)?;
            let mut mod_list = Vec::new();
            for mb in modifiers.chunks_exact(std::mem::size_of::<u64>()) {
                let m = u64::from_le_bytes(mb.try_into().unwrap());
                mod_list.push(m);
            }
            if !mod_list.is_empty() {
                session.dmabuf_formats.push((fmt, mod_list));
            }
            /* Drop message; will be recreated when ::done arrives */
            Ok(ProcMsg::Done)
        }

        (
            WaylandInterface::ExtImageCopyCaptureSessionV1,
            OPCODE_EXT_IMAGE_COPY_CAPTURE_SESSION_V1_DONE,
        ) => {
            /* Replay messages */
            let WpExtra::ExtImageCopyCaptureSession(ref mut session) = obj.extra else {
                return Err(tag!(
                    "Object produced through unexpected method, missing associated state"
                ));
            };

            let mut space_needed = length_evt_ext_image_copy_capture_session_v1_done();
            if let Some(main_device) = session.dmabuf_device {
                let dev: Option<u64> = if matches!(
                    glob.dmabuf_device,
                    DmabufDevice::Unknown | DmabufDevice::VulkanSetup(_)
                ) {
                    /* Identify which device to use, if needed */
                    if glob.on_display_side {
                        Some(main_device)
                    } else if let Some(node) = &glob.opts.drm_node {
                        Some(get_dev_for_drm_node_path(node)?)
                    } else {
                        None
                    }
                } else {
                    None
                };

                match glob.dmabuf_device {
                    DmabufDevice::Unknown => {
                        glob.dmabuf_device = try_setup_dmabuf_instance_full(&glob.opts, dev)?;
                    }
                    DmabufDevice::VulkanSetup(_) => {
                        complete_dmabuf_setup(&glob.opts, dev, &mut glob.dmabuf_device)?;
                    }
                    _ => (),
                }

                if matches!(glob.dmabuf_device, DmabufDevice::Unavailable) {
                    return Err(tag!(
                        "DMABUF device specified, but DMABUFs are not supported"
                    ));
                }
                let current_device_id = dmabuf_dev_get_id(&glob.dmabuf_device);
                if glob.on_display_side && main_device != current_device_id {
                    // todo: handle this case
                    return Err(tag!("image copy device did not match existing device; multiple devices are not yet supported"));
                }

                space_needed += length_evt_ext_image_copy_capture_session_v1_dmabuf_device(
                    write_dev_array(current_device_id).len(),
                );
                for (fmt, mod_list) in session.dmabuf_formats.iter() {
                    let new_list_len = if glob.on_display_side {
                        mod_list
                            .iter()
                            .filter(|m| dmabuf_dev_supports_format(&glob.dmabuf_device, *fmt, **m))
                            .count()
                    } else {
                        dmabuf_dev_modifier_list(&glob.dmabuf_device, *fmt).len()
                    };
                    if new_list_len == 0 {
                        continue;
                    }
                    space_needed += length_evt_ext_image_copy_capture_session_v1_dmabuf_format(
                        new_list_len * std::mem::size_of::<u64>(),
                    );
                }
            }

            check_space!(space_needed, 0, remaining_space);

            if session.dmabuf_device.is_some() {
                let current_device_id = dmabuf_dev_get_id(&glob.dmabuf_device);
                write_evt_ext_image_copy_capture_session_v1_dmabuf_device(
                    dst,
                    object_id,
                    write_dev_array(current_device_id).as_slice(),
                );

                for (fmt, mod_list) in session.dmabuf_formats.iter() {
                    let mut output = Vec::new();
                    if glob.on_display_side {
                        /* Filter list of available modifiers */
                        for m in mod_list.iter() {
                            if dmabuf_dev_supports_format(&glob.dmabuf_device, *fmt, *m) {
                                output.extend_from_slice(&u64::to_le_bytes(*m));
                                add_advertised_modifiers(
                                    &mut glob.advertised_modifiers,
                                    *fmt,
                                    &[*m],
                                );
                            }
                        }
                    } else {
                        /* Replace modifier list with what is available locally */
                        let local_mods = dmabuf_dev_modifier_list(&glob.dmabuf_device, *fmt);
                        add_advertised_modifiers(&mut glob.advertised_modifiers, *fmt, local_mods);
                        for m in local_mods {
                            output.extend_from_slice(&u64::to_le_bytes(*m));
                        }
                    }
                    if output.is_empty() {
                        continue;
                    }

                    write_evt_ext_image_copy_capture_session_v1_dmabuf_format(
                        dst, object_id, *fmt, &output,
                    );
                }

                let mut format_mod_list = Vec::new();
                for (fmt, mods) in session.dmabuf_formats.iter() {
                    for m in mods.iter() {
                        format_mod_list.push((*fmt, *m))
                    }
                }
                format_mod_list.sort_unstable();
                session.last_format_mod_list = format_mod_list;

                if glob.on_display_side {
                    for (fmt, mods) in session.dmabuf_formats.iter() {
                        glob.screencopy_restrictions.insert(*fmt, mods.clone());
                    }
                }
            } else {
                session.last_format_mod_list = Vec::new();
            }
            write_evt_ext_image_copy_capture_session_v1_done(dst, object_id);

            /* Reset state: formats do not persist between ::done calls, because
             * ext_image_copy_capture_session_v1 has no event to remove a format;
             * presumably the dmabuf_device behaves similarly. */
            session.dmabuf_device = None;
            session.dmabuf_formats = Vec::new();
            Ok(ProcMsg::Done)
        }
        (
            WaylandInterface::ExtImageCopyCaptureSessionV1,
            OPCODE_EXT_IMAGE_COPY_CAPTURE_SESSION_V1_CREATE_FRAME,
        ) => {
            check_space!(msg.len(), 0, remaining_space);

            let WpExtra::ExtImageCopyCaptureSession(ref session) = obj.extra else {
                return Err(tag!(
                    "Object produced through unexpected method, missing associated state"
                ));
            };
            let supported_modifiers = session.last_format_mod_list.clone();

            let frame = parse_req_ext_image_copy_capture_session_v1_create_frame(msg)?;
            insert_new_object(
                &mut glob.objects,
                frame,
                WpObject {
                    obj_type: WaylandInterface::ExtImageCopyCaptureFrameV1,
                    extra: WpExtra::ExtImageCopyCaptureFrame(Box::new(
                        ObjExtImageCopyCaptureFrame {
                            buffer: None,
                            supported_modifiers,
                            capture_session: Some(object_id),
                        },
                    )),
                },
            )?;
            copy_msg(msg, dst);
            Ok(ProcMsg::Done)
        }
        (
            WaylandInterface::ExtImageCopyCaptureSessionV1,
            OPCODE_EXT_IMAGE_COPY_CAPTURE_SESSION_V1_DESTROY,
        ) => {
            check_space!(msg.len(), 0, remaining_space);
            copy_msg(msg, dst);

            let WpExtra::ExtImageCopyCaptureSession(ref mut session) = obj.extra else {
                return Err(tag!(
                    "Object produced through unexpected method, missing associated state"
                ));
            };
            let mut frames = Vec::new();
            std::mem::swap(&mut session.frame_list, &mut frames);
            let last_format_mod_list = session.last_format_mod_list.clone();
            for frame_id in frames {
                let object = glob.objects.get_mut(&frame_id).unwrap();
                let WpExtra::ExtImageCopyCaptureFrame(ref mut frame) = object.extra else {
                    return Err(tag!(
                        "Object produced through unexpected method, missing associated state"
                    ));
                };
                frame.capture_session = None;
                frame.supported_modifiers = last_format_mod_list.clone();
            }
            Ok(ProcMsg::Done)
        }

        (WaylandInterface::ZwlrScreencopyFrameV1, OPCODE_ZWLR_SCREENCOPY_FRAME_V1_COPY) => {
            check_space!(msg.len(), 0, remaining_space);
            copy_msg(msg, dst);

            let buffer = parse_req_zwlr_screencopy_frame_v1_copy(msg)?;
            if buffer.0 == 0 {
                return Err(tag!(
                    "zwlr_screencopy_frame_v1::copy requires non-null object"
                ));
            }
            let buf_obj = glob.objects.get(&buffer).ok_or_else(|| {
                tag!(
                    "Failed to lookup buffer (id {}) for zwlr_screencopy_frame_v1::copy",
                    buffer
                )
            })?;
            let WpExtra::WlBuffer(ref d) = buf_obj.extra else {
                return Err(tag!("Expected wl_buffer object"));
            };
            let buf_info = (d.sfd.clone(), d.shm_info);

            let object = glob.objects.get_mut(&object_id).unwrap();
            let WpExtra::ZwlrScreencopyFrame(ref mut frame) = object.extra else {
                return Err(tag!(
                    "Object produced through unexpected method, missing associated state"
                ));
            };
            frame.buffer = Some(buf_info);
            Ok(ProcMsg::Done)
        }
        (
            WaylandInterface::ZwlrScreencopyFrameV1,
            OPCODE_ZWLR_SCREENCOPY_FRAME_V1_COPY_WITH_DAMAGE,
        ) => {
            check_space!(msg.len(), 0, remaining_space);
            copy_msg(msg, dst);

            let buffer = parse_req_zwlr_screencopy_frame_v1_copy_with_damage(msg)?;
            if buffer.0 == 0 {
                return Err(tag!(
                    "zwlr_screencopy_frame_v1::copy requires non-null object"
                ));
            }
            let buf_obj = glob.objects.get(&buffer).ok_or_else(|| {
                tag!(
                    "Failed to lookup buffer (id {}) for zwlr_screencopy_frame_v1::copy",
                    buffer
                )
            })?;
            let WpExtra::WlBuffer(ref d) = buf_obj.extra else {
                return Err(tag!("Expected wl_buffer object"));
            };
            let buf_info = (d.sfd.clone(), d.shm_info);

            let object = glob.objects.get_mut(&object_id).unwrap();
            let WpExtra::ZwlrScreencopyFrame(ref mut frame) = object.extra else {
                return Err(tag!(
                    "Object produced through unexpected method, missing associated state"
                ));
            };
            frame.buffer = Some(buf_info);
            Ok(ProcMsg::Done)
        }

        (
            WaylandInterface::ExtImageCopyCaptureFrameV1,
            OPCODE_EXT_IMAGE_COPY_CAPTURE_FRAME_V1_DESTROY,
        ) => {
            check_space!(msg.len(), 0, remaining_space);
            copy_msg(msg, dst);

            let WpExtra::ExtImageCopyCaptureFrame(ref mut frame) = obj.extra else {
                return Err(tag!(
                    "Object produced through unexpected method, missing associated state"
                ));
            };
            let mut session: Option<ObjId> = None;
            std::mem::swap(&mut frame.capture_session, &mut session);

            if let Some(session_id) = session {
                let object = glob.objects.get_mut(&session_id).unwrap();
                let WpExtra::ExtImageCopyCaptureSession(ref mut session) = object.extra else {
                    return Err(tag!(
                        "Object produced through unexpected method, missing associated state"
                    ));
                };
                if let Some(i) = session.frame_list.iter().position(|x| *x == object_id) {
                    session.frame_list.remove(i);
                }
            }
            Ok(ProcMsg::Done)
        }
        (
            WaylandInterface::ExtImageCopyCaptureFrameV1,
            OPCODE_EXT_IMAGE_COPY_CAPTURE_FRAME_V1_ATTACH_BUFFER,
        ) => {
            check_space!(msg.len(), 0, remaining_space);
            copy_msg(msg, dst);

            let buffer = parse_req_ext_image_copy_capture_frame_v1_attach_buffer(msg)?;
            if buffer.0 == 0 {
                return Err(tag!(
                    "ext_image_copy_capture_frame_v1::attach_buffer requires non-null object"
                ));
            }
            let buf_obj = glob.objects.get(&buffer).ok_or_else(|| {
                tag!(
                    "Failed to lookup buffer (id {}) for ext_image_copy_capture_frame_v1::attach_buffer",
                     buffer
                )
            })?;
            let WpExtra::WlBuffer(ref d) = buf_obj.extra else {
                return Err(tag!("Expected wl_buffer object"));
            };
            let buf_info = (d.sfd.clone(), d.shm_info);

            let object = glob.objects.get_mut(&object_id).unwrap();
            let WpExtra::ExtImageCopyCaptureFrame(ref mut frame) = object.extra else {
                return Err(tag!(
                    "Object produced through unexpected method, missing associated state"
                ));
            };
            frame.buffer = Some(buf_info);
            Ok(ProcMsg::Done)
        }

        (
            WaylandInterface::ExtImageCopyCaptureFrameV1,
            OPCODE_EXT_IMAGE_COPY_CAPTURE_FRAME_V1_CAPTURE,
        ) => {
            check_space!(msg.len(), 0, remaining_space);
            copy_msg(msg, dst);

            if glob.on_display_side {
                let WpExtra::ExtImageCopyCaptureFrame(ref frame) = obj.extra else {
                    return Err(tag!(
                        "Object produced through unexpected method, missing associated state"
                    ));
                };
                /* Warn if the buffer being submitted does not have a format/modifier pair in the
                 * buffer constraints list; Waypipe currently does not have a mechanism to reliably
                 * ensure this occurs. */
                let fmtmod = if let Some(ref buffer) = frame.buffer {
                    let b = buffer.0.borrow();
                    if let ShadowFdVariant::Dmabuf(ref d) = b.data {
                        Some((d.drm_format, d.drm_modifier))
                    } else {
                        None
                    }
                } else {
                    None
                };

                if let Some(pair) = fmtmod {
                    let err = if let Some(session_id) = frame.capture_session {
                        let object = glob.objects.get_mut(&session_id).unwrap();
                        let WpExtra::ExtImageCopyCaptureSession(ref session) = object.extra else {
                            return Err(tag!("Object produced through unexpected method, missing associated state"));
                        };
                        session.last_format_mod_list.binary_search(&pair).is_err()
                    } else {
                        frame.supported_modifiers.binary_search(&pair).is_err()
                    };
                    if err {
                        error!("A wl_buffer Waypipe created with (format, modifier) = (0x{:08x},0x{:016x}) \
                            is being submitted to ext_image_copy_capture_frame_v1#{}, whose parent ext_image_copy_capture_session_v1's \
                            most recent update did not include the (format, modifier) combination. This may be a known Waypipe issue.",
                            pair.0, pair.1, object_id);
                    }
                }
            }
            Ok(ProcMsg::Done)
        }
        (WaylandInterface::ZwlrScreencopyFrameV1, OPCODE_ZWLR_SCREENCOPY_FRAME_V1_READY) => {
            check_space!(msg.len(), 0, remaining_space);
            let WpExtra::ZwlrScreencopyFrame(ref mut frame) = obj.extra else {
                return Err(tag!(
                    "Object produced through unexpected method, missing associated state"
                ));
            };

            let Some((ref sfd, ref shm_info)) = frame.buffer else {
                return Err(tag!(
                    "zwlr_screencopy_frame_v1::ready is missing buffer information"
                ));
            };

            if !glob.on_display_side {
                let b = sfd.borrow();
                let apply_count = if let ShadowFdVariant::File(data) = &b.data {
                    data.pending_apply_tasks
                } else if let ShadowFdVariant::Dmabuf(data) = &b.data {
                    /* Assuming no timelines for screencopy-frame-v1 */
                    data.pending_apply_tasks
                } else {
                    return Err(tag!("Attached buffer is not of file or dmabuf type"));
                };
                if apply_count > 0 {
                    return Ok(ProcMsg::WaitFor(b.remote_id));
                }
            }

            let (tv_sec_hi, tv_sec_lo, tv_nsec) = parse_evt_zwlr_screencopy_frame_v1_ready(msg)?;
            let (new_sec_hi, new_sec_lo, new_nsec) = translate_timestamp(
                tv_sec_hi,
                tv_sec_lo,
                tv_nsec,
                libc::CLOCK_MONOTONIC as u32,
                glob.on_display_side,
            )?;
            write_evt_zwlr_screencopy_frame_v1_ready(
                dst, object_id, new_sec_hi, new_sec_lo, new_nsec,
            );

            if !glob.on_display_side {
                let mut sfd = sfd.borrow_mut();
                if let ShadowFdVariant::Dmabuf(ref mut y) = &mut sfd.data {
                    dmabuf_post_apply_task_operations(y)?;
                }
            }

            if glob.on_display_side {
                /* Mark damage */

                let mut sfd = sfd.borrow_mut();
                if let ShadowFdVariant::File(ref mut y) = &mut sfd.data {
                    let damage_interval = damage_for_entire_buffer(shm_info.as_ref().unwrap());
                    match &y.damage {
                        Damage::Everything => {}
                        Damage::Intervals(old) => {
                            let dmg = &[damage_interval];
                            y.damage = Damage::Intervals(union_damage(&old[..], &dmg[..], 128));
                        }
                    }
                } else if let ShadowFdVariant::Dmabuf(ref mut y) = &mut sfd.data {
                    y.damage = Damage::Everything;

                    y.using_implicit_sync = true;
                } else {
                    return Err(tag!("Expected buffer shadowfd to be of file type"));
                }
            }
            frame.buffer = None;
            Ok(ProcMsg::Done)
        }
        (
            WaylandInterface::ExtImageCopyCaptureFrameV1,
            OPCODE_EXT_IMAGE_COPY_CAPTURE_FRAME_V1_PRESENTATION_TIME,
        ) => {
            check_space!(msg.len(), 0, remaining_space);

            let (tv_sec_hi, tv_sec_lo, tv_nsec) =
                parse_evt_ext_image_copy_capture_frame_v1_presentation_time(msg)?;
            let (new_sec_hi, new_sec_lo, new_nsec) = translate_timestamp(
                tv_sec_hi,
                tv_sec_lo,
                tv_nsec,
                libc::CLOCK_MONOTONIC as u32,
                glob.on_display_side,
            )?;
            write_evt_ext_image_copy_capture_frame_v1_presentation_time(
                dst, object_id, new_sec_hi, new_sec_lo, new_nsec,
            );
            Ok(ProcMsg::Done)
        }

        (
            WaylandInterface::ExtImageCopyCaptureFrameV1,
            OPCODE_EXT_IMAGE_COPY_CAPTURE_FRAME_V1_READY,
        ) => {
            // TODO: deduplicate with wlr_screencopy_frame_v1::ready
            check_space!(msg.len(), 0, remaining_space);
            let WpExtra::ExtImageCopyCaptureFrame(ref mut frame) = obj.extra else {
                return Err(tag!(
                    "Object produced through unexpected method, missing associated state"
                ));
            };

            let Some((ref sfd, ref shm_info)) = frame.buffer else {
                return Err(tag!(
                    "zwlr_screencopy_frame_v1::ready is missing buffer information"
                ));
            };

            if !glob.on_display_side {
                let b = sfd.borrow();
                let apply_count = if let ShadowFdVariant::File(data) = &b.data {
                    data.pending_apply_tasks
                } else if let ShadowFdVariant::Dmabuf(data) = &b.data {
                    /* Assuming no timelines for screencopy-frame-v1 */
                    data.pending_apply_tasks
                } else {
                    return Err(tag!("Attached buffer is not of file or dmabuf type"));
                };
                if apply_count > 0 {
                    return Ok(ProcMsg::WaitFor(b.remote_id));
                }
            }

            copy_msg(msg, dst);

            if !glob.on_display_side {
                let mut sfd = sfd.borrow_mut();
                if let ShadowFdVariant::Dmabuf(ref mut y) = &mut sfd.data {
                    dmabuf_post_apply_task_operations(y)?;
                }
            }

            if glob.on_display_side {
                /* Mark damage */

                let mut sfd = sfd.borrow_mut();
                if let ShadowFdVariant::File(ref mut y) = &mut sfd.data {
                    let damage_interval = damage_for_entire_buffer(shm_info.as_ref().unwrap());
                    match &y.damage {
                        Damage::Everything => {}
                        Damage::Intervals(old) => {
                            let dmg = &[damage_interval];
                            y.damage = Damage::Intervals(union_damage(&old[..], &dmg[..], 128));
                        }
                    }
                } else if let ShadowFdVariant::Dmabuf(ref mut y) = &mut sfd.data {
                    y.using_implicit_sync = true;
                    y.damage = Damage::Everything;
                } else {
                    return Err(tag!("Expected buffer shadowfd to be of file type"));
                }
            }
            frame.buffer = None;
            Ok(ProcMsg::Done)
        }
        (WaylandInterface::ZwlrScreencopyFrameV1, OPCODE_ZWLR_SCREENCOPY_FRAME_V1_FAILED) => {
            check_space!(msg.len(), 0, remaining_space);
            copy_msg(msg, dst);
            let WpExtra::ZwlrScreencopyFrame(ref mut frame) = obj.extra else {
                return Err(tag!(
                    "Object produced through unexpected method, missing associated state"
                ));
            };
            frame.buffer = None;
            Ok(ProcMsg::Done)
        }
        (
            WaylandInterface::ExtImageCopyCaptureFrameV1,
            OPCODE_EXT_IMAGE_COPY_CAPTURE_FRAME_V1_FAILED,
        ) => {
            check_space!(msg.len(), 0, remaining_space);
            copy_msg(msg, dst);
            let WpExtra::ExtImageCopyCaptureFrame(ref mut frame) = obj.extra else {
                return Err(tag!(
                    "Object produced through unexpected method, missing associated state"
                ));
            };
            frame.buffer = None;
            Ok(ProcMsg::Done)
        }
        (WaylandInterface::XdgToplevel, OPCODE_XDG_TOPLEVEL_SET_TITLE) => {
            let title = parse_req_xdg_toplevel_set_title(msg)?;
            let prefix = glob.opts.title_prefix.as_bytes();

            let space_needed = length_req_xdg_toplevel_set_title(title.len() + prefix.len());
            check_space!(space_needed, 0, remaining_space);

            // TODO: direct manipulation is appropriate here, because the output
            // already provides the necessary space
            let mut concat: Vec<u8> = Vec::new();
            concat.extend_from_slice(prefix);
            concat.extend_from_slice(title);
            write_req_xdg_toplevel_set_title(dst, object_id, &concat);
            Ok(ProcMsg::Done)
        }
        (WaylandInterface::XdgToplevelIconV1, OPCODE_XDG_TOPLEVEL_ICON_V1_ADD_BUFFER) => {
            check_space!(msg.len(), 0, remaining_space);

            let (buffer_id, _scale) = parse_req_xdg_toplevel_icon_v1_add_buffer(msg)?;

            let Some(buffer) = glob.objects.get(&buffer_id) else {
                return Err(tag!(
                    "Provided buffer is null, was never created, or is not tracked"
                ));
            };
            let WpExtra::WlBuffer(ref extra) = buffer.extra else {
                return Err(tag!("Expected wl_buffer object"));
            };

            /* This request makes the current buffer contents available to the compositor. */
            if glob.on_display_side {
                let b = extra.sfd.borrow();
                // Only wl_shm buffers are allowed for xdg_toplevel_icon_v1::add_buffer
                let apply_count = if let ShadowFdVariant::File(data) = &b.data {
                    data.pending_apply_tasks
                } else {
                    return Err(tag!("Attached buffer shadowfd is not of file type"));
                };
                if apply_count > 0 {
                    return Ok(ProcMsg::WaitFor(b.remote_id));
                }
            }

            copy_msg(msg, dst);

            if !glob.on_display_side {
                /* Mark entire buffer as damaged */
                let mut sfd = extra.sfd.borrow_mut();
                if let ShadowFdVariant::File(ref mut y) = &mut sfd.data {
                    let damage_interval =
                        damage_for_entire_buffer(extra.shm_info.as_ref().unwrap());
                    match &y.damage {
                        Damage::Everything => {}
                        Damage::Intervals(old) => {
                            let dmg = &[damage_interval];
                            y.damage = Damage::Intervals(union_damage(&old[..], &dmg[..], 128));
                        }
                    }
                } else {
                    return Err(tag!("Expected buffer shadowfd to be of file type"));
                }
            }
            Ok(ProcMsg::Done)
        }
        (WaylandInterface::WpPresentationFeedback, OPCODE_WP_PRESENTATION_FEEDBACK_PRESENTED) => {
            check_space!(msg.len(), 0, remaining_space);
            let (tv_sec_hi, tv_sec_lo, tv_nsec, refresh, seq_hi, seq_lo, flags) =
                parse_evt_wp_presentation_feedback_presented(msg)?;
            let clock_id = glob.presentation_clock.unwrap_or_else(|| {
                error!("wp_presentation_feedback::presented timestamp was received before any wp_presentation::clock event,\
                        so Waypipe assumes CLOCK_MONOTONIC was used and may misconvert times if wrong.");
                libc::CLOCK_MONOTONIC as u32
            }            );
            let (new_sec_hi, new_sec_lo, new_nsec) = translate_timestamp(
                tv_sec_hi,
                tv_sec_lo,
                tv_nsec,
                clock_id,
                glob.on_display_side,
            )?;
            write_evt_wp_presentation_feedback_presented(
                dst, object_id, new_sec_hi, new_sec_lo, new_nsec, refresh, seq_hi, seq_lo, flags,
            );
            Ok(ProcMsg::Done)
        }
        (WaylandInterface::WpPresentation, OPCODE_WP_PRESENTATION_CLOCK_ID) => {
            check_space!(msg.len(), 0, remaining_space);
            let clock_id = parse_evt_wp_presentation_clock_id(msg)?;
            if let Some(old) = glob.presentation_clock {
                if clock_id != old {
                    return Err(tag!(
                        "The wp_presentation clock was already set to {} and cannot be changed to {}.",
                        old,
                        clock_id
                    ));
                }
            }
            // note: in theory, `waypipe server` could choose a preferred clock of its
            // own (like CLOCK_REALTIME or CLOCK_TAI) to reduce the number of clock
            // conversions.
            glob.presentation_clock = Some(clock_id);
            copy_msg(msg, dst);
            Ok(ProcMsg::Done)
        }
        (WaylandInterface::WpCommitTimerV1, OPCODE_WP_COMMIT_TIMER_V1_SET_TIMESTAMP) => {
            check_space!(msg.len(), 0, remaining_space);
            let (tv_sec_hi, tv_sec_lo, tv_nsec) = parse_req_wp_commit_timer_v1_set_timestamp(msg)?;

            let clock_id = glob.presentation_clock.unwrap_or_else(|| {
                error!("wp_commit_timer_v1::set_timestamp was received before wp_presentation::clock,\
                        so Waypipe assumes CLOCK_MONOTONIC was used and may misconvert times if wrong.");
                libc::CLOCK_MONOTONIC as u32
            }            );
            let (new_sec_hi, new_sec_lo, new_nsec) = translate_timestamp(
                tv_sec_hi,
                tv_sec_lo,
                tv_nsec,
                clock_id,
                !glob.on_display_side,
            )?;
            write_req_wp_commit_timer_v1_set_timestamp(
                dst, object_id, new_sec_hi, new_sec_lo, new_nsec,
            );
            Ok(ProcMsg::Done)
        }
        (WaylandInterface::WlRegistry, OPCODE_WL_REGISTRY_GLOBAL) => {
            // filter out events
            let (name, intf, mut version) = parse_evt_wl_registry_global(msg)?;

            /* Note: nothing that gets filtered out should ever be removed,
             * so it is not necessary to track interface name codes for global_remove */
            let blacklist: &'static [&'static [u8]] = &[
                b"wl_drm", /* very old/deprecated API: linux-dmabuf-v4 replaces */
                b"wp_drm_lease_device_v1",
                b"zwlr_export_dmabuf_manager_v1",
                b"zwp_linux_explicit_synchronization_v1", // outdated, uses fences instead of timelines
                b"wp_security_context_manager_v1",        // sends socket listen fd over network
            ];

            /* Limit the version of the following protocols to what Waypipe has compiled
             * in, because future versions are very likely to require additional handling. */
            let intf_code = match intf {
                WL_SHM => Some(WaylandInterface::WlShm),
                ZWP_LINUX_DMABUF_V1 => Some(WaylandInterface::ZwpLinuxDmabufV1),
                WP_LINUX_DRM_SYNCOBJ_MANAGER_V1 => {
                    Some(WaylandInterface::WpLinuxDrmSyncobjManagerV1)
                }
                EXT_IMAGE_COPY_CAPTURE_MANAGER_V1 => {
                    Some(WaylandInterface::ExtImageCopyCaptureManagerV1)
                }
                ZWLR_SCREENCOPY_MANAGER_V1 => Some(WaylandInterface::ZwlrScreencopyManagerV1),
                ZWLR_EXPORT_DMABUF_MANAGER_V1 => Some(WaylandInterface::ZwlrExportDmabufManagerV1),
                _ => None,
            };
            if let Some(code) = intf_code {
                let max_v = INTERFACE_TABLE[code as usize].version;
                if version > max_v {
                    debug!(
                        "Downgrading {} version from {} to {}",
                        EscapeWlName(intf),
                        version,
                        max_v
                    );
                    version = max_v;
                }
            }
            if blacklist.contains(&intf) {
                /* Drop interface entirely */
                debug!("Dropping interface: {}", EscapeWlName(intf));
                return Ok(ProcMsg::Done);
            }

            if intf == ZWP_LINUX_DMABUF_V1 {
                /* waypipe-server side: Filter out dmabuf support if the target device (or _any_ device)
                 * is not available; this must be done now to prevent advertising this global when
                 * DMABUF support is not actually available. */
                match glob.dmabuf_device {
                    DmabufDevice::Unavailable => (), /* case handled later */
                    DmabufDevice::Vulkan(_) | DmabufDevice::Gbm(_) => (),
                    DmabufDevice::VulkanSetup(_) => (),
                    DmabufDevice::Unknown => {
                        if !glob.on_display_side {
                            let dev = if let Some(node) = &glob.opts.drm_node {
                                /* Pick specified device */
                                Some(get_dev_for_drm_node_path(node)?)
                            } else {
                                /* Pick best device */
                                None
                            };
                            glob.dmabuf_device = try_setup_dmabuf_instance_light(&glob.opts, dev)?;
                            assert!(!matches!(glob.dmabuf_device, DmabufDevice::Unknown));
                        }
                    }
                }
                if matches!(glob.dmabuf_device, DmabufDevice::Unavailable) {
                    debug!(
                        "No DMABUF handling device available: Dropping interface: {}",
                        EscapeWlName(intf)
                    );
                    return Ok(ProcMsg::Done);
                }

                /* note: with versions < 4, the the compositor has no way to specify the preferred
                 * drm node, so it may be chosen arbitrarily */
            }
            if intf == WP_LINUX_DRM_SYNCOBJ_MANAGER_V1 {
                match &glob.dmabuf_device {
                    DmabufDevice::Unknown => {
                        /* store globals for replay later */
                        let WpExtra::WlRegistry(ref mut reg) = obj.extra else {
                            return Err(tag!("Unexpected extra type for wl_registry"));
                        };
                        reg.syncobj_manager_replay.push((name, version));
                    }
                    DmabufDevice::Gbm(_) | DmabufDevice::Unavailable => {
                        /* drop, not supported */
                        debug!(
                            "No timeline semaphore handling device available: Dropping interface: {}",
                            EscapeWlName(intf)
                        );
                        return Ok(ProcMsg::Done);
                    }
                    DmabufDevice::VulkanSetup(vkinst) => {
                        let dev = if let Some(node) = &glob.opts.drm_node {
                            Some(get_dev_for_drm_node_path(node)?)
                        } else {
                            None
                        };
                        if !vkinst.device_supports_timeline_import_export(dev) {
                            debug!(
                                "Timeline semaphore import/export will not be supported: Dropping interface: {}",
                                EscapeWlName(intf)
                            );
                            return Ok(ProcMsg::Done);
                        }
                    }
                    DmabufDevice::Vulkan((_, vulk)) => {
                        /* Keep if timeline semaphores supported */
                        if !vulk.supports_timeline_import_export() {
                            debug!(
                                "Timeline semaphore import/export is not supported: Dropping interface: {}",
                                EscapeWlName(intf)
                            );
                            return Ok(ProcMsg::Done);
                        }
                    }
                }
            }

            let mut space = msg.len();
            if intf == ZWP_LINUX_DMABUF_V1 {
                let WpExtra::WlRegistry(ref mut reg) = obj.extra else {
                    return Err(tag!("Unexpected extra type for wl_registry"));
                };
                if !reg.syncobj_manager_replay.is_empty() {
                    space += length_evt_wl_registry_global(WP_LINUX_DRM_SYNCOBJ_MANAGER_V1.len())
                        * reg.syncobj_manager_replay.len();
                }
            }

            check_space!(space, 0, remaining_space);
            write_evt_wl_registry_global(dst, object_id, name, intf, version);

            if intf == ZWP_LINUX_DMABUF_V1 {
                /* Replay syncobj manager events once it is certain Vulkan is available. */
                let WpExtra::WlRegistry(ref mut reg) = obj.extra else {
                    return Err(tag!("Unexpected extra type for wl_registry"));
                };
                let timelines_supported = match &glob.dmabuf_device {
                    /* Assume true since, once set up, Waypipe will most likely support drm syncobj iff the compositor does */
                    DmabufDevice::VulkanSetup(_) => true,
                    DmabufDevice::Unknown => {
                        if glob.on_display_side {
                            true
                        } else {
                            unreachable!();
                        }
                    }
                    DmabufDevice::Unavailable => unreachable!(),
                    DmabufDevice::Gbm(_) => false,
                    DmabufDevice::Vulkan((_, vulk)) => vulk.supports_timeline_import_export(),
                };
                if !timelines_supported && !reg.syncobj_manager_replay.is_empty() {
                    debug!(
                        "Timeline semaphore import/export is not supported, not replaying {} advertisements for {}",
                        reg.syncobj_manager_replay.len(),
                        EscapeWlName(WP_LINUX_DRM_SYNCOBJ_MANAGER_V1)
                    );
                }

                for (sync_name, sync_version) in reg.syncobj_manager_replay.drain(..) {
                    if timelines_supported {
                        write_evt_wl_registry_global(
                            dst,
                            object_id,
                            sync_name,
                            WP_LINUX_DRM_SYNCOBJ_MANAGER_V1,
                            sync_version,
                        );
                    }
                }
            }

            Ok(ProcMsg::Done)
        }
        (WaylandInterface::WlRegistry, OPCODE_WL_REGISTRY_BIND) => {
            // filter out events
            let (_id, name, version, oid) = parse_req_wl_registry_bind(msg)?;
            if name == ZWP_LINUX_DMABUF_V1 {
                let light_setup = version >= 4 && glob.on_display_side;
                if matches!(glob.dmabuf_device, DmabufDevice::Unknown) {
                    let dev = if let Some(node) = &glob.opts.drm_node {
                        /* Pick specified device */
                        Some(get_dev_for_drm_node_path(node)?)
                    } else {
                        /* Pick best device */
                        None
                    };
                    if light_setup {
                        /* In this case, device will be provided later through dmabuf-feedback
                         * main_device event */
                        glob.dmabuf_device = try_setup_dmabuf_instance_light(&glob.opts, dev)?;
                    } else {
                        debug!(
                            "Client bound zwp_linux_dmabuf_v1 at version {} older than 4, using best-or-specified drm node",
                            version
                        );
                        glob.dmabuf_device = try_setup_dmabuf_instance_full(&glob.opts, dev)?;
                    }
                    assert!(!matches!(glob.dmabuf_device, DmabufDevice::Unknown));
                }
                if !light_setup && matches!(glob.dmabuf_device, DmabufDevice::VulkanSetup(_)) {
                    let dev = if let Some(node) = &glob.opts.drm_node {
                        Some(get_dev_for_drm_node_path(node)?)
                    } else {
                        None
                    };
                    complete_dmabuf_setup(&glob.opts, dev, &mut glob.dmabuf_device)?;
                }
                if matches!(glob.dmabuf_device, DmabufDevice::Unavailable) {
                    return Err(tag!("Failed to set up a device to handle DMABUFS"));
                }

                check_space!(msg.len(), 0, remaining_space);
                copy_msg(msg, dst);
                insert_new_object(
                    &mut glob.objects,
                    oid,
                    WpObject {
                        obj_type: WaylandInterface::ZwpLinuxDmabufV1,
                        extra: WpExtra::ZwpDmabuf(Box::new(ObjZwpLinuxDmabuf {
                            formats_seen: BTreeSet::new(),
                        })),
                    },
                )?;
                return Ok(ProcMsg::Done);
            }

            /* create new global objects */
            default_proc_way_msg(msg, dst, meth, is_req, object_id, glob)
        }

        _ => {
            // Default handling: copy message, and create IDs
            default_proc_way_msg(msg, dst, meth, is_req, object_id, glob)
        }
    }
}

/** Log the _changed_ messages in the given buffer whose corresponding object is
 * currently in the provided set of objects.
 *
 * To avoid issues resulting from object deletion, this should be called
 * promptly after processing a message and producing `output_msgs`.
 */
pub fn log_way_msg_output(
    orig_msg: &[u8],
    mut output_msgs: &[u8],
    objects: &BTreeMap<ObjId, WpObject>,
    is_req: bool,
) {
    if !log::log_enabled!(log::Level::Debug) {
        return;
    }

    if output_msgs.is_empty() {
        debug!("Dropped last {}", if is_req { "request" } else { "event" },);
        return;
    }
    if orig_msg[0..4] == output_msgs[0..4] && orig_msg[8..] == output_msgs[8..] {
        /* No change, only message was copied as is, modulo fd tagging changes */
        return;
    }

    /* Output messages should be well formed. */
    while !output_msgs.is_empty() {
        let object_id = ObjId(u32::from_le_bytes(output_msgs[0..4].try_into().unwrap()));
        let header2 = u32::from_le_bytes(output_msgs[4..8].try_into().unwrap());
        let length = (header2 >> 16) as usize;
        let opcode = (header2 & ((1 << 11) - 1)) as usize;
        let msg = &output_msgs[..length];
        output_msgs = &output_msgs[length..];

        let Some(obj) = objects.get(&object_id) else {
            /* Unknown messages will always be copied, so no point in logging them again */
            continue;
        };

        let opt_meth: Option<&WaylandMethod> = if is_req {
            INTERFACE_TABLE[obj.obj_type as usize].reqs.get(opcode)
        } else {
            INTERFACE_TABLE[obj.obj_type as usize].evts.get(opcode)
        };
        let Some(meth) = opt_meth else {
            /* Method out of range, will be copied */
            continue;
        };
        debug!(
            "Modified {}: {}#{}.{}({})",
            if is_req { "request" } else { "event" },
            INTERFACE_TABLE[obj.obj_type as usize].name,
            object_id,
            meth.name,
            MethodArguments { meth, msg }
        );
    }
}

/** Construct the Wayland object map with the initial object, wl_display#1 */
pub fn setup_object_map() -> BTreeMap<ObjId, WpObject> {
    let mut map = BTreeMap::new();
    map.insert(
        ObjId(1),
        WpObject {
            obj_type: WaylandInterface::WlDisplay,
            extra: WpExtra::None,
        },
    );
    map
}
