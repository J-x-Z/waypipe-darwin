/* SPDX-License-Identifier: GPL-3.0-or-later */
/*! Video encoding for dmabufs */
#![cfg(feature = "video")]
use crate::dmabuf::*;
use crate::tag;
use crate::util::*;
use crate::wayland_gen::*;
use ash::vk::Handle;
use ash::*;
use log::{debug, error};
use std::ffi::{c_char, c_int, CStr};
use std::ptr::slice_from_raw_parts;
use std::sync::{Arc, Mutex};
use waypipe_ffmpeg_wrapper::{
    ffmpeg, AVBufferRef, AVCodec, AVCodecContext, AVDictionary, AVFrame, AVHWDeviceContext,
    AVHWDeviceType_AV_HWDEVICE_TYPE_VULKAN, AVHWFramesContext, AVPacket,
    AVPixelFormat_AV_PIX_FMT_NONE, AVPixelFormat_AV_PIX_FMT_NV12, AVPixelFormat_AV_PIX_FMT_VULKAN,
    AVPixelFormat_AV_PIX_FMT_YUV420P, AVRational, AVVkFrame, AVVulkanDeviceContext,
    AVVulkanFramesContext, VkStructureType_VK_STRUCTURE_TYPE_PHYSICAL_DEVICE_FEATURES_2,
    AV_LOG_VERBOSE, AV_LOG_WARNING, AV_NUM_DATA_POINTERS, LIBAVCODEC_VERSION_MAJOR,
};
use waypipe_shaders::{NV12_IMG_TO_RGB, RGB_TO_NV12_IMG, RGB_TO_YUV420_BUF, YUV420_BUF_TO_RGB};

struct VulkanComputePipeline {
    shader_module: vk::ShaderModule,
    ds_layout: vk::DescriptorSetLayout,
    pipeline_layout: vk::PipelineLayout,
    pipeline: vk::Pipeline,
}

/** The preferred hardware and software decoders and encoders for a format.
 * Entries may be null if the codec is not available, and the same AVCodec may
 * be used for both software and hardware video.
 */
struct CodecSet {
    decoder: *const AVCodec,
    sw_decoder: *const AVCodec,
    encoder: *const AVCodec,
    sw_encoder: *const AVCodec,
}

/** Structure holding all video-related state (ffmpeg bindings, devices, pipelines, etc.)
 * linked to a specific Vulkan logical device. */
pub struct VulkanVideo {
    bindings: ffmpeg,
    av_hwdevice: *mut AVBufferRef, // to AVHWDeviceContext

    codecs_h264: CodecSet,
    codecs_vp9: CodecSet,
    codecs_av1: CodecSet,

    can_hw_enc_h264: bool,
    can_hw_dec_h264: bool,
    can_hw_enc_av1: bool,
    can_hw_dec_av1: bool,

    // TODO: is it possible to detect in advance when hardware en/decoding works?
    // and only create the necessary pipeline?
    nv12_img_to_rgb: VulkanComputePipeline,
    rgb_to_nv12_img: VulkanComputePipeline,
    yuv420_buf_to_rgb: VulkanComputePipeline,
    rgb_to_yuv420_buf: VulkanComputePipeline,

    yuv_to_rgb_sampler_y: vk::Sampler,
    yuv_to_rgb_sampler_rb: vk::Sampler,
    rgb_to_yuv_sampler_rgb: vk::Sampler,
}
unsafe impl Send for VulkanVideo {}
unsafe impl Sync for VulkanVideo {}

struct VulkanSWDecodeData {
    _buf_y: VulkanBuffer,
    _buf_u: VulkanBuffer,
    _buf_v: VulkanBuffer,
    buf_y_view: vk::BufferView,
    buf_u_view: vk::BufferView,
    buf_v_view: vk::BufferView,
}

struct VulkanHWDecodeData {}

enum VulkanDecodeOpData {
    Software(VulkanSWDecodeData),
    Hardware(VulkanHWDecodeData),
}

pub struct VulkanDecodeOpHandle {
    /* Copy operation is between these two objects */
    decode: Arc<VideoDecodeState>,
    pool: Arc<VulkanCommandPool>,

    // TODO: not safe to free a 'pending' command buffer; give Vulkan itself a list of copy-handles?
    cb: vk::CommandBuffer,
    desc_pool: vk::DescriptorPool,
    descriptor_set: vk::DescriptorSet,

    data: VulkanDecodeOpData,

    // on the main queue's timeline semaphore
    pub completion_time_point: u64,
}

impl Drop for VulkanDecodeOpHandle {
    fn drop(&mut self) {
        let cmd_pool = self.pool.pool.lock().unwrap();
        let vulk: &VulkanDevice = &self.decode.target.vulk;
        unsafe {
            /* Verify that the command buffer execution has completed; if not, panic, as it's a program error */
            if let Ok(counter) = vulk
                .timeline_semaphore
                .get_semaphore_counter_value(vulk.semaphore)
            {
                assert!(
                    counter >= self.completion_time_point,
                    "decode op handle deleted at {} >!= {}; dropped too early?",
                    counter,
                    self.completion_time_point
                );
            }
            vulk.dev.free_command_buffers(*cmd_pool, &[self.cb]);

            match self.data {
                VulkanDecodeOpData::Software(ref x) => {
                    vulk.dev.destroy_buffer_view(x.buf_y_view, None);
                    vulk.dev.destroy_buffer_view(x.buf_u_view, None);
                    vulk.dev.destroy_buffer_view(x.buf_v_view, None);
                    // VulkanBuffer has Drop impl
                }
                VulkanDecodeOpData::Hardware(_) => {}
            }

            vulk.dev
                .free_descriptor_sets(self.desc_pool, &[self.descriptor_set])
                .map_err(|_| "Failed to free descriptor set")
                .unwrap();
            vulk.dev.destroy_descriptor_pool(self.desc_pool, None);
        }
    }
}

impl Drop for VulkanVideo {
    fn drop(&mut self) {
        unsafe {
            let r = &mut self.av_hwdevice;
            self.bindings.av_buffer_unref(r);
        }
    }
}

unsafe fn destroy_compute_pipeline(dev: &Device, p: &VulkanComputePipeline) {
    dev.destroy_pipeline(p.pipeline, None);
    dev.destroy_pipeline_layout(p.pipeline_layout, None);
    dev.destroy_descriptor_set_layout(p.ds_layout, None);
    dev.destroy_shader_module(p.shader_module, None);
}

pub unsafe fn destroy_video(dev: &Device, video: &VulkanVideo) {
    /* Cleanup vulkan bits of video state */
    destroy_compute_pipeline(dev, &video.nv12_img_to_rgb);
    destroy_compute_pipeline(dev, &video.rgb_to_nv12_img);
    destroy_compute_pipeline(dev, &video.yuv420_buf_to_rgb);
    destroy_compute_pipeline(dev, &video.rgb_to_yuv420_buf);

    dev.destroy_sampler(video.yuv_to_rgb_sampler_y, None);
    dev.destroy_sampler(video.yuv_to_rgb_sampler_rb, None);

    dev.destroy_sampler(video.rgb_to_yuv_sampler_rgb, None);
}

struct VideoDecodeInner {
    ctx: *mut AVCodecContext,
}
unsafe impl Send for VideoDecodeInner {}

/** Hardware video decoding produces, and encoding uses, a multi-planar image with a
 * format like G8_B8R8_2PLANE_420_UNORM; however, these typically only have
 * TRANSFER and various SAMPLED format features, but not BLIT, STORAGE, or
 * COLOR_ATTACHMENT. (Note: STORAGE sometimes works in practice despite not being
 * supported; doing this may be faster but is risky and could lead to exciting bugs.)
 * Also,
 * Also, aliasing individual planes appears to require ALIAS|DISJOINT which Vulkan
 * video does not appear to always support (or if so, figuring it out and
 * integrating it with ffmpeg's code is complicated.) Therefore: To make encoding and
 * decoding more reliable and symmetric, transfer image data to/from a staging image
 * before doing the YUV<->RGB conversion.
 *
 * This does not hold a reference to the vulkan instance and cleaning up this struct
 * is the owner's responsibility.
 */
struct VideoHWStagingImages {
    plane_memories: [vk::DeviceMemory; 2],
    plane_images: [vk::Image; 2],
    plane_image_views: [vk::ImageView; 2],
}
enum VideoDecodeStateData {
    SW,
    HW(VideoHWStagingImages),
}

pub struct VideoDecodeState {
    // for now, only updating a single dmabuf
    target: Arc<VulkanDmabuf>,
    inner: Mutex<VideoDecodeInner>,
    /* Image view for `target`, type COLOR, entire image */
    output_image_view: vk::ImageView,
    /* State specific to hardware vs software decoding pathways */
    data: VideoDecodeStateData,
}

struct VideoEncodeInner {
    ctx: *mut AVCodecContext,
}
unsafe impl Send for VideoEncodeInner {}

enum VideoEncodeStateData {
    SW,
    HW(VideoHWStagingImages),
}
pub struct VideoEncodeState {
    target: Arc<VulkanDmabuf>,
    inner: Mutex<VideoEncodeInner>,
    /* Image view for `target`, type COLOR, entire image */
    output_image_view: vk::ImageView,
    /* State specific to hardware vs software encoding pathways */
    data: VideoEncodeStateData,
}

impl Drop for VideoDecodeState {
    fn drop(&mut self) {
        unsafe {
            let mut x = self.inner.lock().unwrap();
            self.target
                .vulk
                .video
                .as_ref()
                .unwrap()
                .bindings
                .avcodec_free_context(&mut x.ctx);
            self.target
                .vulk
                .dev
                .destroy_image_view(self.output_image_view, None);
            match self.data {
                VideoDecodeStateData::HW(ref data) => {
                    free_staging_images(&self.target.vulk, data);
                }
                VideoDecodeStateData::SW => (),
            }
        }
    }
}
#[cfg(feature = "video")]
impl Drop for VideoEncodeState {
    fn drop(&mut self) {
        unsafe {
            let mut x = self.inner.lock().unwrap();
            self.target
                .vulk
                .video
                .as_ref()
                .unwrap()
                .bindings
                .avcodec_free_context(&mut x.ctx);
            self.target
                .vulk
                .dev
                .destroy_image_view(self.output_image_view, None);
            match self.data {
                VideoEncodeStateData::HW(ref data) => {
                    free_staging_images(&self.target.vulk, data);
                }
                VideoEncodeStateData::SW => (),
            }
        }
    }
}

impl VulkanDecodeOpHandle {
    /* Not recommended in general -- blocks the thread. Returns true if point reached. */
    #[cfg(test)]
    pub fn wait_until_done(self: &VulkanDecodeOpHandle) -> Result<(), String> {
        self.decode
            .target
            .vulk
            .wait_for_timeline_pt(self.completion_time_point, u64::MAX)
            .map(|_| ())
    }
    pub fn get_timeline_point(self: &VulkanDecodeOpHandle) -> u64 {
        self.completion_time_point
    }
}

/** Return the length (excluding trailing 0) of a 0-terminated string. */
unsafe fn strlen(s: *const c_char) -> usize {
    for i in 0.. {
        if s.add(i).read() == 0 {
            return i;
        }
    }
    unreachable!();
}

fn av_strerror<'a>(bindings: &ffmpeg, err_buf: &'a mut [u8], ret: c_int) -> &'a str {
    unsafe {
        // SAFETY: av_strerror null-terminates, sizeof(u8) = sizeof(char), todo
        if bindings.av_strerror(ret, err_buf.as_mut_ptr() as *mut c_char, err_buf.len()) == 0 {
            std::str::from_utf8(&err_buf[..err_buf.iter().position(|x| *x == 0).unwrap()]).unwrap()
        } else {
            "unknown error"
        }
    }
}

unsafe fn av_hwframe_ctx_init(
    bindings: &ffmpeg,
    frames_ref: *mut AVBufferRef,
) -> Result<(), String> {
    let ret = bindings.av_hwframe_ctx_init(frames_ref);
    if ret != 0 {
        let mut err_buf = [0_u8; 1024];
        let err = av_strerror(bindings, &mut err_buf, ret);
        return Err(tag!(
            "Failed to initialize hwframe context: {}: {:?}",
            ret,
            err
        ));
    }
    Ok(())
}
unsafe fn av_hwdevice_ctx_init(
    bindings: &ffmpeg,
    device_ref: *mut AVBufferRef,
) -> Result<(), String> {
    let ret = bindings.av_hwdevice_ctx_init(device_ref);
    if ret != 0 {
        let mut err_buf = [0_u8; 1024];
        let err = av_strerror(bindings, &mut err_buf, ret);
        return Err(tag!(
            "Failed to initialize vulkan hwdevice: {}: {:?}",
            ret,
            err
        ));
    }
    Ok(())
}
unsafe fn avcodec_open(
    bindings: &ffmpeg,
    avctx: *mut AVCodecContext,
    codec: *const AVCodec,
    opts: *mut *mut AVDictionary,
) -> Result<(), String> {
    let ret = bindings.avcodec_open2(avctx, codec, opts);
    if ret != 0 {
        let mut err_buf = [0_u8; 1024];
        let err = av_strerror(bindings, &mut err_buf, ret);
        let name = CStr::from_ptr((*codec).name);
        return Err(tag!(
            "Failed to open codec context for {:?}: {}: {:?}",
            name,
            ret,
            err
        ));
    }
    Ok(())
}
unsafe fn avcodec_send_packet(
    bindings: &ffmpeg,
    avctx: *mut AVCodecContext,
    packet: *const AVPacket,
) -> Result<(), String> {
    let ret = bindings.avcodec_send_packet(avctx, packet);
    if ret != 0 {
        let mut err_buf = [0_u8; 1024];
        let err = av_strerror(bindings, &mut err_buf, ret);
        let name = CStr::from_ptr((*(*avctx).codec).name);
        return Err(tag!(
            "Failed to send video packet to {:?}: {}: {:?}",
            name,
            ret,
            err
        ));
    }
    Ok(())
}
unsafe fn avcodec_receive_packet(
    bindings: &ffmpeg,
    avctx: *mut AVCodecContext,
    packet: *mut AVPacket,
) -> Result<(), String> {
    let ret = bindings.avcodec_receive_packet(avctx, packet);
    if ret != 0 {
        let mut err_buf = [0_u8; 1024];
        let err = av_strerror(bindings, &mut err_buf, ret);
        let name = CStr::from_ptr((*(*avctx).codec).name);
        if name == c"libsvtav1" {
            debug!("Note: libsvtav1 version > 2.3.0 is required for low delay encoding to work");
        }
        return Err(tag!(
            "Failed to receive video packet from {:?}: {}: {:?}",
            name,
            ret,
            err
        ));
    }
    Ok(())
}
unsafe fn avcodec_send_frame(
    bindings: &ffmpeg,
    avctx: *mut AVCodecContext,
    frame: *const AVFrame,
) -> Result<(), String> {
    let ret = bindings.avcodec_send_frame(avctx, frame);
    if ret != 0 {
        let mut err_buf = [0_u8; 1024];
        let err = av_strerror(bindings, &mut err_buf, ret);
        let name = CStr::from_ptr((*(*avctx).codec).name);
        return Err(tag!(
            "Failed to send video packet to {:?}: {}: {:?}",
            name,
            ret,
            err
        ));
    }
    Ok(())
}
unsafe fn avcodec_receive_frame(
    bindings: &ffmpeg,
    avctx: *mut AVCodecContext,
    frame: *mut AVFrame,
) -> Result<(), String> {
    let ret = bindings.avcodec_receive_frame(avctx, frame);
    if ret != 0 {
        let mut err_buf = [0_u8; 1024];
        let err = av_strerror(bindings, &mut err_buf, ret);
        let name = CStr::from_ptr((*(*avctx).codec).name);
        return Err(tag!(
            "Failed to receive video frame from {:?}: {}: {:?}",
            name,
            ret,
            err
        ));
    }
    Ok(())
}

fn pack_glsl_mat3x4(mtx: &[[f32; 4]; 3]) -> [u8; 48] {
    let mut push_u8 = [0u8; 48];
    // 3 columns, 4 rows, rows packed
    for (j, col) in mtx.iter().enumerate().take(3) {
        for (i, px) in col.iter().enumerate().take(4) {
            let k = 4 * j + i;
            push_u8[k * 4..(k + 1) * 4].copy_from_slice(&px.to_le_bytes());
        }
    }
    push_u8
}

/* For compatibility with original Waypipe; align to 16-pixel blocks. This will
 * suffice for most alignment requirements. This is not a big deal since we should
 * copy to an intermediate buffer anyway. */
fn align_size(width: u32, height: u32, format: VideoFormat) -> (i32, i32) {
    let mut w = width.next_multiple_of(16) as i32;
    if format == VideoFormat::H264 {
        /* libavcodec requires width >= 32 for software encoding H264 */
        w = w.max(32);
    }
    let h = height.next_multiple_of(16) as i32;
    (w, h)
}

fn set_context_extensions(
    bindings: &ffmpeg,
    ctx: &mut AVVulkanDeviceContext,
    device_exts: &[*const c_char],
    instance_exts: &[*const c_char],
) -> Result<(), String> {
    /* Provide instance and device extensions being used; all associated data
     * (including strings) must be allocated, as it will be freed with av_free later */
    unsafe {
        let inst_exts: *mut *const c_char =
            bindings.av_malloc(std::mem::size_of_val(instance_exts)) as _;
        if inst_exts.is_null() {
            return Err(tag!("failed to allocate instance extensions"));
        }

        let dev_exts: *mut *const c_char =
            bindings.av_malloc(std::mem::size_of_val(device_exts)) as _;
        if dev_exts.is_null() {
            bindings.av_free(inst_exts as _);
            return Err(tag!("failed to allocate device extensions"));
        }
        for (i, e) in instance_exts.iter().enumerate() {
            let len = strlen(*e);
            let v: *mut c_char = bindings.av_malloc(len + 1) as _;
            if v.is_null() {
                for j in 0..i {
                    bindings.av_free(inst_exts.add(j) as _);
                }
                bindings.av_free(inst_exts as _);
                bindings.av_free(dev_exts as _);

                return Err(tag!("failed to allocated extension name"));
            }
            v.copy_from_nonoverlapping(*e, len + 1);
            (*inst_exts.add(i)) = v as _;
        }
        for (i, e) in device_exts.iter().enumerate() {
            let len = strlen(*e);
            let v: *mut c_char = bindings.av_malloc(len + 1) as _;
            if v.is_null() {
                for j in 0..i {
                    bindings.av_free(dev_exts.add(j) as _);
                }
                for j in 0..instance_exts.len() {
                    bindings.av_free(dev_exts.add(j) as _);
                }
                bindings.av_free(inst_exts as _);
                bindings.av_free(dev_exts as _);
                return Err(tag!("failed to allocated extension name"));
            }
            v.copy_from_nonoverlapping(*e, len + 1);
            (*dev_exts.add(i)) = v as _;
        }

        ctx.enabled_inst_extensions = inst_exts;
        ctx.nb_enabled_inst_extensions = instance_exts.len().try_into().unwrap();
        ctx.nb_enabled_dev_extensions = device_exts.len().try_into().unwrap();
        ctx.enabled_dev_extensions = dev_exts;
    }

    Ok(())
}

fn create_compute_pipeline(
    dev: &Device,
    shader: &[u32],
    bindings: &[vk::DescriptorSetLayoutBinding],
    push_len: usize,
) -> Result<VulkanComputePipeline, String> {
    unsafe {
        let shader_create = vk::ShaderModuleCreateInfo::default()
            .flags(vk::ShaderModuleCreateFlags::empty())
            .code(shader);
        let shader_module = dev
            .create_shader_module(&shader_create, None)
            .map_err(|_| "Failed to create shader")?;

        let layout_info = vk::DescriptorSetLayoutCreateInfo::default()
            .flags(vk::DescriptorSetLayoutCreateFlags::empty())
            .bindings(bindings);
        let ds_layout = dev
            .create_descriptor_set_layout(&layout_info, None)
            .map_err(|_| "Failed to create descriptor set layout")?;

        let layouts = &[ds_layout];
        let push_ranges = &[vk::PushConstantRange::default()
            .stage_flags(vk::ShaderStageFlags::COMPUTE)
            .offset(0)
            .size(push_len.try_into().unwrap())];
        let pipeline_layout_info = vk::PipelineLayoutCreateInfo::default()
            .flags(vk::PipelineLayoutCreateFlags::empty())
            .set_layouts(layouts)
            .push_constant_ranges(push_ranges);
        let pipeline_layout = dev
            .create_pipeline_layout(&pipeline_layout_info, None)
            .map_err(|_| "Failed to create pipeline layout")?;

        let entrypoint = c"main";
        let pipeline_shader_create = vk::PipelineShaderStageCreateInfo::default()
            .stage(vk::ShaderStageFlags::COMPUTE)
            .module(shader_module)
            .name(entrypoint); // no specialization info
        let pipeline_info = vk::ComputePipelineCreateInfo::default()
            .flags(vk::PipelineCreateFlags::empty())
            .stage(pipeline_shader_create)
            .layout(pipeline_layout);
        let pipeline = dev
            .create_compute_pipelines(vk::PipelineCache::null(), &[pipeline_info], None)
            .map_err(|_| "Failed to create compute pipeline")?
            .pop()
            .unwrap();

        Ok(VulkanComputePipeline {
            shader_module,
            ds_layout,
            pipeline_layout,
            pipeline,
        })
    }
}

pub unsafe fn setup_video(
    entry: &Entry,
    instance: &Instance,
    physdev: &vk::PhysicalDevice,
    dev: &Device,
    pdev_info: &DeviceInfo,
    debug: bool,
    queue_info: &[(vk::QueueFlags, vk::VideoCodecOperationFlagsKHR, i32, i32)],
    device_exts: &[*const c_char],
    instance_exts: &[*const c_char],
) -> Result<Option<VulkanVideo>, String> {
    /* loading libavcodec transitively loads a matching libavutil */
    let lib = match ffmpeg::new(format!("libavcodec.so.{}", LIBAVCODEC_VERSION_MAJOR)) {
        Ok(x) => x,
        Err(x) => {
            error!("Failed to load libavcodec (+ libavutil, etc.): {}. Video encoding/decoding is disabled.", x);
            return Ok(None);
        }
    };

    lib.av_log_set_level(if debug {
        AV_LOG_VERBOSE
    } else {
        AV_LOG_WARNING
    } as _);
    // av_log_set_level(AV_LOG_TRACE as _);
    lib.av_log_set_callback(Some(lib.av_log_default_callback));

    let hw_video = pdev_info.hw_enc_h264 | pdev_info.hw_dec_h264 | pdev_info.hw_dec_av1;
    let device_ref = if hw_video {
        debug!("Setting up video hardware device context");

        // Option<Video-ptr?>
        let device_ref: *mut AVBufferRef =
            lib.av_hwdevice_ctx_alloc(AVHWDeviceType_AV_HWDEVICE_TYPE_VULKAN);
        if device_ref.is_null() {
            return Err(tag!("Failed to allocate vulkan type hwdevice"));
        }
        let hw_context = (*device_ref).data.cast::<AVHWDeviceContext>();
        let vk_context = (*hw_context).hwctx.cast::<AVVulkanDeviceContext>();
        let ctx: &mut AVVulkanDeviceContext = vk_context.as_mut().unwrap();
        // todo: sanity check this
        ctx.get_proc_addr = Some(core::mem::transmute::<
            unsafe extern "system" fn(
                ash::vk::Instance,
                *const c_char,
            )
                -> std::option::Option<unsafe extern "system" fn()>,
            unsafe extern "C" fn(
                *mut waypipe_ffmpeg_wrapper::VkInstance_T,
                *const c_char,
            ) -> std::option::Option<unsafe extern "C" fn()>,
        >(entry.static_fn().get_instance_proc_addr));
        // u64?
        ctx.inst = instance.handle().as_raw() as *mut _;
        ctx.phys_dev = physdev.as_raw() as *mut _;
        ctx.act_dev = dev.handle().as_raw() as *mut _;

        let mut feats = vk::PhysicalDeviceFeatures2 {
            ..Default::default()
        };
        instance.get_physical_device_features2(*physdev, &mut feats);

        ctx.device_features.sType = VkStructureType_VK_STRUCTURE_TYPE_PHYSICAL_DEVICE_FEATURES_2;
        ctx.device_features.pNext = std::ptr::null_mut();
        ctx.device_features.features = std::mem::transmute::<
            ash::vk::PhysicalDeviceFeatures,
            waypipe_ffmpeg_wrapper::VkPhysicalDeviceFeatures,
        >(feats.features);

        set_context_extensions(&lib, ctx, device_exts, instance_exts)?;

        /* Note: the queue_family_indices are deprecated and will be replaced
         * by `.qf`/`.nb_qf` */
        for (i, (flags, codec_ops, index, nbound)) in queue_info.iter().enumerate() {
            let Some(qf) = ctx.qf.get_mut(i) else {
                continue;
            };
            qf.idx = *index;
            qf.num = *nbound;
            qf.flags = flags.as_raw();
            qf.video_caps = codec_ops.as_raw();
        }
        ctx.nb_qf = i32::try_from(queue_info.len().min(ctx.qf.len())).unwrap();

        av_hwdevice_ctx_init(&lib, device_ref)?;

        // For vulkan, hwconfig currently ignored
        let hwframes_constraints =
            lib.av_hwdevice_get_hwframe_constraints(device_ref, std::ptr::null_mut());

        // NOTE: these are all formats supported by Vulkan; must constrain with decoding details...
        let _hw_fmtlist = (*hwframes_constraints).valid_hw_formats;
        let _sw_fmtlist = (*hwframes_constraints).valid_sw_formats;
        /* TODO: hwframes_constraints only gives Vulkan limitations; decoder may have other limits (like <= 4096 wide), as seen by trace output */

        device_ref
    } else {
        std::ptr::null_mut()
    };

    // todo: loading earlier may simplify video availability detection; loading on demand may reduce latency
    let h264dec = lib.avcodec_find_decoder_by_name("h264\0".as_bytes().as_ptr() as *const _);
    let codecs_h264 = CodecSet {
        decoder: h264dec,
        sw_decoder: h264dec,
        encoder: lib.avcodec_find_encoder_by_name("h264_vulkan\0".as_bytes().as_ptr() as *const _),
        sw_encoder: lib.avcodec_find_encoder_by_name("libx264\0".as_bytes().as_ptr() as *const _),
    };
    let codecs_vp9 = CodecSet {
        decoder: std::ptr::null(),
        sw_decoder: lib.avcodec_find_decoder_by_name("vp9\0".as_bytes().as_ptr() as *const _),
        encoder: std::ptr::null(),
        sw_encoder: lib
            .avcodec_find_encoder_by_name("libvpx-vp9\0".as_bytes().as_ptr() as *const _),
    };
    let codecs_av1 = CodecSet {
        decoder: lib.avcodec_find_decoder_by_name("av1\0".as_bytes().as_ptr() as *const _),
        sw_decoder: lib.avcodec_find_decoder_by_name("libdav1d\0".as_bytes().as_ptr() as *const _),
        encoder: lib.avcodec_find_encoder_by_name("av1_vulkan\0".as_bytes().as_ptr() as *const _),
        /* AV1 software encoder comparison. As of writing:
         * - librav1e: may require a minimum frame lookahead, unknown if this was ever fixed
         * - libsvtav1: as of version 2.3.0, zero latency is attainable with pred-struct=1:rc=2.
         *     but: the setup/memory allocation for encoding takes a large fraction of a second,
         *     which is impractical for Waypipe's current one-stream-per-buffer approach
         * - libaom-av1: works, has a zero lag mode
         */
        sw_encoder: lib
            .avcodec_find_encoder_by_name("libaom-av1\0".as_bytes().as_ptr() as *const _),
    };

    debug!(
        "H264 support: hwenc {} swenc {} hwdec {} swdec {}",
        fmt_bool(!codecs_h264.encoder.is_null() && pdev_info.hw_enc_h264),
        fmt_bool(!codecs_h264.sw_encoder.is_null()),
        fmt_bool(!codecs_h264.decoder.is_null() && pdev_info.hw_dec_h264),
        fmt_bool(!codecs_h264.sw_decoder.is_null()),
    );
    debug!(
        "VP9 support:  hwenc f swenc {} hwdec f swdec {}",
        fmt_bool(!codecs_vp9.sw_encoder.is_null()),
        fmt_bool(!codecs_vp9.sw_decoder.is_null()),
    );
    debug!(
        "AV1 support:  hwenc {} swenc {} hwdec {} swdec {}",
        fmt_bool(!codecs_av1.encoder.is_null() && pdev_info.hw_enc_av1),
        fmt_bool(!codecs_av1.sw_encoder.is_null()),
        fmt_bool(!codecs_av1.decoder.is_null() && pdev_info.hw_dec_av1),
        fmt_bool(!codecs_av1.sw_decoder.is_null()),
    );

    let bindings = &[
        vk::DescriptorSetLayoutBinding::default()
            .descriptor_type(vk::DescriptorType::STORAGE_IMAGE)
            .binding(0)
            .descriptor_count(1)
            .stage_flags(vk::ShaderStageFlags::COMPUTE),
        vk::DescriptorSetLayoutBinding::default()
            .descriptor_type(vk::DescriptorType::COMBINED_IMAGE_SAMPLER)
            .binding(1)
            .descriptor_count(1)
            .stage_flags(vk::ShaderStageFlags::COMPUTE),
        vk::DescriptorSetLayoutBinding::default()
            .descriptor_type(vk::DescriptorType::COMBINED_IMAGE_SAMPLER)
            .binding(2)
            .descriptor_count(1)
            .stage_flags(vk::ShaderStageFlags::COMPUTE),
    ];
    let nv12_img_to_rgb = create_compute_pipeline(dev, NV12_IMG_TO_RGB, bindings, 4 * 3 * 4)?;
    let bindings = &[
        vk::DescriptorSetLayoutBinding::default()
            .descriptor_type(vk::DescriptorType::COMBINED_IMAGE_SAMPLER)
            .binding(0)
            .descriptor_count(1)
            .stage_flags(vk::ShaderStageFlags::COMPUTE),
        vk::DescriptorSetLayoutBinding::default()
            .descriptor_type(vk::DescriptorType::STORAGE_IMAGE)
            .binding(1)
            .descriptor_count(1)
            .stage_flags(vk::ShaderStageFlags::COMPUTE),
        vk::DescriptorSetLayoutBinding::default()
            .descriptor_type(vk::DescriptorType::STORAGE_IMAGE)
            .binding(2)
            .descriptor_count(1)
            .stage_flags(vk::ShaderStageFlags::COMPUTE),
    ];
    let rgb_to_nv12_img = create_compute_pipeline(dev, RGB_TO_NV12_IMG, bindings, 4 * 3 * 4)?;

    let bindings = &[
        vk::DescriptorSetLayoutBinding::default()
            .descriptor_type(vk::DescriptorType::COMBINED_IMAGE_SAMPLER)
            .binding(0)
            .descriptor_count(1)
            .stage_flags(vk::ShaderStageFlags::COMPUTE),
        vk::DescriptorSetLayoutBinding::default()
            .descriptor_type(vk::DescriptorType::STORAGE_TEXEL_BUFFER)
            .binding(1)
            .descriptor_count(1)
            .stage_flags(vk::ShaderStageFlags::COMPUTE),
        vk::DescriptorSetLayoutBinding::default()
            .descriptor_type(vk::DescriptorType::STORAGE_TEXEL_BUFFER)
            .binding(2)
            .descriptor_count(1)
            .stage_flags(vk::ShaderStageFlags::COMPUTE),
        vk::DescriptorSetLayoutBinding::default()
            .descriptor_type(vk::DescriptorType::STORAGE_TEXEL_BUFFER)
            .binding(3)
            .descriptor_count(1)
            .stage_flags(vk::ShaderStageFlags::COMPUTE),
    ];
    let rgb_to_yuv420_buf =
        create_compute_pipeline(dev, RGB_TO_YUV420_BUF, bindings, 4 * 3 * 4 + 3 * 4)?;
    let bindings = &[
        vk::DescriptorSetLayoutBinding::default()
            .descriptor_type(vk::DescriptorType::STORAGE_IMAGE)
            .binding(0)
            .descriptor_count(1)
            .stage_flags(vk::ShaderStageFlags::COMPUTE),
        vk::DescriptorSetLayoutBinding::default()
            .descriptor_type(vk::DescriptorType::UNIFORM_TEXEL_BUFFER)
            .binding(1)
            .descriptor_count(1)
            .stage_flags(vk::ShaderStageFlags::COMPUTE),
        vk::DescriptorSetLayoutBinding::default()
            .descriptor_type(vk::DescriptorType::UNIFORM_TEXEL_BUFFER)
            .binding(2)
            .descriptor_count(1)
            .stage_flags(vk::ShaderStageFlags::COMPUTE),
        vk::DescriptorSetLayoutBinding::default()
            .descriptor_type(vk::DescriptorType::UNIFORM_TEXEL_BUFFER)
            .binding(3)
            .descriptor_count(1)
            .stage_flags(vk::ShaderStageFlags::COMPUTE),
    ];
    let yuv420_buf_to_rgb =
        create_compute_pipeline(dev, YUV420_BUF_TO_RGB, bindings, 4 * 3 * 4 + 3 * 4)?;

    let rect_lin_sampler = vk::SamplerCreateInfo::default()
        .flags(vk::SamplerCreateFlags::empty())
        .mag_filter(vk::Filter::LINEAR)
        .min_filter(vk::Filter::LINEAR)
        .mipmap_mode(vk::SamplerMipmapMode::NEAREST)
        .address_mode_u(vk::SamplerAddressMode::CLAMP_TO_EDGE)
        .address_mode_v(vk::SamplerAddressMode::CLAMP_TO_EDGE)
        .address_mode_w(vk::SamplerAddressMode::REPEAT)
        .mip_lod_bias(0.)
        .anisotropy_enable(false)
        .max_anisotropy(0.)
        .compare_enable(false)
        .min_lod(0.)
        .max_lod(0.)
        .border_color(vk::BorderColor::default())
        .unnormalized_coordinates(true);

    let sampler_y_info = rect_lin_sampler;
    let sampler_rb_info = rect_lin_sampler;
    let yuv_to_rgb_sampler_y = dev
        .create_sampler(&sampler_y_info, None)
        .map_err(|_| "Failed to allocate sampler Y")?;
    let yuv_to_rgb_sampler_rb = dev
        .create_sampler(&sampler_rb_info, None)
        .map_err(|_| "Failed to allocate sampler CrCb")?;
    let sampler_rgb_info = rect_lin_sampler;
    let rgb_to_yuv_sampler_rgb = dev
        .create_sampler(&sampler_rgb_info, None)
        .map_err(|_| "Failed to allocate sampler RGB")?;

    Ok(Some(VulkanVideo {
        bindings: lib,
        av_hwdevice: device_ref,
        codecs_h264,
        codecs_vp9,
        codecs_av1,
        can_hw_enc_h264: pdev_info.hw_enc_h264,
        can_hw_dec_h264: pdev_info.hw_dec_h264,
        can_hw_enc_av1: pdev_info.hw_enc_av1,
        can_hw_dec_av1: pdev_info.hw_dec_av1,
        rgb_to_yuv420_buf,
        yuv420_buf_to_rgb,
        nv12_img_to_rgb,
        rgb_to_nv12_img,
        yuv_to_rgb_sampler_y,
        yuv_to_rgb_sampler_rb,
        rgb_to_yuv_sampler_rgb,
    }))
}

/** Lock the first queue in the given family, if a hardware context was set up */
pub unsafe fn video_lock_queue(video: &VulkanVideo, queue_family: u32) {
    if video.av_hwdevice.is_null() {
        return;
    }
    let hw_context = (*video.av_hwdevice).data.cast::<AVHWDeviceContext>();
    let vk_context = (*hw_context).hwctx.cast::<AVVulkanDeviceContext>();
    (*vk_context).lock_queue.unwrap()(hw_context, queue_family, 0);
}
/** Unlock the first queue in the given family, if a hardware context was set up */
pub unsafe fn video_unlock_queue(video: &VulkanVideo, queue_family: u32) {
    if video.av_hwdevice.is_null() {
        return;
    }
    let hw_context = (*video.av_hwdevice).data.cast::<AVHWDeviceContext>();
    let vk_context = (*hw_context).hwctx.cast::<AVVulkanDeviceContext>();
    (*vk_context).unlock_queue.unwrap()(hw_context, queue_family, 0);
}

/* Pick format: Vulkan, and setup hw frames context */
#[cfg(feature = "video")]
unsafe extern "C" fn pick_video_format_hw(ctx: *mut AVCodecContext, fmts: *const i32) -> i32 {
    /* Return AV_PIX_FMT_VULKAN if present in list */
    for i in 0.. {
        let f = fmts.add(i).read();
        if f == AVPixelFormat_AV_PIX_FMT_NONE {
            /* Failure */
            error!("Did not find AV_PIX_FMT_VULKAN in format list");
            return AVPixelFormat_AV_PIX_FMT_NONE;
        }
        if f == AVPixelFormat_AV_PIX_FMT_VULKAN {
            break;
        }
    }

    let bindings: &ffmpeg = ((*ctx).opaque as *const ffmpeg).as_ref().unwrap();

    {
        let out_frames_ref = &mut (*ctx).hw_frames_ctx;
        let ret = bindings.avcodec_get_hw_frames_parameters(
            ctx,
            (*ctx).hw_device_ctx,
            AVPixelFormat_AV_PIX_FMT_VULKAN,
            out_frames_ref,
        );
        if ret != 0 {
            error!("Failed to get hw frame parameters: {}", ret);
            return AVPixelFormat_AV_PIX_FMT_NONE;
        }
    }

    if let Err(e) = av_hwframe_ctx_init(bindings, (*ctx).hw_frames_ctx) {
        error!("Failed to initialize hw frames: {}", e);
        return AVPixelFormat_AV_PIX_FMT_NONE;
    }

    AVPixelFormat_AV_PIX_FMT_VULKAN
}

unsafe fn free_staging_images(vulk: &VulkanDevice, data: &VideoHWStagingImages) {
    for i in 0..2 {
        vulk.dev.destroy_image_view(data.plane_image_views[i], None);
        vulk.dev.destroy_image(data.plane_images[i], None);
        vulk.dev.free_memory(data.plane_memories[i], None);
    }
}

fn create_staging_images(
    vulk: &VulkanDevice,
    width: u32,
    height: u32,
    for_encode: bool,
    fmt: vk::Format,
) -> Result<VideoHWStagingImages, String> {
    assert!(fmt == vk::Format::G8_B8R8_2PLANE_420_UNORM);
    assert!(width % 2 == 0 && height % 2 == 0);

    let planes = &[
        (vk::Format::R8_UNORM, width, height),
        (vk::Format::R8G8_UNORM, width / 2, height / 2),
    ];

    let mut plane_images = [vk::Image::null(); 2];
    let mut plane_memories = [vk::DeviceMemory::null(); 2];
    let mut plane_image_views = [vk::ImageView::null(); 2];

    unsafe {
        for plane in 0..=1 {
            let image_info = vk::ImageCreateInfo::default()
                .flags(vk::ImageCreateFlags::empty())
                .image_type(vk::ImageType::TYPE_2D)
                .format(planes[plane].0)
                .extent(vk::Extent3D {
                    width: planes[plane].1,
                    height: planes[plane].2,
                    depth: 1,
                })
                .mip_levels(1)
                .array_layers(1)
                .samples(vk::SampleCountFlags::TYPE_1)
                .tiling(vk::ImageTiling::OPTIMAL)
                .usage(if for_encode {
                    vk::ImageUsageFlags::STORAGE | vk::ImageUsageFlags::TRANSFER_SRC
                } else {
                    vk::ImageUsageFlags::TRANSFER_DST | vk::ImageUsageFlags::SAMPLED
                })
                .sharing_mode(vk::SharingMode::EXCLUSIVE)
                .initial_layout(vk::ImageLayout::UNDEFINED);

            let image = vulk
                .dev
                .create_image(&image_info, None)
                .map_err(|x| tag!("Failed to create Vulkan image: {:?}", x))?;

            let mem_reqs = vulk.dev.get_image_memory_requirements(image);

            assert!(mem_reqs.memory_type_bits != 0);
            let mem_index = mem_reqs.memory_type_bits.trailing_zeros();

            let alloc_info = vk::MemoryAllocateInfo::default()
                .allocation_size(mem_reqs.size)
                .memory_type_index(mem_index);
            let memory = vulk
                .dev
                .allocate_memory(&alloc_info, None)
                .map_err(|x| tag!("Failed to allocate image memory: {:?}", x))?;

            vulk.dev
                .bind_image_memory(image, memory, 0)
                .map_err(|x| tag!("Failed to bind image memory: {:?}", x))?;

            let image_view_info = vk::ImageViewCreateInfo::default()
                .flags(vk::ImageViewCreateFlags::empty())
                .image(image)
                .view_type(vk::ImageViewType::TYPE_2D)
                .format(planes[plane].0)
                .components(vk::ComponentMapping::default().r(vk::ComponentSwizzle::IDENTITY))
                .subresource_range(
                    vk::ImageSubresourceRange::default()
                        .aspect_mask(vk::ImageAspectFlags::COLOR)
                        .base_mip_level(0)
                        .level_count(1)
                        .base_array_layer(0)
                        .layer_count(1),
                );
            let image_view = vulk
                .dev
                .create_image_view(&image_view_info, None)
                .map_err(|_| "Failed to create plane 1 image view")?;

            plane_images[plane] = image;
            plane_memories[plane] = memory;
            plane_image_views[plane] = image_view;
        }

        Ok(VideoHWStagingImages {
            plane_memories,
            plane_images,
            plane_image_views,
        })
    }
}

/** Create a vkImageSubresourceLayers for an entire image (single layer, no mipmaps) */
fn subresource_layer(aspect_mask: vk::ImageAspectFlags) -> vk::ImageSubresourceLayers {
    vk::ImageSubresourceLayers {
        aspect_mask,
        mip_level: 0,
        base_array_layer: 0,
        layer_count: 1,
    }
}

/** Image memory barrier for a same-queue layout transition.
 *
 * The access range is COLOR for the single level/layer of the entire image. */
fn image_layout_transition(
    image: vk::Image,
    old_layout: vk::ImageLayout,
    new_layout: vk::ImageLayout,
    src_access_mask: vk::AccessFlags,
    dst_access_mask: vk::AccessFlags,
) -> vk::ImageMemoryBarrier<'static> {
    let standard_access_range = vk::ImageSubresourceRange::default()
        .aspect_mask(vk::ImageAspectFlags::COLOR)
        .level_count(1)
        .layer_count(1);

    vk::ImageMemoryBarrier::default()
        .image(image)
        .old_layout(old_layout)
        .new_layout(new_layout)
        .src_queue_family_index(vk::QUEUE_FAMILY_IGNORED)
        .dst_queue_family_index(vk::QUEUE_FAMILY_IGNORED)
        .src_access_mask(src_access_mask)
        .dst_access_mask(dst_access_mask)
        .subresource_range(standard_access_range)
}

fn create_dmabuf_view(img: &VulkanDmabuf) -> Result<vk::ImageView, String> {
    let idswizzle = vk::ComponentMapping::default()
        .r(vk::ComponentSwizzle::IDENTITY)
        .g(vk::ComponentSwizzle::IDENTITY)
        .b(vk::ComponentSwizzle::IDENTITY)
        .a(vk::ComponentSwizzle::IDENTITY);
    let output_image_view_info = vk::ImageViewCreateInfo::default()
        .flags(vk::ImageViewCreateFlags::empty())
        .image(img.image)
        .view_type(vk::ImageViewType::TYPE_2D)
        .format(img.vk_format)
        .components(idswizzle)
        .subresource_range(
            vk::ImageSubresourceRange::default()
                .aspect_mask(vk::ImageAspectFlags::COLOR)
                .base_mip_level(0)
                .level_count(1)
                .base_array_layer(0)
                .layer_count(1),
        );
    unsafe {
        let output_image_view = img
            .vulk
            .dev
            .create_image_view(&output_image_view_info, None)
            .map_err(|x| tag!("Failed to create image view for dmabuf image: {}", x))?;
        Ok(output_image_view)
    }
}

pub fn setup_video_decode_hw(
    img: &Arc<VulkanDmabuf>,
    fmt: VideoFormat,
) -> Result<VideoDecodeState, String> {
    let video = img.vulk.video.as_ref().unwrap();

    let decoder: *const AVCodec = match fmt {
        VideoFormat::H264 => video.codecs_h264.decoder,
        VideoFormat::VP9 => video.codecs_vp9.decoder,
        VideoFormat::AV1 => video.codecs_av1.decoder,
    };
    assert!(!decoder.is_null());
    unsafe {
        let ctx: *mut AVCodecContext = video.bindings.avcodec_alloc_context3(decoder);
        if ctx.is_null() {
            return Err(tag!("Failed to allocate context"));
        }

        let (awidth, aheight) = align_size(img.width, img.height, fmt);
        {
            let cr = ctx.as_mut().unwrap();

            let nref = video.bindings.av_buffer_ref(video.av_hwdevice);
            if nref.is_null() {
                return Err(tag!("Failed to add reference for av_hwdevice"));
            }

            cr.hw_device_ctx = nref;
            // todo: need to ensure video bindings are not moved; do Arc<Pin> ?
            cr.opaque = &video.bindings as *const ffmpeg as *mut _;
            cr.get_format = Some(pick_video_format_hw);

            (cr.width, cr.height) = (awidth, aheight);
        }

        /* ctx->get_format will be called to do setup work once a packet is received */
        avcodec_open(&video.bindings, ctx, decoder, std::ptr::null_mut())?;

        let output_image_view = create_dmabuf_view(img)?;

        let staging_images = create_staging_images(
            &img.vulk,
            awidth.try_into().unwrap(),
            aheight.try_into().unwrap(),
            false,
            vk::Format::G8_B8R8_2PLANE_420_UNORM,
        )?;

        Ok(VideoDecodeState {
            target: img.clone(),
            inner: Mutex::new(VideoDecodeInner { ctx }),
            output_image_view,
            data: VideoDecodeStateData::HW(staging_images),
        })
    }
}

/* Pick format: NV12 */
unsafe extern "C" fn pick_video_format_sw(_ctx: *mut AVCodecContext, fmts: *const i32) -> i32 {
    for i in 0.. {
        let f = fmts.add(i).read();
        if f == AVPixelFormat_AV_PIX_FMT_NONE {
            /* Failure */
            error!("Did not find AVPixelFormat_AV_PIX_FMT_YUV420P in list");
            return AVPixelFormat_AV_PIX_FMT_NONE;
        }
        if f == AVPixelFormat_AV_PIX_FMT_YUV420P {
            break;
        }
    }

    AVPixelFormat_AV_PIX_FMT_YUV420P
}

pub fn setup_video_decode_sw(
    img: &Arc<VulkanDmabuf>,
    fmt: VideoFormat,
) -> Result<VideoDecodeState, String> {
    let video = img.vulk.video.as_ref().unwrap();
    let decoder: *const AVCodec = match fmt {
        VideoFormat::H264 => video.codecs_h264.sw_decoder,
        VideoFormat::VP9 => video.codecs_vp9.sw_decoder,
        VideoFormat::AV1 => video.codecs_av1.sw_decoder,
    };
    unsafe {
        let ctx: *mut AVCodecContext = video.bindings.avcodec_alloc_context3(decoder);
        if ctx.is_null() {
            return Err(tag!("Failed to allocate context"));
        }
        {
            let cr = ctx.as_mut().unwrap();

            // todo: need to ensure video bindings are not moved; do Arc<Pin> ?
            cr.opaque = &video.bindings as *const ffmpeg as *mut _;
            cr.get_format = Some(pick_video_format_sw);

            (cr.width, cr.height) = align_size(img.width, img.height, fmt);
        }

        /* ctx->get_format will be called to do setup work once a packet is received */
        avcodec_open(&video.bindings, ctx, decoder, std::ptr::null_mut())?;

        let output_image_view = create_dmabuf_view(img)?;

        Ok(VideoDecodeState {
            target: img.clone(),
            inner: Mutex::new(VideoDecodeInner { ctx }),
            output_image_view,
            data: VideoDecodeStateData::SW,
        })
    }
}

/** libavcodec does not currently appear to have a way to expose the range of sizes
 * for which hardware encoding/decoding is supported. (Which may be graphics driver
 * dependent.) Technically one could try avcodec_open() and look for an EINVAL, but
 * doing that could hide bugs in Waypipe.) Hardcode a relatively safe threshold
 * for now, until a better solution is found.
 */
const MIN_H264_HW_SIZE: (u32, u32) = (128, 128);
const MIN_AV1_HW_ENC_SIZE: (u32, u32) = (128, 128);

pub fn setup_video_decode(
    img: &Arc<VulkanDmabuf>,
    fmt: VideoFormat,
) -> Result<VideoDecodeState, String> {
    assert!(img.can_store_and_sample);
    let video = img.vulk.video.as_ref().unwrap();
    let can_hw_decode = match fmt {
        VideoFormat::AV1 => video.can_hw_dec_av1 && !video.codecs_av1.decoder.is_null(),
        VideoFormat::VP9 => false,
        VideoFormat::H264 => {
            video.can_hw_dec_h264
                && !video.codecs_h264.decoder.is_null()
                && (img.width >= MIN_H264_HW_SIZE.0 && img.height >= MIN_H264_HW_SIZE.1)
        }
    };

    if can_hw_decode {
        setup_video_decode_hw(img, fmt)
    } else {
        setup_video_decode_sw(img, fmt)
    }
}

pub fn supports_video_format(
    vulk: &VulkanDevice,
    fmt: VideoFormat,
    drm_format: u32,
    _width: u32,
    _height: u32,
) -> bool {
    let Some(ref vid) = vulk.video else {
        return false;
    };
    let Ok(wlfmt) = TryInto::<WlShmFormat>::try_into(drm_to_wayland(drm_format)) else {
        return false;
    };
    match wlfmt {
        WlShmFormat::Rgb565 => (),
        WlShmFormat::Bgr565 => (),
        WlShmFormat::Xrgb8888 => (),
        WlShmFormat::Xbgr8888 => (),
        // These exceed the bit depth of the video formats, but video is already lossy.
        // 10-bit encoding is still probably worth using if fast
        WlShmFormat::Xrgb2101010 => (),
        WlShmFormat::Xbgr2101010 => (),
        WlShmFormat::Xrgb16161616 => (),
        WlShmFormat::Xbgr16161616 => (),
        _ => {
            return false;
        }
    };

    // TODO: lookup max size available for format
    match fmt {
        VideoFormat::H264 => {
            !vid.codecs_h264.sw_decoder.is_null() && !vid.codecs_h264.sw_encoder.is_null()
        }
        VideoFormat::VP9 => {
            !vid.codecs_vp9.sw_decoder.is_null() && !vid.codecs_vp9.sw_encoder.is_null()
        }
        VideoFormat::AV1 => {
            !vid.codecs_av1.sw_decoder.is_null() && !vid.codecs_av1.sw_encoder.is_null()
        }
    }
}

/* YUV to RGB conversion matrices. For compatibility with original Waypipe,
 * broadcast-limited output ranges (16-235 & 16-240, not 0-255) are used, and
 * Rec. 601 (where Y = 0.299 R + 0.587 G + 0.114 B, instead of
 * Rec. 709's Y = 0.2126 R + 0.7152 G + 0.0722 B)
 */
const RGB_TO_YUV: &[[f32; 4]; 3] = &[
    /* Limited range */
    [0.25678822, 0.5041294, 0.09790588, 0.0627451], // Y
    [0.4392157, -0.3677883, -0.07142738, 0.5019608], // U (Cb)
    [-0.1482229, -0.2909928, 0.4392157, 0.5019608], // V (Cr)
];
const YUV_TO_RGB: &[[f32; 4]; 3] = &[
    [1.1643835, 1.5960268, 0., -0.8742022],
    [1.1643835, -0.81296766, -0.3917623, 0.5316678],
    [1.1643835, 0., 2.0172322, -1.0856308],
];

pub fn start_dmavid_decode_hw(
    state: &Arc<VideoDecodeState>,
    pool: &Arc<VulkanCommandPool>,
    packet: &[u8],
) -> Result<VulkanDecodeOpHandle, String> {
    let vulk: &VulkanDevice = &state.target.vulk;
    let video = vulk.video.as_ref().unwrap();

    debug!(
        "Hardware decoding frame for {}x{} image, packet len {}",
        state.target.width,
        state.target.height,
        packet.len()
    );
    let VideoDecodeStateData::HW(ref state_data) = state.data else {
        unreachable!()
    };

    unsafe {
        let av_packet = video.bindings.av_packet_alloc();
        video
            .bindings
            .av_new_packet(av_packet, packet.len().try_into().unwrap());
        (*av_packet).data.copy_from(packet.as_ptr(), packet.len());

        let dec_inner = state.inner.lock().unwrap();
        avcodec_send_packet(&video.bindings, dec_inner.ctx, av_packet)?;

        let frame: *mut AVFrame = video.bindings.av_frame_alloc();

        // ignoring EAGAIN, since Waypipe's video streaming does one packet, one frame
        avcodec_receive_frame(&video.bindings, dec_inner.ctx, frame)?;

        let (frame_width, frame_height): (u32, u32) = (
            (*frame).width.try_into().unwrap(),
            (*frame).height.try_into().unwrap(),
        );
        assert!(frame_width >= state.target.width && frame_height >= state.target.height);

        let hw_fr_ref = (*frame).hw_frames_ctx.as_ref().unwrap();
        let hwfc_ref = hw_fr_ref.data.cast::<AVHWFramesContext>().as_mut().unwrap();

        let avvulk = hwfc_ref
            .hwctx
            .cast::<AVVulkanFramesContext>()
            .as_mut()
            .unwrap();
        let vkframe = ((*frame).data[0]).cast::<AVVkFrame>().as_mut().unwrap();

        /* Lock frame while recording command buffer */
        avvulk.lock_frame.as_ref().unwrap()(hwfc_ref, vkframe);

        /* Assert single image output */
        assert!(vkframe.img[1..]
            .iter()
            .all(|x| vk::Image::from_raw(*x as _).is_null()));

        assert!(avvulk.format[0] == (vk::Format::G8_B8R8_2PLANE_420_UNORM.as_raw() as u32));

        let wait_sems = &[vk::Semaphore::from_raw(vkframe.sem[0] as _)];
        let wait_values = &[vkframe.sem_value[0]];

        let init_layout = vkframe.layout[0];
        let src_img = vk::Image::from_raw(vkframe.img[0] as _);

        let sizes = &[
            vk::DescriptorPoolSize::default()
                .descriptor_count(1)
                .ty(vk::DescriptorType::STORAGE_IMAGE),
            vk::DescriptorPoolSize::default()
                .descriptor_count(2)
                .ty(vk::DescriptorType::COMBINED_IMAGE_SAMPLER),
        ];
        // at most 1 descriptor set
        let pool_storage_info = vk::DescriptorPoolCreateInfo::default()
            .flags(vk::DescriptorPoolCreateFlags::FREE_DESCRIPTOR_SET)
            .max_sets(1)
            .pool_sizes(sizes);
        let desc_pool = vulk
            .dev
            .create_descriptor_pool(&pool_storage_info, None)
            .map_err(|_| "Failed to create descriptor pool")?;

        let layouts = &[video.nv12_img_to_rgb.ds_layout];
        let desc_set_alloc_info = vk::DescriptorSetAllocateInfo::default()
            .descriptor_pool(desc_pool)
            .set_layouts(layouts);
        let descs = vulk
            .dev
            .allocate_descriptor_sets(&desc_set_alloc_info)
            .map_err(|_| "Failed to allocate descriptor sets")?;
        let descriptor_set = descs[0];

        let output_image_info = &[vk::DescriptorImageInfo::default()
            .image_view(state.output_image_view)
            .image_layout(vk::ImageLayout::GENERAL)];
        let input_1_image_info = &[vk::DescriptorImageInfo::default()
            .image_view(state_data.plane_image_views[0])
            .image_layout(vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL)
            .sampler(video.yuv_to_rgb_sampler_y)];
        let input_2_image_info = &[vk::DescriptorImageInfo::default()
            .image_view(state_data.plane_image_views[1])
            .image_layout(vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL)
            .sampler(video.yuv_to_rgb_sampler_rb)];

        let descriptor_writes = &[
            vk::WriteDescriptorSet::default()
                .dst_set(descriptor_set)
                .dst_binding(0)
                .descriptor_count(1)
                .descriptor_type(vk::DescriptorType::STORAGE_IMAGE)
                .image_info(output_image_info),
            vk::WriteDescriptorSet::default()
                .dst_set(descriptor_set)
                .dst_binding(1)
                .descriptor_count(1)
                .descriptor_type(vk::DescriptorType::COMBINED_IMAGE_SAMPLER)
                .image_info(input_1_image_info),
            vk::WriteDescriptorSet::default()
                .dst_set(descriptor_set)
                .dst_binding(2)
                .descriptor_count(1)
                .descriptor_type(vk::DescriptorType::COMBINED_IMAGE_SAMPLER)
                .image_info(input_2_image_info),
        ];
        vulk.dev.update_descriptor_sets(descriptor_writes, &[]);

        let inner_pool = pool.pool.lock().unwrap();

        let alloc_cb_info = vk::CommandBufferAllocateInfo::default()
            .command_pool(*inner_pool)
            .command_buffer_count(1)
            .level(vk::CommandBufferLevel::PRIMARY);
        drop(inner_pool);

        let cbvec = vulk
            .dev
            .allocate_command_buffers(&alloc_cb_info)
            .map_err(|_| "Failed to allocate command buffers")?;
        let cb = cbvec[0];

        let cb_info =
            vk::CommandBufferBeginInfo::default().flags(vk::CommandBufferUsageFlags::empty());
        vulk.dev
            .begin_command_buffer(cb, &cb_info)
            .map_err(|_| "Failed to begin command buffer")?;

        let target_layout = vk::ImageLayout::GENERAL;
        let src_layout = vk::ImageLayout::TRANSFER_SRC_OPTIMAL;

        let standard_access_range = vk::ImageSubresourceRange::default()
            .aspect_mask(vk::ImageAspectFlags::COLOR)
            .level_count(1)
            .layer_count(1);
        let mut img_inner = state.target.inner.lock().unwrap();

        let pre_transfer_barriers = &[vk::ImageMemoryBarrier::default()
            .image(src_img)
            .old_layout(vk::ImageLayout::from_raw(init_layout as _))
            .new_layout(src_layout)
            .src_queue_family_index(vk::QUEUE_FAMILY_IGNORED)
            .dst_queue_family_index(vk::QUEUE_FAMILY_IGNORED)
            .src_access_mask(vk::AccessFlags::MEMORY_WRITE)
            .dst_access_mask(vk::AccessFlags::TRANSFER_READ)
            .subresource_range(standard_access_range)];
        vulk.dev.cmd_pipeline_barrier(
            cb,
            vk::PipelineStageFlags::TOP_OF_PIPE,
            vk::PipelineStageFlags::TRANSFER,
            vk::DependencyFlags::empty(),
            &[],
            &[],
            pre_transfer_barriers,
        );
        /* Transition the staging images from undefined, discarding their previous
         * contents; the images will be entirely filled by the following copy operations. */
        let pre_transfer_barriers2 = &[
            image_layout_transition(
                state_data.plane_images[0],
                vk::ImageLayout::UNDEFINED,
                vk::ImageLayout::TRANSFER_DST_OPTIMAL,
                vk::AccessFlags::SHADER_READ,
                vk::AccessFlags::TRANSFER_WRITE,
            ),
            image_layout_transition(
                state_data.plane_images[1],
                vk::ImageLayout::UNDEFINED,
                vk::ImageLayout::TRANSFER_DST_OPTIMAL,
                vk::AccessFlags::SHADER_READ,
                vk::AccessFlags::TRANSFER_WRITE,
            ),
        ];
        vulk.dev.cmd_pipeline_barrier(
            cb,
            vk::PipelineStageFlags::COMPUTE_SHADER,
            vk::PipelineStageFlags::TRANSFER,
            vk::DependencyFlags::empty(),
            &[],
            &[],
            pre_transfer_barriers2,
        );

        let copy_plane_1 = &[vk::ImageCopy {
            src_subresource: subresource_layer(vk::ImageAspectFlags::PLANE_0),
            src_offset: vk::Offset3D::default(),
            dst_subresource: subresource_layer(vk::ImageAspectFlags::COLOR),
            dst_offset: vk::Offset3D::default(),
            extent: vk::Extent3D {
                width: frame_width,
                height: frame_height,
                depth: 1,
            },
        }];
        let copy_plane_2 = &[vk::ImageCopy {
            src_subresource: subresource_layer(vk::ImageAspectFlags::PLANE_1),
            src_offset: vk::Offset3D::default(),
            dst_subresource: subresource_layer(vk::ImageAspectFlags::COLOR),
            dst_offset: vk::Offset3D::default(),
            extent: vk::Extent3D {
                width: frame_width / 2,
                height: frame_height / 2,
                depth: 1,
            },
        }];
        vulk.dev.cmd_copy_image(
            cb,
            src_img,
            src_layout,
            state_data.plane_images[0],
            vk::ImageLayout::TRANSFER_DST_OPTIMAL,
            copy_plane_1,
        );
        vulk.dev.cmd_copy_image(
            cb,
            src_img,
            src_layout,
            state_data.plane_images[1],
            vk::ImageLayout::TRANSFER_DST_OPTIMAL,
            copy_plane_2,
        );

        let output_image_barrier = &[qfot_acquire_image_memory_barrier(
            state.target.image,
            img_inner.image_layout,
            target_layout,
            vulk.queue_family,
            vk::AccessFlags::SHADER_WRITE,
        )];
        vulk.dev.cmd_pipeline_barrier(
            cb,
            vk::PipelineStageFlags::TOP_OF_PIPE,
            vk::PipelineStageFlags::COMPUTE_SHADER,
            vk::DependencyFlags::empty(),
            &[],
            &[],
            output_image_barrier,
        );
        let staging_barrier = &[
            image_layout_transition(
                state_data.plane_images[0],
                vk::ImageLayout::TRANSFER_DST_OPTIMAL,
                vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL,
                vk::AccessFlags::TRANSFER_WRITE,
                vk::AccessFlags::SHADER_READ,
            ),
            image_layout_transition(
                state_data.plane_images[1],
                vk::ImageLayout::TRANSFER_DST_OPTIMAL,
                vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL,
                vk::AccessFlags::TRANSFER_WRITE,
                vk::AccessFlags::SHADER_READ,
            ),
        ];
        vulk.dev.cmd_pipeline_barrier(
            cb,
            vk::PipelineStageFlags::TRANSFER,
            vk::PipelineStageFlags::COMPUTE_SHADER,
            vk::DependencyFlags::empty(),
            &[],
            &[],
            staging_barrier,
        );

        vulk.dev.cmd_bind_pipeline(
            cb,
            vk::PipelineBindPoint::COMPUTE,
            video.nv12_img_to_rgb.pipeline,
        );

        let push_u8 = pack_glsl_mat3x4(YUV_TO_RGB);
        vulk.dev.cmd_push_constants(
            cb,
            video.nv12_img_to_rgb.pipeline_layout,
            vk::ShaderStageFlags::COMPUTE,
            0,
            &push_u8,
        );

        let bind_descs = &[descriptor_set];
        vulk.dev.cmd_bind_descriptor_sets(
            cb,
            vk::PipelineBindPoint::COMPUTE,
            video.nv12_img_to_rgb.pipeline_layout,
            0,
            bind_descs,
            &[],
        );
        let xgroups = state.target.width.div_ceil(8);
        let ygroups = state.target.height.div_ceil(8);
        vulk.dev.cmd_dispatch(cb, xgroups, ygroups, 1);

        // Only for main image; other barriers are
        let exit_barriers = &[qfot_release_image_memory_barrier(
            state.target.image,
            target_layout,
            vk::ImageLayout::GENERAL,
            vulk.queue_family,
            vk::AccessFlags::SHADER_WRITE,
        )];
        vulk.dev.cmd_pipeline_barrier(
            cb,
            vk::PipelineStageFlags::COMPUTE_SHADER,
            vk::PipelineStageFlags::BOTTOM_OF_PIPE,
            vk::DependencyFlags::empty(),
            &[],
            &[],
            exit_barriers,
        );
        img_inner.image_layout = vk::ImageLayout::GENERAL;
        vkframe.layout[0] = src_layout.as_raw() as _;

        vulk.dev
            .end_command_buffer(cb)
            .map_err(|_| "Failed to end command buffer")?;

        /* Wait for _everything_ to complete -- do not know if graphics/compute/decode is last */
        // vkframe.access not used?
        let waitv_stage_flags = &[vk::PipelineStageFlags::ALL_COMMANDS];
        let cbs = &[cb];

        let mut queue = vulkan_lock_queue(vulk);
        queue.inner.last_semaphore_value += 1;
        let completion_time_point = queue.inner.last_semaphore_value;
        vkframe.sem_value[0] += 1;
        /* Signal vkframe's semaphore to indicate when this operation is done,
         * and main semaphore to notify main loop. */
        let signal_values = &[completion_time_point, vkframe.sem_value[0]];
        let signal_semaphores = &[vulk.semaphore, wait_sems[0]];

        let mut wait_timeline_info = vk::TimelineSemaphoreSubmitInfo::default()
            .wait_semaphore_values(wait_values)
            .signal_semaphore_values(signal_values);
        let submits = &[vk::SubmitInfo::default()
            .command_buffers(cbs)
            .wait_semaphores(wait_sems)
            .wait_dst_stage_mask(waitv_stage_flags)
            .signal_semaphores(signal_semaphores)
            .push_next(&mut wait_timeline_info)];
        vulk.dev
            .queue_submit(queue.inner.queue, submits, vk::Fence::null())
            .map_err(|_| "Queue submit failed")?; // <- can fail with OOM
        drop(queue);

        /* Unlock frame, now that command is submitted. (Note: unlocking before
         * submission could risk timeline semaphore value updates and monotonicity
         * violations */
        avvulk.unlock_frame.as_ref().unwrap()(hwfc_ref, vkframe);

        let mut av_packet_ref = av_packet;
        video
            .bindings
            .av_packet_free(std::ptr::from_mut(&mut av_packet_ref));

        // av_hwframe_transfer_data: does not work, width/height do not match
        // (and dmabuf sharing does not have a reliable way to have "display" dimensions <= "allocated" dimensions)

        let mut frame_ref: *mut AVFrame = frame;
        video.bindings.av_frame_free(&mut frame_ref);

        Ok(VulkanDecodeOpHandle {
            decode: state.clone(),
            pool: pool.clone(),
            data: VulkanDecodeOpData::Hardware(VulkanHWDecodeData {}),
            desc_pool,
            descriptor_set,
            cb,
            completion_time_point,
        })
    }
}

pub fn start_dmavid_decode_sw(
    state: &Arc<VideoDecodeState>,
    pool: &Arc<VulkanCommandPool>,
    packet: &[u8],
) -> Result<VulkanDecodeOpHandle, String> {
    let vulk: &VulkanDevice = &state.target.vulk;
    let video = vulk.video.as_ref().unwrap();

    debug!(
        "Software decoding frame for {}x{} image, packet len {}",
        state.target.width,
        state.target.height,
        packet.len()
    );
    unsafe {
        let av_packet = video.bindings.av_packet_alloc();
        video
            .bindings
            .av_new_packet(av_packet, packet.len().try_into().unwrap());
        (*av_packet).data.copy_from(packet.as_ptr(), packet.len());

        let dec_inner = state.inner.lock().unwrap();

        avcodec_send_packet(&video.bindings, dec_inner.ctx, av_packet)?;

        let frame: *mut AVFrame = video.bindings.av_frame_alloc();

        avcodec_receive_frame(&video.bindings, dec_inner.ctx, frame)?;

        let ext_w = (*dec_inner.ctx).width as usize;
        let ext_h = (*dec_inner.ctx).height as usize;
        assert!(ext_w % 2 == 0 && ext_h % 2 == 0);

        let ystride = (*frame).linesize[0] as usize;
        let ustride = (*frame).linesize[1] as usize;
        let vstride = (*frame).linesize[2] as usize;

        let buf_y = vulkan_get_buffer(&state.target.vulk, ystride * ext_h, true)?;
        let buf_u = vulkan_get_buffer(&state.target.vulk, ustride * (ext_h / 2), true)?;
        let buf_v = vulkan_get_buffer(&state.target.vulk, vstride * (ext_h / 2), true)?;
        let view_y = buf_y.get_write_view();
        let view_u = buf_u.get_write_view();
        let view_v = buf_v.get_write_view();

        {
            // TODO: avoid this copy by implementing AVCodecContext.get_buffer2
            let ydata: &[u8] = &*slice_from_raw_parts((*frame).data[0], ystride * ext_h);
            let udata: &[u8] = &*slice_from_raw_parts((*frame).data[1], ustride * (ext_h / 2));
            let vdata: &[u8] = &*slice_from_raw_parts((*frame).data[2], vstride * (ext_h / 2));

            view_y.data.copy_from_slice(ydata);
            view_u.data.copy_from_slice(udata);
            view_v.data.copy_from_slice(vdata);
        }

        drop(view_y);
        drop(view_u);
        drop(view_v);
        buf_y.complete_write()?;
        buf_u.complete_write()?;
        buf_v.complete_write()?;

        let sizes = &[
            vk::DescriptorPoolSize::default()
                .descriptor_count(1)
                .ty(vk::DescriptorType::STORAGE_IMAGE),
            vk::DescriptorPoolSize::default()
                .descriptor_count(3)
                .ty(vk::DescriptorType::UNIFORM_TEXEL_BUFFER),
        ];
        // at most 1 descriptor set
        let pool_storage_info = vk::DescriptorPoolCreateInfo::default()
            .flags(vk::DescriptorPoolCreateFlags::FREE_DESCRIPTOR_SET)
            .max_sets(1)
            .pool_sizes(sizes);
        let desc_pool = vulk
            .dev
            .create_descriptor_pool(&pool_storage_info, None)
            .map_err(|_| "Failed to create descriptor pool")?;

        let layouts = &[video.yuv420_buf_to_rgb.ds_layout];
        let desc_set_alloc_info = vk::DescriptorSetAllocateInfo::default()
            .descriptor_pool(desc_pool)
            .set_layouts(layouts);
        let descs = vulk
            .dev
            .allocate_descriptor_sets(&desc_set_alloc_info)
            .map_err(|_| "Failed to allocate descriptor sets")?;
        let descriptor_set = descs[0];

        let output_image_info = &[vk::DescriptorImageInfo::default()
            .image_view(state.output_image_view)
            .image_layout(vk::ImageLayout::GENERAL)];
        let buf_y_image_view_info = vk::BufferViewCreateInfo::default()
            .flags(vk::BufferViewCreateFlags::empty())
            .buffer(buf_y.buffer)
            .format(vk::Format::R8_UNORM)
            .offset(0)
            .range(vk::WHOLE_SIZE); // todo: with buffer pooling, precise size will need specifying
        let buf_u_image_view_info = vk::BufferViewCreateInfo::default()
            .flags(vk::BufferViewCreateFlags::empty())
            .buffer(buf_u.buffer)
            .format(vk::Format::R8_UNORM)
            .offset(0)
            .range(vk::WHOLE_SIZE);
        let buf_v_image_view_info = vk::BufferViewCreateInfo::default()
            .flags(vk::BufferViewCreateFlags::empty())
            .buffer(buf_v.buffer)
            .format(vk::Format::R8_UNORM)
            .offset(0)
            .range(vk::WHOLE_SIZE);
        let buf_y_view = vulk
            .dev
            .create_buffer_view(&buf_y_image_view_info, None)
            .map_err(|_| tag!("Failed to create y buffer image view"))?;
        let buf_u_view = vulk
            .dev
            .create_buffer_view(&buf_u_image_view_info, None)
            .map_err(|_| tag!("Failed to create u buffer image view"))?;
        let buf_v_view = vulk
            .dev
            .create_buffer_view(&buf_v_image_view_info, None)
            .map_err(|_| tag!("Failed to create v buffer image view"))?;

        let buf_y_info = &[buf_y_view];
        let buf_u_info = &[buf_u_view];
        let buf_v_info = &[buf_v_view];

        let descriptor_writes = &[
            vk::WriteDescriptorSet::default()
                .dst_set(descriptor_set)
                .dst_binding(0)
                .descriptor_count(1)
                .descriptor_type(vk::DescriptorType::STORAGE_IMAGE)
                .image_info(output_image_info),
            vk::WriteDescriptorSet::default()
                .dst_set(descriptor_set)
                .dst_binding(1)
                .descriptor_count(1)
                .descriptor_type(vk::DescriptorType::UNIFORM_TEXEL_BUFFER)
                .texel_buffer_view(buf_y_info),
            vk::WriteDescriptorSet::default()
                .dst_set(descriptor_set)
                .dst_binding(2)
                .descriptor_count(1)
                .descriptor_type(vk::DescriptorType::UNIFORM_TEXEL_BUFFER)
                .texel_buffer_view(buf_u_info),
            vk::WriteDescriptorSet::default()
                .dst_set(descriptor_set)
                .dst_binding(3)
                .descriptor_count(1)
                .descriptor_type(vk::DescriptorType::UNIFORM_TEXEL_BUFFER)
                .texel_buffer_view(buf_v_info),
        ];
        vulk.dev.update_descriptor_sets(descriptor_writes, &[]);

        let inner_pool = pool.pool.lock().unwrap();

        let alloc_cb_info = vk::CommandBufferAllocateInfo::default()
            .command_pool(*inner_pool)
            .command_buffer_count(1)
            .level(vk::CommandBufferLevel::PRIMARY);
        drop(inner_pool);

        let cbvec = vulk
            .dev
            .allocate_command_buffers(&alloc_cb_info)
            .map_err(|_| "Failed to allocate command buffers")?;
        let cb = cbvec[0];

        let cb_info =
            vk::CommandBufferBeginInfo::default().flags(vk::CommandBufferUsageFlags::empty());
        vulk.dev
            .begin_command_buffer(cb, &cb_info)
            .map_err(|_| "Failed to begin command buffer")?;

        let target_layout = vk::ImageLayout::GENERAL;

        let mut img_inner = state.target.inner.lock().unwrap();
        // note: original contents of image do not need to be preserved as
        // all pixels should be written; can the queue transfer be skipped?
        let entry_barriers = &[qfot_acquire_image_memory_barrier(
            state.target.image,
            img_inner.image_layout,
            target_layout,
            vulk.queue_family,
            vk::AccessFlags::SHADER_WRITE,
        )];
        let buf_memory_barriers = &[
            vk::BufferMemoryBarrier::default()
                .src_access_mask(vk::AccessFlags::HOST_WRITE)
                .dst_access_mask(vk::AccessFlags::SHADER_READ)
                .buffer(buf_y.buffer)
                .offset(0)
                .size(buf_y.buffer_len)
                .src_queue_family_index(vk::QUEUE_FAMILY_IGNORED)
                .dst_queue_family_index(vk::QUEUE_FAMILY_IGNORED),
            vk::BufferMemoryBarrier::default()
                .src_access_mask(vk::AccessFlags::HOST_WRITE)
                .dst_access_mask(vk::AccessFlags::SHADER_READ)
                .buffer(buf_u.buffer)
                .offset(0)
                .size(buf_u.buffer_len)
                .src_queue_family_index(vk::QUEUE_FAMILY_IGNORED)
                .dst_queue_family_index(vk::QUEUE_FAMILY_IGNORED),
            vk::BufferMemoryBarrier::default()
                .src_access_mask(vk::AccessFlags::HOST_WRITE)
                .dst_access_mask(vk::AccessFlags::SHADER_READ)
                .buffer(buf_v.buffer)
                .offset(0)
                .size(buf_v.buffer_len)
                .src_queue_family_index(vk::QUEUE_FAMILY_IGNORED)
                .dst_queue_family_index(vk::QUEUE_FAMILY_IGNORED),
        ];
        vulk.dev.cmd_pipeline_barrier(
            cb,
            vk::PipelineStageFlags::HOST,
            vk::PipelineStageFlags::COMPUTE_SHADER,
            vk::DependencyFlags::empty(),
            &[],
            buf_memory_barriers,
            &[],
        );
        vulk.dev.cmd_pipeline_barrier(
            cb,
            vk::PipelineStageFlags::TOP_OF_PIPE,
            vk::PipelineStageFlags::COMPUTE_SHADER,
            vk::DependencyFlags::empty(),
            &[],
            &[],
            entry_barriers,
        );

        vulk.dev.cmd_bind_pipeline(
            cb,
            vk::PipelineBindPoint::COMPUTE,
            video.yuv420_buf_to_rgb.pipeline,
        );

        let push_u8_mtx = pack_glsl_mat3x4(YUV_TO_RGB);
        let mut push_u8: [u8; 60] = [0; 60];
        push_u8[..48].copy_from_slice(&push_u8_mtx);
        push_u8[48..52].copy_from_slice(&(ystride as i32).to_le_bytes());
        push_u8[52..56].copy_from_slice(&(ustride as i32).to_le_bytes());
        push_u8[56..60].copy_from_slice(&(vstride as i32).to_le_bytes());
        vulk.dev.cmd_push_constants(
            cb,
            video.yuv420_buf_to_rgb.pipeline_layout,
            vk::ShaderStageFlags::COMPUTE,
            0,
            &push_u8,
        );

        let bind_descs = &[descriptor_set];
        vulk.dev.cmd_bind_descriptor_sets(
            cb,
            vk::PipelineBindPoint::COMPUTE,
            video.yuv420_buf_to_rgb.pipeline_layout,
            0,
            bind_descs,
            &[],
        );
        let xgroups = state.target.width.div_ceil(8);
        let ygroups = state.target.height.div_ceil(8);
        vulk.dev.cmd_dispatch(cb, xgroups, ygroups, 1);

        // Only for main image; other barriers are
        let exit_barriers = &[qfot_release_image_memory_barrier(
            state.target.image,
            target_layout,
            vk::ImageLayout::GENERAL,
            vulk.queue_family,
            vk::AccessFlags::SHADER_WRITE,
        )];
        vulk.dev.cmd_pipeline_barrier(
            cb,
            vk::PipelineStageFlags::COMPUTE_SHADER,
            vk::PipelineStageFlags::BOTTOM_OF_PIPE,
            vk::DependencyFlags::empty(),
            &[],
            &[],
            exit_barriers,
        );
        img_inner.image_layout = vk::ImageLayout::GENERAL;

        vulk.dev
            .end_command_buffer(cb)
            .map_err(|_| "Failed to end command buffer")?;

        let cbs = &[cb];

        let mut queue = vulkan_lock_queue(vulk);
        queue.inner.last_semaphore_value += 1;
        let completion_time_point = queue.inner.last_semaphore_value;
        /* Signal vkframe's semaphore to indicate when this operation is done,
         * and main semaphore to notify main loop. */
        let signal_values = &[completion_time_point];
        let signal_semaphores = &[vulk.semaphore];

        let mut wait_timeline_info = vk::TimelineSemaphoreSubmitInfo::default()
            .wait_semaphore_values(&[])
            .signal_semaphore_values(signal_values);
        let submits = &[vk::SubmitInfo::default()
            .command_buffers(cbs)
            .wait_semaphores(&[])
            .wait_dst_stage_mask(&[])
            .signal_semaphores(signal_semaphores)
            .push_next(&mut wait_timeline_info)];
        vulk.dev
            .queue_submit(queue.inner.queue, submits, vk::Fence::null())
            .map_err(|_| "Queue submit failed")?; // <- can fail with OOM
        drop(queue);

        let mut av_packet_ref = av_packet;
        video
            .bindings
            .av_packet_free(std::ptr::from_mut(&mut av_packet_ref));

        let mut frame_ref: *mut AVFrame = frame;
        video.bindings.av_frame_free(&mut frame_ref);

        Ok(VulkanDecodeOpHandle {
            decode: state.clone(),
            pool: pool.clone(),
            data: VulkanDecodeOpData::Software(VulkanSWDecodeData {
                _buf_y: buf_y,
                _buf_u: buf_u,
                _buf_v: buf_v,
                buf_y_view,
                buf_u_view,
                buf_v_view,
            }),
            desc_pool,
            descriptor_set,
            cb,
            completion_time_point,
        })
    }
}

pub fn start_dmavid_apply(
    state: &Arc<VideoDecodeState>,
    pool: &Arc<VulkanCommandPool>,
    packet: &[u8],
) -> Result<VulkanDecodeOpHandle, String> {
    if matches!(state.data, VideoDecodeStateData::SW) {
        start_dmavid_decode_sw(state, pool, packet)
    } else {
        start_dmavid_decode_hw(state, pool, packet)
    }
}

/* kv must contain _null terminated_ strings */
fn build_av_dict(bindings: &ffmpeg, kv: &[(&[u8], &[u8])]) -> Result<*mut AVDictionary, String> {
    let mut options = std::ptr::null_mut();
    for (k, v) in kv.iter() {
        assert!(k.ends_with(&[0]) && v.ends_with(&[0]));
        unsafe {
            // SAFETY: null termination verified;
            // todo:
            let r = bindings.av_dict_set(
                &mut options,
                k.as_ptr() as *const c_char,
                v.as_ptr() as *const c_char,
                0,
            );
            if r < 0 {
                bindings.av_dict_free(&mut options);
                return Err(tag!("Failed to set key/value pair in dictionary"));
            }
        }
    }
    Ok(options)
}

pub fn setup_video_encode_hw(
    img: &Arc<VulkanDmabuf>,
    fmt: VideoFormat,
    bpf: Option<f32>,
) -> Result<VideoEncodeState, String> {
    let video = img.vulk.video.as_ref().unwrap();

    let encoder: *const AVCodec = match fmt {
        VideoFormat::H264 => video.codecs_h264.encoder,
        VideoFormat::VP9 => video.codecs_vp9.encoder,
        VideoFormat::AV1 => video.codecs_av1.encoder,
    };
    assert!(!encoder.is_null());
    unsafe {
        let ctx = video.bindings.avcodec_alloc_context3(encoder);
        if ctx.is_null() {
            return Err(tag!("Failed to allocate codec context"));
        }

        let hctx_ref = video.bindings.av_hwframe_ctx_alloc(video.av_hwdevice);
        if hctx_ref.is_null() {
            return Err(tag!("Failed to allocate hardware frames context"));
        }

        let (frame_width, frame_height) = align_size(img.width, img.height, fmt);

        {
            let hr = (*hctx_ref)
                .data
                .cast::<AVHWFramesContext>()
                .as_mut()
                .unwrap();
            hr.format = AVPixelFormat_AV_PIX_FMT_VULKAN;
            hr.sw_format = AVPixelFormat_AV_PIX_FMT_NV12;
            hr.height = frame_height;
            hr.width = frame_width;
        }

        av_hwframe_ctx_init(&video.bindings, hctx_ref)?;

        {
            let cr = ctx.as_mut().unwrap();

            let nref = video.bindings.av_buffer_ref(video.av_hwdevice);
            if nref.is_null() {
                return Err(tag!("Failed to add reference for av_hwdevice"));
            }
            cr.hw_device_ctx = nref;

            cr.hw_frames_ctx = hctx_ref;

            cr.width = frame_width;
            cr.height = frame_height;

            /* Arbitrary, since Waypipe currently does per-buffer video instead of per-surface */
            let nom_fps = 100;
            cr.time_base = AVRational {
                num: 1,
                den: nom_fps,
            };
            cr.framerate = AVRational {
                num: nom_fps,
                den: 1,
            };

            /* Streaming, no latency, only I and P frames */
            cr.delay = 0;
            cr.max_b_frames = 0;

            // todo: instead of bpf, use a 'bpp' -- bits-per-pixel equivalent, which scales
            // properly with image size. Or crf?
            let b = bpf.unwrap_or(1e5);
            // todo: sanity checks
            cr.bit_rate = (b * (nom_fps as f32)) as i64;

            cr.pix_fmt = AVPixelFormat_AV_PIX_FMT_VULKAN;
            // cr.color_range = AVColorRange_AVCOL_RANGE_MPEG;
        }

        /* Encoder specific options */
        let mut options = build_av_dict(
            &video.bindings,
            &[
                (b"tune\0", b"ull\0"),
                (b"usage\0", b"stream\0"),
                (b"async_depth\0", b"1\0"),
            ],
        )?;

        avcodec_open(&video.bindings, ctx, encoder, &mut options)?;
        video.bindings.av_dict_free(&mut options);

        let output_image_view = create_dmabuf_view(img)?;

        let staging_images = create_staging_images(
            &img.vulk,
            frame_width.try_into().unwrap(),
            frame_height.try_into().unwrap(),
            true,
            vk::Format::G8_B8R8_2PLANE_420_UNORM,
        )?;

        Ok(VideoEncodeState {
            target: img.clone(),
            inner: Mutex::new(VideoEncodeInner { ctx }),
            output_image_view,
            data: VideoEncodeStateData::HW(staging_images),
        })
    }
}

pub fn setup_video_encode_sw(
    img: &Arc<VulkanDmabuf>,
    fmt: VideoFormat,
    bpf: Option<f32>,
) -> Result<VideoEncodeState, String> {
    let video = img.vulk.video.as_ref().unwrap();

    let sw_encoder: *const AVCodec = match fmt {
        VideoFormat::H264 => video.codecs_h264.sw_encoder,
        VideoFormat::VP9 => video.codecs_vp9.sw_encoder,
        VideoFormat::AV1 => video.codecs_av1.sw_encoder,
    };
    assert!(!sw_encoder.is_null());
    unsafe {
        let ctx = video.bindings.avcodec_alloc_context3(sw_encoder);
        if ctx.is_null() {
            return Err(tag!("Failed to allocate codec context"));
        }

        let (frame_width, frame_height) = align_size(img.width, img.height, fmt);

        {
            let cr = ctx.as_mut().unwrap();

            cr.width = frame_width;
            cr.height = frame_height;

            /* Arbitrary, since Waypipe currently does per-buffer video instead of per-surface */
            let nom_fps = 100;
            cr.time_base = AVRational {
                num: 1,
                den: nom_fps,
            };
            cr.framerate = AVRational {
                num: nom_fps,
                den: 1,
            };

            /* Streaming, no latency, only I and P frames */
            cr.delay = 0;
            cr.max_b_frames = 0;

            // todo: instead of bpf, use a 'bpp' -- bits-per-pixel equivalent, which scales
            // properly with image size. Or crf?
            let b = bpf.unwrap_or(1e5);
            // todo: sanity checks
            cr.bit_rate = (b * (nom_fps as f32)) as i64;

            cr.pix_fmt = AVPixelFormat_AV_PIX_FMT_YUV420P;
            // cr.color_range = AVColorRange_AVCOL_RANGE_MPEG;
        }

        /* Encoder specific options. In general, minimize latency */
        let mut options = match fmt {
            VideoFormat::H264 => build_av_dict(
                &video.bindings,
                &[(b"tune\0", b"zerolatency\0"), (b"preset\0", b"ultrafast\0")],
            )?,
            VideoFormat::VP9 => build_av_dict(
                &video.bindings,
                &[
                    (b"lag-in-frames\0", b"0\0"),
                    (b"deadline\0", b"realtime\0"),
                    (b"quality\0", b"realtime\0"),
                    (b"cpu-used\0", b"8\0"),
                ],
            )?,
            VideoFormat::AV1 => build_av_dict(
                &video.bindings,
                &[
                    (b"usage\0", b"realtime\0"),
                    (b"lag-in-frames\0", b"0\0"),
                    (b"cpu-used\0", b"8\0"),
                ],
            )?,
        };

        avcodec_open(&video.bindings, ctx, sw_encoder, &mut options)?;
        video.bindings.av_dict_free(&mut options);

        let output_image_view = create_dmabuf_view(img)?;

        Ok(VideoEncodeState {
            target: img.clone(),
            inner: Mutex::new(VideoEncodeInner { ctx }),
            output_image_view,
            data: VideoEncodeStateData::SW,
        })
    }
}

pub fn setup_video_encode(
    img: &Arc<VulkanDmabuf>,
    fmt: VideoFormat,
    bpf: Option<f32>,
) -> Result<VideoEncodeState, String> {
    assert!(img.can_store_and_sample);
    let video = img.vulk.video.as_ref().unwrap();
    let can_hw_encode = match fmt {
        VideoFormat::AV1 => {
            video.can_hw_enc_av1
                && !video.codecs_av1.encoder.is_null()
                && (img.width >= MIN_AV1_HW_ENC_SIZE.0 && img.height >= MIN_AV1_HW_ENC_SIZE.1)
        }
        VideoFormat::VP9 => false,
        VideoFormat::H264 => {
            video.can_hw_enc_h264
                && !video.codecs_h264.encoder.is_null()
                && (img.width >= MIN_H264_HW_SIZE.0 && img.height >= MIN_H264_HW_SIZE.1)
        }
    };
    if can_hw_encode {
        setup_video_encode_hw(img, fmt, bpf)
    } else {
        setup_video_encode_sw(img, fmt, bpf)
    }
}

pub fn start_dmavid_encode_hw(
    state: &Arc<VideoEncodeState>,
    pool: &Arc<VulkanCommandPool>,
    wait_semaphores: &[(Arc<VulkanTimelineSemaphore>, u64)],
    wait_binary_semaphores: &[VulkanBinarySemaphore],
) -> Result<Vec<u8>, String> {
    let vulk: &VulkanDevice = &state.target.vulk;
    let video = vulk.video.as_ref().unwrap();

    let VideoEncodeStateData::HW(ref state_data) = state.data else {
        unreachable!()
    };
    debug!(
        "Hardware encoding frame for {}x{} image",
        state.target.width, state.target.height
    );
    unsafe {
        let enc_inner = state.inner.lock().unwrap();
        let hwframe_ctx_ref = (*enc_inner.ctx).hw_frames_ctx;

        let frame: *mut AVFrame = video.bindings.av_frame_alloc();
        if frame.is_null() {
            return Err(tag!("Failed to allocate frame"));
        }

        let get_buf_ret = video
            .bindings
            .av_hwframe_get_buffer(hwframe_ctx_ref, frame, 0);
        if get_buf_ret != 0 {
            return Err(tag!("Failed to get buffer for frame: {}", get_buf_ret));
        }
        let hw_fr_ref = (*frame).hw_frames_ctx.as_ref().unwrap();
        let hwfc_ref = hw_fr_ref.data.cast::<AVHWFramesContext>().as_mut().unwrap();
        let vk_fc = hwfc_ref
            .hwctx
            .cast::<AVVulkanFramesContext>()
            .as_mut()
            .unwrap();
        let vkframe = ((*frame).data[0]).cast::<AVVkFrame>().as_mut().unwrap();
        /* Lock frame, to prevent concurrent modifications */
        vk_fc.lock_frame.as_ref().unwrap()(hwfc_ref, vkframe);

        assert!(vk_fc.format[0] == vk::Format::G8_B8R8_2PLANE_420_UNORM.as_raw() as _);
        assert!(vkframe.img[1..]
            .iter()
            .all(|x| vk::Image::from_raw(*x as _).is_null()));

        /* Blocking wait for semaphores; remove this later */
        let mut wait_sems = vec![vk::Semaphore::from_raw(vkframe.sem[0] as _)];
        let mut wait_values = vec![vkframe.sem_value[0]];
        wait_sems.extend(wait_semaphores.iter().map(|x| x.0.semaphore));
        wait_values.extend(wait_semaphores.iter().map(|x| x.1));
        wait_sems.extend(wait_binary_semaphores.iter().map(|x| x.semaphore));
        wait_values.extend(wait_binary_semaphores.iter().map(|_| u64::MAX));

        let init_layout = vkframe.layout[0];
        let dst_img = vk::Image::from_raw(vkframe.img[0] as _);

        let sizes = &[
            vk::DescriptorPoolSize::default()
                .descriptor_count(1)
                .ty(vk::DescriptorType::COMBINED_IMAGE_SAMPLER),
            vk::DescriptorPoolSize::default()
                .descriptor_count(2)
                .ty(vk::DescriptorType::STORAGE_IMAGE),
        ];
        // at most 1 descriptor set
        let pool_storage_info = vk::DescriptorPoolCreateInfo::default()
            .flags(vk::DescriptorPoolCreateFlags::FREE_DESCRIPTOR_SET)
            .max_sets(1)
            .pool_sizes(sizes);
        let desc_pool = vulk
            .dev
            .create_descriptor_pool(&pool_storage_info, None)
            .map_err(|_| "Failed to create descriptor pool")?;

        let layouts = &[video.rgb_to_nv12_img.ds_layout];
        let desc_set_alloc_info = vk::DescriptorSetAllocateInfo::default()
            .descriptor_pool(desc_pool)
            .set_layouts(layouts);
        let descs = vulk
            .dev
            .allocate_descriptor_sets(&desc_set_alloc_info)
            .map_err(|_| "Failed to allocate descriptor sets")?;
        let descriptor_set = descs[0];

        let output_image_info = &[vk::DescriptorImageInfo::default()
            .image_view(state.output_image_view)
            .image_layout(vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL)
            .sampler(video.rgb_to_yuv_sampler_rgb)];
        let input_1_image_info = &[vk::DescriptorImageInfo::default()
            .image_view(state_data.plane_image_views[0])
            .image_layout(vk::ImageLayout::GENERAL)];
        let input_2_image_info = &[vk::DescriptorImageInfo::default()
            .image_view(state_data.plane_image_views[1])
            .image_layout(vk::ImageLayout::GENERAL)];

        let descriptor_writes = &[
            vk::WriteDescriptorSet::default()
                .dst_set(descriptor_set)
                .dst_binding(0)
                .descriptor_count(1)
                .descriptor_type(vk::DescriptorType::COMBINED_IMAGE_SAMPLER)
                .image_info(output_image_info),
            vk::WriteDescriptorSet::default()
                .dst_set(descriptor_set)
                .dst_binding(1)
                .descriptor_count(1)
                .descriptor_type(vk::DescriptorType::STORAGE_IMAGE)
                .image_info(input_1_image_info),
            vk::WriteDescriptorSet::default()
                .dst_set(descriptor_set)
                .dst_binding(2)
                .descriptor_count(1)
                .descriptor_type(vk::DescriptorType::STORAGE_IMAGE)
                .image_info(input_2_image_info),
        ];
        vulk.dev.update_descriptor_sets(descriptor_writes, &[]);

        let inner_pool = pool.pool.lock().unwrap();

        let alloc_cb_info = vk::CommandBufferAllocateInfo::default()
            .command_pool(*inner_pool)
            .command_buffer_count(1)
            .level(vk::CommandBufferLevel::PRIMARY);
        drop(inner_pool);

        let cbvec = vulk
            .dev
            .allocate_command_buffers(&alloc_cb_info)
            .map_err(|_| "Failed to allocate command buffers")?;
        let cb = cbvec[0];
        // TODO: figure out proper pipeline barriers & queue transfers
        // want image memory barriers on all three images

        let cb_info =
            vk::CommandBufferBeginInfo::default().flags(vk::CommandBufferUsageFlags::empty());
        vulk.dev
            .begin_command_buffer(cb, &cb_info)
            .map_err(|_| "Failed to begin command buffer")?;

        let target_layout = vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL;
        let dst_layout = vk::ImageLayout::TRANSFER_DST_OPTIMAL;

        let standard_access_range = vk::ImageSubresourceRange::default()
            .aspect_mask(vk::ImageAspectFlags::COLOR)
            .level_count(1)
            .layer_count(1);
        let mut img_inner = state.target.inner.lock().unwrap();
        let entry_barriers = &[qfot_acquire_image_memory_barrier(
            state.target.image,
            img_inner.image_layout,
            target_layout,
            vulk.queue_family,
            vk::AccessFlags::SHADER_READ,
        )];
        vulk.dev.cmd_pipeline_barrier(
            cb,
            vk::PipelineStageFlags::TOP_OF_PIPE,
            vk::PipelineStageFlags::COMPUTE_SHADER,
            vk::DependencyFlags::empty(),
            &[],
            &[],
            entry_barriers,
        );
        /* Set staging image layout and discard old contents */
        let staging_entry_barriers = &[
            image_layout_transition(
                state_data.plane_images[0],
                vk::ImageLayout::UNDEFINED,
                vk::ImageLayout::GENERAL,
                vk::AccessFlags::TRANSFER_READ,
                vk::AccessFlags::SHADER_WRITE,
            ),
            image_layout_transition(
                state_data.plane_images[1],
                vk::ImageLayout::UNDEFINED,
                vk::ImageLayout::GENERAL,
                vk::AccessFlags::TRANSFER_READ,
                vk::AccessFlags::SHADER_WRITE,
            ),
        ];
        vulk.dev.cmd_pipeline_barrier(
            cb,
            vk::PipelineStageFlags::TRANSFER,
            vk::PipelineStageFlags::COMPUTE_SHADER,
            vk::DependencyFlags::empty(),
            &[],
            &[],
            staging_entry_barriers,
        );

        vulk.dev.cmd_bind_pipeline(
            cb,
            vk::PipelineBindPoint::COMPUTE,
            video.rgb_to_nv12_img.pipeline,
        );
        let bind_descs = &[descriptor_set];
        vulk.dev.cmd_bind_descriptor_sets(
            cb,
            vk::PipelineBindPoint::COMPUTE,
            video.rgb_to_nv12_img.pipeline_layout,
            0,
            bind_descs,
            &[],
        );
        let push_u8 = pack_glsl_mat3x4(RGB_TO_YUV);
        vulk.dev.cmd_push_constants(
            cb,
            video.rgb_to_nv12_img.pipeline_layout,
            vk::ShaderStageFlags::COMPUTE,
            0,
            &push_u8,
        );
        /* Fill every pixel of the Y and CbCr planes */
        assert!(hwfc_ref.width % 16 == 0);
        assert!(hwfc_ref.height % 16 == 0);
        let xgroups = (hwfc_ref.width / 16) as u32;
        let ygroups = (hwfc_ref.height / 16) as u32;
        vulk.dev.cmd_dispatch(cb, xgroups, ygroups, 1);

        let exit_barriers = &[qfot_release_image_memory_barrier(
            state.target.image,
            target_layout,
            vk::ImageLayout::GENERAL,
            vulk.queue_family,
            vk::AccessFlags::SHADER_READ,
        )];
        vulk.dev.cmd_pipeline_barrier(
            cb,
            vk::PipelineStageFlags::COMPUTE_SHADER,
            vk::PipelineStageFlags::BOTTOM_OF_PIPE,
            vk::DependencyFlags::empty(),
            &[],
            &[],
            exit_barriers,
        );
        let post_compute_barriers = &[vk::ImageMemoryBarrier::default()
            .image(dst_img)
            .old_layout(vk::ImageLayout::from_raw(init_layout as _))
            .new_layout(dst_layout)
            .src_queue_family_index(vk::QUEUE_FAMILY_IGNORED)
            .dst_queue_family_index(vk::QUEUE_FAMILY_IGNORED)
            .src_access_mask(vk::AccessFlags::MEMORY_WRITE)
            .dst_access_mask(vk::AccessFlags::MEMORY_WRITE)
            .subresource_range(standard_access_range)];
        vulk.dev.cmd_pipeline_barrier(
            cb,
            vk::PipelineStageFlags::TOP_OF_PIPE,
            vk::PipelineStageFlags::TRANSFER,
            vk::DependencyFlags::empty(),
            &[],
            &[],
            post_compute_barriers,
        );
        let staging_tx_barriers = &[
            image_layout_transition(
                state_data.plane_images[0],
                vk::ImageLayout::GENERAL,
                vk::ImageLayout::TRANSFER_SRC_OPTIMAL,
                vk::AccessFlags::SHADER_WRITE,
                vk::AccessFlags::TRANSFER_READ,
            ),
            image_layout_transition(
                state_data.plane_images[1],
                vk::ImageLayout::GENERAL,
                vk::ImageLayout::TRANSFER_SRC_OPTIMAL,
                vk::AccessFlags::SHADER_WRITE,
                vk::AccessFlags::TRANSFER_READ,
            ),
        ];
        vulk.dev.cmd_pipeline_barrier(
            cb,
            vk::PipelineStageFlags::COMPUTE_SHADER,
            vk::PipelineStageFlags::TRANSFER,
            vk::DependencyFlags::empty(),
            &[],
            &[],
            staging_tx_barriers,
        );

        let copy_plane_1 = &[vk::ImageCopy {
            src_subresource: subresource_layer(vk::ImageAspectFlags::COLOR),
            src_offset: vk::Offset3D::default(),
            dst_subresource: subresource_layer(vk::ImageAspectFlags::PLANE_0),
            dst_offset: vk::Offset3D::default(),
            extent: vk::Extent3D {
                width: hwfc_ref.width as u32,
                height: hwfc_ref.height as u32,
                depth: 1,
            },
        }];
        let copy_plane_2 = &[vk::ImageCopy {
            src_subresource: subresource_layer(vk::ImageAspectFlags::COLOR),
            src_offset: vk::Offset3D::default(),
            dst_subresource: subresource_layer(vk::ImageAspectFlags::PLANE_1),
            dst_offset: vk::Offset3D::default(),
            extent: vk::Extent3D {
                width: (hwfc_ref.width as u32) / 2,
                height: (hwfc_ref.height as u32) / 2,
                depth: 1,
            },
        }];
        vulk.dev.cmd_copy_image(
            cb,
            state_data.plane_images[0],
            vk::ImageLayout::TRANSFER_SRC_OPTIMAL,
            dst_img,
            dst_layout,
            copy_plane_1,
        );
        vulk.dev.cmd_copy_image(
            cb,
            state_data.plane_images[1],
            vk::ImageLayout::TRANSFER_SRC_OPTIMAL,
            dst_img,
            dst_layout,
            copy_plane_2,
        );

        // ffmpeg will handle barriers / layout transitions for the vkframe
        img_inner.image_layout = vk::ImageLayout::GENERAL;
        vkframe.layout[0] = dst_layout.as_raw() as _;

        vulk.dev
            .end_command_buffer(cb)
            .map_err(|_| "Failed to end command buffer")?;

        /* Wait for _everything_ to complete -- do not know if graphics/compute/decode is last */
        // vkframe.access not used?
        let waitv_stage_flags = vec![vk::PipelineStageFlags::ALL_COMMANDS; wait_values.len()];
        let cbs = &[cb];

        vkframe.sem_value[0] += 1;
        /* Signal vkframe's semaphore to indicate when this operation is done,
         * and main semaphore to notify main loop. */
        let signal_values = &[vkframe.sem_value[0]];
        let signal_semaphores = &[wait_sems[0]];

        let mut wait_timeline_info = vk::TimelineSemaphoreSubmitInfo::default()
            .wait_semaphore_values(&wait_values)
            .signal_semaphore_values(signal_values);
        let submits = &[vk::SubmitInfo::default()
            .command_buffers(cbs)
            .wait_semaphores(&wait_sems)
            .wait_dst_stage_mask(&waitv_stage_flags)
            .signal_semaphores(signal_semaphores)
            .push_next(&mut wait_timeline_info)];

        let queue = vulkan_lock_queue(vulk);
        vulk.dev
            .queue_submit(queue.inner.queue, submits, vk::Fence::null())
            .map_err(|_| "Queue submit failed")?; // <- can fail with OOM
        drop(queue);

        /* Unlock frame, now that command is submitted. (Note: unlocking before
         * submission could risk timeline semaphore value updates and monotonicity
         * violations */
        vk_fc.unlock_frame.as_ref().unwrap()(hwfc_ref, vkframe);

        avcodec_send_frame(&video.bindings, enc_inner.ctx, frame)?;

        let mut packet: *mut AVPacket = video.bindings.av_packet_alloc();

        avcodec_receive_packet(&video.bindings, enc_inner.ctx, packet)?;

        /* Cleanup; receiving packet should mean preceding operation is entirely done? */
        let inner_pool = pool.pool.lock().unwrap();
        vulk.dev.free_command_buffers(*inner_pool, &[cb]);
        vulk.dev
            .free_descriptor_sets(desc_pool, &[descriptor_set])
            .map_err(|_| "Failed to free descriptor set")
            .unwrap();
        vulk.dev.destroy_descriptor_pool(desc_pool, None);

        let mut f = frame;
        video.bindings.av_frame_free(&mut f);

        let data = std::slice::from_raw_parts((*packet).data, (*packet).size.try_into().unwrap());
        let mut packet_data = Vec::<u8>::new();
        packet_data.extend_from_slice(data);

        video.bindings.av_packet_free(&mut packet);
        Ok(packet_data)
    }
}

pub fn start_dmavid_encode_sw(
    state: &Arc<VideoEncodeState>,
    pool: &Arc<VulkanCommandPool>,
    wait_semaphores: &[(Arc<VulkanTimelineSemaphore>, u64)],
    wait_binary_semaphores: &[VulkanBinarySemaphore],
) -> Result<Vec<u8>, String> {
    let vulk: &VulkanDevice = &state.target.vulk;
    let video = vulk.video.as_ref().unwrap();

    debug!(
        "Software encoding a frame for {}x{} image",
        state.target.width, state.target.height
    );
    unsafe {
        let frame: *mut AVFrame = video.bindings.av_frame_alloc();
        if frame.is_null() {
            return Err(tag!("Failed to allocate frame"));
        }

        let enc_inner = state.inner.lock().unwrap();
        // Y plane
        let ext_w = (*enc_inner.ctx).width as usize;
        let ext_h = (*enc_inner.ctx).height as usize;
        assert!(ext_w % 2 == 0 && ext_h % 2 == 0);

        let mut w: i32 = (*enc_inner.ctx).width;
        let mut h: i32 = (*enc_inner.ctx).height;
        let mut line_alignments: [i32; AV_NUM_DATA_POINTERS as usize] = [0, 0, 0, 0, 0, 0, 0, 0];
        // todo: variable alignment?
        // TODO: can be computed _once_ and cached
        video.bindings.avcodec_align_dimensions2(
            enc_inner.ctx,
            &mut w,
            &mut h,
            &mut line_alignments as *mut i32,
        );
        assert!(w as usize == ext_w);
        assert!(h as usize >= ext_h);

        // TODO: handle ffmpeg extra height request to allow overreading
        let stride_y = align(ext_w, line_alignments[0].try_into().unwrap()); // 1bpp, no subsampling
        let stride_u = align(ext_w / 2, line_alignments[1].try_into().unwrap()); // 1bpp and 2x subsampled
        let stride_v = align(ext_w / 2, line_alignments[2].try_into().unwrap()); // 1bpp and 2x subsampled

        let buf_y = vulkan_get_buffer(&state.target.vulk, stride_y * ext_h, true)?;
        let buf_u = vulkan_get_buffer(&state.target.vulk, stride_u * (ext_h / 2), true)?;
        let buf_v = vulkan_get_buffer(&state.target.vulk, stride_v * (ext_h / 2), true)?;

        let sizes = &[
            vk::DescriptorPoolSize::default()
                .descriptor_count(1)
                .ty(vk::DescriptorType::COMBINED_IMAGE_SAMPLER),
            vk::DescriptorPoolSize::default()
                .descriptor_count(3)
                .ty(vk::DescriptorType::STORAGE_TEXEL_BUFFER),
        ];
        // at most 1 descriptor set
        let pool_storage_info = vk::DescriptorPoolCreateInfo::default()
            .flags(vk::DescriptorPoolCreateFlags::FREE_DESCRIPTOR_SET)
            .max_sets(1)
            .pool_sizes(sizes);
        let desc_pool = vulk
            .dev
            .create_descriptor_pool(&pool_storage_info, None)
            .map_err(|_| "Failed to create descriptor pool")?;

        let layouts = &[video.rgb_to_yuv420_buf.ds_layout];
        let desc_set_alloc_info = vk::DescriptorSetAllocateInfo::default()
            .descriptor_pool(desc_pool)
            .set_layouts(layouts);
        let descs = vulk
            .dev
            .allocate_descriptor_sets(&desc_set_alloc_info)
            .map_err(|_| "Failed to allocate descriptor sets")?;
        let descriptor_set = descs[0];

        let buf_y_image_view_info = vk::BufferViewCreateInfo::default()
            .flags(vk::BufferViewCreateFlags::empty())
            .buffer(buf_y.buffer)
            .format(vk::Format::R8_UNORM)
            .offset(0)
            .range(vk::WHOLE_SIZE); // todo: with buffer pooling, precise size will need specifying
        let buf_u_image_view_info = vk::BufferViewCreateInfo::default()
            .flags(vk::BufferViewCreateFlags::empty())
            .buffer(buf_u.buffer)
            .format(vk::Format::R8_UNORM)
            .offset(0)
            .range(vk::WHOLE_SIZE);
        let buf_v_image_view_info = vk::BufferViewCreateInfo::default()
            .flags(vk::BufferViewCreateFlags::empty())
            .buffer(buf_v.buffer)
            .format(vk::Format::R8_UNORM)
            .offset(0)
            .range(vk::WHOLE_SIZE);
        let buf_y_image_view = vulk
            .dev
            .create_buffer_view(&buf_y_image_view_info, None)
            .map_err(|_| tag!("Failed to create y buffer image view"))?;
        let buf_u_image_view = vulk
            .dev
            .create_buffer_view(&buf_u_image_view_info, None)
            .map_err(|_| tag!("Failed to create u buffer image view"))?;
        let buf_v_image_view = vulk
            .dev
            .create_buffer_view(&buf_v_image_view_info, None)
            .map_err(|_| tag!("Failed to create v buffer image view"))?;

        let output_image_info = &[vk::DescriptorImageInfo::default()
            .image_view(state.output_image_view)
            .image_layout(vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL)
            .sampler(video.rgb_to_yuv_sampler_rgb)];
        let buf_y_info = &[buf_y_image_view];
        let buf_u_info = &[buf_u_image_view];
        let buf_v_info = &[buf_v_image_view];

        let descriptor_writes = &[
            vk::WriteDescriptorSet::default()
                .dst_set(descriptor_set)
                .dst_binding(0)
                .descriptor_count(1)
                .descriptor_type(vk::DescriptorType::COMBINED_IMAGE_SAMPLER)
                .image_info(output_image_info),
            vk::WriteDescriptorSet::default()
                .dst_set(descriptor_set)
                .dst_binding(1)
                .descriptor_count(1)
                .descriptor_type(vk::DescriptorType::STORAGE_TEXEL_BUFFER)
                .texel_buffer_view(buf_y_info),
            vk::WriteDescriptorSet::default()
                .dst_set(descriptor_set)
                .dst_binding(2)
                .descriptor_count(1)
                .descriptor_type(vk::DescriptorType::STORAGE_TEXEL_BUFFER)
                .texel_buffer_view(buf_u_info),
            vk::WriteDescriptorSet::default()
                .dst_set(descriptor_set)
                .dst_binding(3)
                .descriptor_count(1)
                .descriptor_type(vk::DescriptorType::STORAGE_TEXEL_BUFFER)
                .texel_buffer_view(buf_v_info),
        ];
        vulk.dev.update_descriptor_sets(descriptor_writes, &[]);

        let inner_pool = pool.pool.lock().unwrap();

        let alloc_cb_info = vk::CommandBufferAllocateInfo::default()
            .command_pool(*inner_pool)
            .command_buffer_count(1)
            .level(vk::CommandBufferLevel::PRIMARY);
        drop(inner_pool);

        let cbvec = vulk
            .dev
            .allocate_command_buffers(&alloc_cb_info)
            .map_err(|_| "Failed to allocate command buffers")?;
        let cb = cbvec[0];
        // TODO: figure out proper pipeline barriers & queue transfers
        // want image memory barriers on all three images

        let cb_info =
            vk::CommandBufferBeginInfo::default().flags(vk::CommandBufferUsageFlags::empty());
        vulk.dev
            .begin_command_buffer(cb, &cb_info)
            .map_err(|_| "Failed to begin command buffer")?;

        let target_layout = vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL;

        let mut img_inner = state.target.inner.lock().unwrap();
        let entry_barriers = &[qfot_acquire_image_memory_barrier(
            state.target.image,
            img_inner.image_layout,
            target_layout,
            vulk.queue_family,
            vk::AccessFlags::SHADER_READ,
        )];
        vulk.dev.cmd_pipeline_barrier(
            cb,
            vk::PipelineStageFlags::TOP_OF_PIPE,
            vk::PipelineStageFlags::COMPUTE_SHADER,
            vk::DependencyFlags::empty(),
            &[],
            &[],
            entry_barriers,
        );

        vulk.dev.cmd_bind_pipeline(
            cb,
            vk::PipelineBindPoint::COMPUTE,
            video.rgb_to_yuv420_buf.pipeline,
        );
        let bind_descs = &[descriptor_set];
        vulk.dev.cmd_bind_descriptor_sets(
            cb,
            vk::PipelineBindPoint::COMPUTE,
            video.rgb_to_yuv420_buf.pipeline_layout,
            0,
            bind_descs,
            &[],
        );
        let push_u8_mtx = pack_glsl_mat3x4(RGB_TO_YUV);
        let mut push_u8: [u8; 60] = [0; 60];
        push_u8[..48].copy_from_slice(&push_u8_mtx);
        push_u8[48..52].copy_from_slice(&(stride_y as i32).to_le_bytes());
        push_u8[52..56].copy_from_slice(&(stride_u as i32).to_le_bytes());
        push_u8[56..60].copy_from_slice(&(stride_v as i32).to_le_bytes());
        vulk.dev.cmd_push_constants(
            cb,
            video.rgb_to_yuv420_buf.pipeline_layout,
            vk::ShaderStageFlags::COMPUTE,
            0,
            &push_u8,
        );

        /* Fill every pixel of the Y and CbCr planes */
        assert!(ext_w % 16 == 0);
        assert!(ext_h % 16 == 0);
        let xgroups = (ext_w / 16) as u32;
        let ygroups = (ext_h / 16) as u32;
        vulk.dev.cmd_dispatch(cb, xgroups, ygroups, 1);

        // Only for main image; buffers
        let exit_barriers = &[qfot_release_image_memory_barrier(
            state.target.image,
            target_layout,
            vk::ImageLayout::GENERAL,
            vulk.queue_family,
            vk::AccessFlags::SHADER_READ,
        )];
        let buf_memory_barriers = &[
            vk::BufferMemoryBarrier::default()
                .src_access_mask(vk::AccessFlags::SHADER_WRITE)
                .dst_access_mask(vk::AccessFlags::HOST_READ)
                .buffer(buf_y.buffer)
                .offset(0)
                .size(buf_y.buffer_len)
                .src_queue_family_index(vk::QUEUE_FAMILY_IGNORED)
                .dst_queue_family_index(vk::QUEUE_FAMILY_IGNORED),
            vk::BufferMemoryBarrier::default()
                .src_access_mask(vk::AccessFlags::SHADER_WRITE)
                .dst_access_mask(vk::AccessFlags::HOST_READ)
                .buffer(buf_u.buffer)
                .offset(0)
                .size(buf_u.buffer_len)
                .src_queue_family_index(vk::QUEUE_FAMILY_IGNORED)
                .dst_queue_family_index(vk::QUEUE_FAMILY_IGNORED),
            vk::BufferMemoryBarrier::default()
                .src_access_mask(vk::AccessFlags::SHADER_WRITE)
                .dst_access_mask(vk::AccessFlags::HOST_READ)
                .buffer(buf_v.buffer)
                .offset(0)
                .size(buf_v.buffer_len)
                .src_queue_family_index(vk::QUEUE_FAMILY_IGNORED)
                .dst_queue_family_index(vk::QUEUE_FAMILY_IGNORED),
        ];
        vulk.dev.cmd_pipeline_barrier(
            cb,
            vk::PipelineStageFlags::COMPUTE_SHADER,
            vk::PipelineStageFlags::HOST,
            vk::DependencyFlags::empty(),
            &[],
            buf_memory_barriers,
            &[],
        );
        vulk.dev.cmd_pipeline_barrier(
            cb,
            vk::PipelineStageFlags::COMPUTE_SHADER,
            vk::PipelineStageFlags::BOTTOM_OF_PIPE,
            vk::DependencyFlags::empty(),
            &[],
            &[],
            exit_barriers,
        );
        img_inner.image_layout = vk::ImageLayout::GENERAL;

        vulk.dev
            .end_command_buffer(cb)
            .map_err(|_| "Failed to end command buffer")?;

        /* Wait for _everything_ to complete -- do not know if graphics/compute/decode is last */
        let cbs = &[cb];

        // todo: deduplicate with `start_copy_segments_from_dmabuf`, and structure to reduce allocations
        let mut waitv_values: Vec<u64> = wait_semaphores.iter().map(|x| x.1).collect();
        let mut waitv_semaphores: Vec<vk::Semaphore> =
            wait_semaphores.iter().map(|x| x.0.semaphore).collect();
        for bs in wait_binary_semaphores {
            waitv_values.push(u64::MAX);
            waitv_semaphores.push(bs.semaphore);
        }
        let mut waitv_stage_flags = Vec::new();
        waitv_stage_flags.resize(waitv_semaphores.len(), vk::PipelineStageFlags::ALL_COMMANDS);

        let mut wait_timeline_info = vk::TimelineSemaphoreSubmitInfo::default()
            .wait_semaphore_values(&waitv_values)
            .signal_semaphore_values(&[]);
        let submits = &[vk::SubmitInfo::default()
            .command_buffers(cbs)
            .wait_semaphores(&waitv_semaphores)
            .wait_dst_stage_mask(&waitv_stage_flags)
            .signal_semaphores(&[])
            .push_next(&mut wait_timeline_info)];

        let queue = vulkan_lock_queue(vulk);
        vulk.dev
            .queue_submit(queue.inner.queue, submits, vk::Fence::null())
            .map_err(|_| "Queue submit failed")?; // <- can fail with OOM

        vulk.dev
            .queue_wait_idle(queue.inner.queue)
            .map_err(|_| tag!("Queue wait idle failed"))?;

        drop(queue);

        buf_y.prepare_read()?;
        buf_u.prepare_read()?;
        buf_v.prepare_read()?;
        let view_y = buf_y.get_read_view();
        let view_u = buf_u.get_read_view();
        let view_v = buf_v.get_read_view();

        (*frame).width = ext_w as i32;
        (*frame).height = ext_h as i32;
        (*frame).format = AVPixelFormat_AV_PIX_FMT_YUV420P;
        (*frame).linesize[0] = stride_y.try_into().unwrap();
        (*frame).linesize[1] = stride_u.try_into().unwrap();
        (*frame).linesize[2] = stride_v.try_into().unwrap();
        assert!(view_y.data.as_ptr() as usize % 64 == 0);
        assert!(view_u.data.as_ptr() as usize % 64 == 0);
        assert!(view_v.data.as_ptr() as usize % 64 == 0);
        (*frame).data[0] = view_y.data.as_ptr() as *mut u8;
        (*frame).data[1] = view_u.data.as_ptr() as *mut u8;
        (*frame).data[2] = view_v.data.as_ptr() as *mut u8;

        // TODO: refcounting frame may avoid ffmpeg-side copies?
        avcodec_send_frame(&video.bindings, enc_inner.ctx, frame)?;

        let mut packet: *mut AVPacket = video.bindings.av_packet_alloc();

        avcodec_receive_packet(&video.bindings, enc_inner.ctx, packet)?;

        /* Keep frame data alive until processing is done? TODO: what does ffmpeg require? It might want to hold onto the frame a bit longer */
        drop(view_y);
        drop(view_u);
        drop(view_v);

        /* Cleanup; receiving packet should mean preceding operation is entirely done? */
        let inner_pool = pool.pool.lock().unwrap();
        vulk.dev.free_command_buffers(*inner_pool, &[cb]);
        vulk.dev
            .free_descriptor_sets(desc_pool, &[descriptor_set])
            .map_err(|_| "Failed to free descriptor set")
            .unwrap();
        vulk.dev.destroy_descriptor_pool(desc_pool, None);

        vulk.dev.destroy_buffer_view(buf_y_image_view, None);
        vulk.dev.destroy_buffer_view(buf_u_image_view, None);
        vulk.dev.destroy_buffer_view(buf_v_image_view, None);

        drop(buf_y);
        drop(buf_u);
        drop(buf_v);

        let mut f = frame;
        video.bindings.av_frame_free(&mut f);

        // TODO: allocate the packet data ourselves, using AVCodecContext.get_encode_buffer
        let data = std::slice::from_raw_parts((*packet).data, (*packet).size.try_into().unwrap());
        let mut packet_data = Vec::<u8>::new();
        packet_data.extend_from_slice(data);

        video.bindings.av_packet_free(&mut packet);
        Ok(packet_data)
    }
}

pub fn start_dmavid_encode(
    state: &Arc<VideoEncodeState>,
    pool: &Arc<VulkanCommandPool>,
    wait_semaphores: &[(Arc<VulkanTimelineSemaphore>, u64)],
    wait_binary_semaphores: &[VulkanBinarySemaphore],
) -> Result<Vec<u8>, String> {
    if matches!(state.data, VideoEncodeStateData::SW) {
        start_dmavid_encode_sw(state, pool, wait_semaphores, wait_binary_semaphores)
    } else {
        start_dmavid_encode_hw(state, pool, wait_semaphores, wait_binary_semaphores)
    }
}

/* Fill with specified RGB color */
#[cfg(test)]
fn fill_with_color(w: usize, h: usize, format: u32, color: (f32, f32, f32)) -> Vec<u8> {
    use crate::wayland_gen::*;

    fn scale_to_u(b: f32, bits: u8) -> u32 {
        assert!(bits < 32);
        let max = ((1u32 << bits) - 1) as f32;
        (b * max).clamp(0.0, max).round() as u32
    }

    fn pack565(b0: f32, b1: f32, b2: f32) -> [u8; 2] {
        let a0 = scale_to_u(b0, 5) as u16;
        let a1 = scale_to_u(b1, 6) as u16;
        let a2 = scale_to_u(b2, 5) as u16;
        u16::to_le_bytes(a0 | (a1 << 5) | (a2 << 11))
    }

    /* using: byte order of channels */
    fn pack8888(b0: f32, b1: f32, b2: f32, b3: f32) -> [u8; 4] {
        [
            scale_to_u(b0, 8) as u8,
            scale_to_u(b1, 8) as u8,
            scale_to_u(b2, 8) as u8,
            scale_to_u(b3, 8) as u8,
        ]
    }
    fn pack2101010(b0: f32, b1: f32, b2: f32, b3: f32) -> [u8; 4] {
        let a0 = scale_to_u(b0, 10);
        let a1 = scale_to_u(b1, 10);
        let a2 = scale_to_u(b2, 10);
        let a3 = scale_to_u(b3, 2);
        u32::to_le_bytes(a0 | (a1 << 10) | (a2 << 20) | (a3 << 30))
    }
    fn pack16161616(b0: f32, b1: f32, b2: f32, b3: f32) -> [u8; 8] {
        let x0 = (scale_to_u(b0, 16) as u16).to_le_bytes();
        let x1 = (scale_to_u(b1, 16) as u16).to_le_bytes();
        let x2 = (scale_to_u(b2, 16) as u16).to_le_bytes();
        let x3 = (scale_to_u(b3, 16) as u16).to_le_bytes();
        [x0[0], x0[1], x1[0], x1[1], x2[0], x2[1], x3[0], x3[1]]
    }

    fn replicate(pattern: &[u8], len: usize) -> Vec<u8> {
        pattern
            .iter()
            .cycle()
            .take(pattern.len() * len)
            .copied()
            .collect()
    }

    match drm_to_wayland(format).try_into().unwrap() {
        WlShmFormat::Rgb565 => replicate(&pack565(color.2, color.1, color.0), w * h),
        WlShmFormat::Bgr565 => replicate(&pack565(color.0, color.1, color.2), w * h),
        WlShmFormat::Xrgb8888 => replicate(&pack8888(color.2, color.1, color.0, 1.0), w * h),
        WlShmFormat::Xbgr8888 => replicate(&pack8888(color.0, color.1, color.2, 1.0), w * h),
        WlShmFormat::Xrgb2101010 => replicate(&pack2101010(color.2, color.1, color.0, 1.0), w * h),
        WlShmFormat::Xbgr2101010 => replicate(&pack2101010(color.0, color.1, color.2, 1.0), w * h),
        WlShmFormat::Xrgb16161616 => {
            replicate(&pack16161616(color.2, color.1, color.0, 1.0), w * h)
        }
        WlShmFormat::Xbgr16161616 => {
            replicate(&pack16161616(color.0, color.1, color.2, 1.0), w * h)
        }

        _ => todo!(),
    }
}

#[cfg(test)]
fn get_average_color(w: usize, h: usize, format: u32, data: &[u8]) -> (f32, f32, f32) {
    use crate::wayland_gen::*;

    /* from: byte order of channels -> rgb */
    fn swizzle_bgrx(x: (f32, f32, f32, f32)) -> (f32, f32, f32) {
        (x.2, x.1, x.0)
    }
    fn swizzle_rgbx(x: (f32, f32, f32, f32)) -> (f32, f32, f32) {
        (x.0, x.1, x.2)
    }
    fn u32_to_f(x: u32, bits: u8) -> f32 {
        assert!(bits < 32);
        let max = (1u32 << bits) - 1;
        assert!(x <= max);
        x as f32 / max as f32
    }
    fn u16_to_f(x: u16, bits: u8) -> f32 {
        u32_to_f(x as u32, bits)
    }
    fn u8_to_f(x: u8, bits: u8) -> f32 {
        u32_to_f(x as u32, bits)
    }
    fn unpack565(x: &[u8]) -> (f32, f32, f32, f32) {
        let y = u16::from_le_bytes(x.try_into().unwrap());
        (
            u16_to_f(y & 0x1f, 5),
            u16_to_f((y >> 5) & 0x3f, 6),
            u16_to_f((y >> 11) & 0x1f, 5),
            1.0,
        )
    }
    fn unpack8888(x: &[u8]) -> (f32, f32, f32, f32) {
        (
            u8_to_f(x[0], 8),
            u8_to_f(x[1], 8),
            u8_to_f(x[2], 8),
            u8_to_f(x[3], 8),
        )
    }
    fn unpack2101010(x: &[u8]) -> (f32, f32, f32, f32) {
        let y = u32::from_le_bytes(x.try_into().unwrap());
        (
            u32_to_f(y & 0x3ff, 10),
            u32_to_f((y >> 10) & 0x3ff, 10),
            u32_to_f((y >> 20) & 0x3ff, 10),
            u32_to_f((y >> 30) & 0x3, 2),
        )
    }
    fn unpack16161616(x: &[u8]) -> (f32, f32, f32, f32) {
        (
            u16_to_f(u16::from_le_bytes([x[0], x[1]]), 16),
            u16_to_f(u16::from_le_bytes([x[2], x[3]]), 16),
            u16_to_f(u16::from_le_bytes([x[4], x[5]]), 16),
            u16_to_f(u16::from_le_bytes([x[6], x[7]]), 16),
        )
    }

    fn add_color(x: (f32, f32, f32), y: (f32, f32, f32)) -> (f32, f32, f32) {
        (x.0 + y.0, x.1 + y.1, x.2 + y.2)
    }

    let base: (f32, f32, f32) = (0., 0., 0.);
    let rgb = match drm_to_wayland(format).try_into().unwrap() {
        WlShmFormat::Rgb565 => data
            .chunks_exact(2)
            .map(|x| swizzle_bgrx(unpack565(x)))
            .fold(base, add_color),
        WlShmFormat::Bgr565 => data
            .chunks_exact(2)
            .map(|x| swizzle_rgbx(unpack565(x)))
            .fold(base, add_color),
        WlShmFormat::Xrgb8888 => data
            .chunks_exact(4)
            .map(|x| swizzle_bgrx(unpack8888(x)))
            .fold(base, add_color),
        WlShmFormat::Xbgr8888 => data
            .chunks_exact(4)
            .map(|x| swizzle_rgbx(unpack8888(x)))
            .fold(base, add_color),
        WlShmFormat::Xrgb2101010 => data
            .chunks_exact(4)
            .map(|x| swizzle_bgrx(unpack2101010(x)))
            .fold(base, add_color),
        WlShmFormat::Xbgr2101010 => data
            .chunks_exact(4)
            .map(|x| swizzle_rgbx(unpack2101010(x)))
            .fold(base, add_color),
        WlShmFormat::Xrgb16161616 => data
            .chunks_exact(8)
            .map(|x| swizzle_bgrx(unpack16161616(x)))
            .fold(base, add_color),
        WlShmFormat::Xbgr16161616 => data
            .chunks_exact(8)
            .map(|x| swizzle_rgbx(unpack16161616(x)))
            .fold(base, add_color),

        _ => todo!(),
    };
    let scale = 1. / ((w * h) as f32);
    (rgb.0 * scale, rgb.1 * scale, rgb.2 * scale)
}

#[cfg(test)]
fn test_video(try_hardware: bool) {
    let _serialize_test = VULKAN_MUTEX.lock().unwrap();

    /* A crude error metric */
    fn color_error(x: (f32, f32, f32), y: (f32, f32, f32)) -> f32 {
        (x.0 - y.0).abs() + (x.1 - y.1).abs() + (x.2 - y.2).abs()
    }

    // debug disabled as libavcodec logging escapes test framework
    let debug = false;

    for dev_id in list_render_device_ids() {
        let pref = Some(if try_hardware {
            CodecPreference::HW
        } else {
            CodecPreference::SW
        });
        let vid_setting = VideoSetting {
            format: Some(VideoFormat::H264), /* the actual format given here does not matter */
            bits_per_frame: None,
            enc_pref: pref,
            dec_pref: pref,
        };
        let Ok(Some(instance)) = setup_vulkan_instance(debug, &vid_setting, false, false) else {
            continue;
        };
        let Ok(Some(vulk)) = setup_vulkan_device(&instance, Some(dev_id), &vid_setting, debug)
        else {
            continue;
        };

        println!("Setup complete for device id {}", dev_id);

        /* Test relatively small image sizes, since many formats will be tested */
        let sizes: [(usize, usize); 2] = [(63, 65), (1, 1)];

        for video_format in [VideoFormat::H264, VideoFormat::VP9, VideoFormat::AV1] {
            let mut format_modifiers = Vec::<(u32, u64, bool)>::new();
            'scan: for f in DRM_FORMATS {
                for s in sizes {
                    if !supports_video_format(&vulk, video_format, *f, s.0 as u32, s.1 as u32) {
                        continue 'scan;
                    }
                }
                /* no point in testing all modifiers for all video formats, since the intermediate
                 * copy step does not depend on the format. So only do this in one case. */
                if video_format == VideoFormat::H264 && !try_hardware {
                    let mut first = false;
                    for m in vulk.get_supported_modifiers(*f) {
                        format_modifiers.push((*f, *m, first));
                        first = false;
                    }
                } else if let [m, ..] = vulk.get_supported_modifiers(*f) {
                    format_modifiers.push((*f, *m, true));
                }
            }

            let pool = vulkan_get_cmd_pool(&vulk).unwrap();

            for (j, (format, modifier, first)) in format_modifiers.iter().enumerate() {
                let (format, modifier) = (*format, *modifier);
                let vkf = drm_to_vulkan(format).unwrap();
                println!(
                    "\nTesting {:?}, format {}/{} 0x{:x} => {:?}, modifier 0x{:x}",
                    video_format,
                    j + 1,
                    format_modifiers.len(),
                    format,
                    vkf,
                    modifier
                );

                let start_time = std::time::Instant::now();

                let mut color_errs = Vec::new();
                let mut elapsed_times = Vec::new();
                for (w, h) in sizes {
                    let mod_options = &[modifier];
                    let (dmabuf1, planes) =
                        vulkan_create_dmabuf(&vulk, w as u32, h as u32, format, mod_options, true)
                            .unwrap();
                    drop(planes);

                    let (dmabuf2, planes) =
                        vulkan_create_dmabuf(&vulk, w as u32, h as u32, format, mod_options, true)
                            .unwrap();
                    drop(planes);

                    let enc_state =
                        Arc::new(setup_video_encode(&dmabuf1, video_format, None).unwrap());
                    let dec_state = Arc::new(setup_video_decode(&dmabuf2, video_format).unwrap());

                    println!(
                        "Enc is sw: {}, Dec is sw: {}",
                        matches!(enc_state.data, VideoEncodeStateData::SW),
                        matches!(dec_state.data, VideoDecodeStateData::SW)
                    );

                    let copy1 = Arc::new(
                        vulkan_get_buffer(&vulk, dmabuf1.nominal_size(None), false).unwrap(),
                    );
                    let copy2 = Arc::new(
                        vulkan_get_buffer(&vulk, dmabuf2.nominal_size(None), true).unwrap(),
                    );

                    let colors_long = &[
                        (0.0, 0.0, 0.0),
                        (0.5, 0.5, 0.5),
                        (1.0, 0.5, 0.2),
                        (0.3, 0.0, 0.7),
                        (0.0, 1.0, 0.0),
                        (1.0, 1.0, 1.0),
                    ];
                    let colors_short = &[(1.0, 0.5, 0.2), (0.3, 0.0, 0.7)];
                    let colors: &[(f32, f32, f32)] =
                        if *first { colors_long } else { colors_short };
                    let mut net_err = 0.0;
                    for color in colors {
                        let data = fill_with_color(w, h, format, *color);
                        let check = get_average_color(w, h, format, &data);

                        copy_onto_dmabuf(&dmabuf1, &copy1, &data).unwrap();

                        let packet = start_dmavid_encode(&enc_state, &pool, &[], &[]).unwrap();

                        let vid_op = start_dmavid_apply(&dec_state, &pool, &packet).unwrap();
                        vid_op.wait_until_done().unwrap();

                        let mirror = copy_from_dmabuf(&dmabuf2, &copy2).unwrap();
                        let output = get_average_color(w, h, format, &mirror);

                        let check_err = color_error(*color, check);
                        let rtrip_err = color_error(*color, output);

                        /* Verify that the video encoding gets the color relatively close */
                        assert!(
                            check_err <= 0.1,
                            "encoding error: {:?}, ideal {:?} input {:?} output {:?}",
                            check_err,
                            color,
                            check,
                            output
                        );
                        if !try_hardware {
                            // As of writing, H264+radeon hardware video decoding fails on <=32x32 images
                            let thresh = if video_format == VideoFormat::AV1 {
                                0.2
                            } else {
                                0.1
                            };
                            assert!(
                                rtrip_err <= thresh,
                                "size: {:?} color: {:?} output: {:?}",
                                (w, h),
                                *color,
                                output
                            );
                        }
                        net_err += rtrip_err;
                    }

                    let end_time = std::time::Instant::now();
                    let duration = end_time.duration_since(start_time);
                    elapsed_times.push(duration.as_secs_f32());
                    color_errs.push(net_err / (colors.len() as f32));
                }
                println!(
                    "Tested sizes: {:?}; average errors: {:?}; times: {:?}",
                    sizes, color_errs, elapsed_times,
                );
            }
        }
    }
}

#[test]
fn test_video_try_hw() {
    test_video(true)
}

#[test]
fn test_video_sw() {
    test_video(false)
}
