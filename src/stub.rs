/* SPDX-License-Identifier: GPL-3.0-or-later */
/*! No-op implementations to build with specific features disabled */
#![allow(unused_variables)]
#[cfg(not(feature = "dmabuf"))]
mod dmabuf_stub {
    use crate::util::AddDmabufPlane;
    use log::debug;
    use std::os::fd::{BorrowedFd, OwnedFd};
    use std::path::PathBuf;
    use std::sync::Arc;

    use crate::VideoSetting;
    pub struct VulkanInstance(());
    pub struct VulkanDevice(());
    pub struct VulkanCommandPool {
        pub vulk: Arc<VulkanDevice>,
    }
    pub struct VulkanSyncFile(());
    pub struct VulkanBinarySemaphore(());
    pub struct VulkanTimelineSemaphore(());
    pub struct VulkanCopyHandle(());
    pub struct VulkanDmabuf {
        pub vulk: Arc<VulkanDevice>,
        pub width: u32,
        pub height: u32,
        pub main_fd: OwnedFd,
    }
    pub struct VulkanBuffer(());
    pub struct VulkanBufferReadView<'a> {
        pub data: &'a [u8],
    }
    pub struct VulkanBufferWriteView<'a> {
        pub data: &'a mut [u8],
    }

    pub fn setup_vulkan_instance(
        debug: bool,
        video: &VideoSetting,
        test_no_timeline_export: bool,
        test_no_binary_import: bool,
    ) -> Result<Option<Arc<VulkanInstance>>, String> {
        debug!("Waypipe was not compiled with Vulkan DMABUF backend");
        return Ok(None);
    }
    pub fn setup_vulkan_device_base(
        instance: &Arc<VulkanInstance>,
        main_device: Option<u64>,
        format_filter_for_video: bool,
    ) -> Result<Option<VulkanDevice>, String> {
        unreachable!();
    }
    pub fn setup_vulkan_device(
        instance: &Arc<VulkanInstance>,
        main_device: Option<u64>,
        video: &VideoSetting,
        debug: bool,
    ) -> Result<Option<Arc<VulkanDevice>>, String> {
        unreachable!();
    }

    pub fn start_copy_segments_from_dmabuf(
        img: &Arc<VulkanDmabuf>,
        copy: &Arc<VulkanBuffer>,
        pool: &Arc<VulkanCommandPool>,
        segments: &[(u32, u32, u32)],
        view_row_length: Option<u32>,
        wait_semaphores: &[(Arc<VulkanTimelineSemaphore>, u64)],
        wait_binary_semaphores: &[VulkanBinarySemaphore],
    ) -> Result<VulkanCopyHandle, String> {
        unreachable!();
    }
    pub fn start_copy_segments_onto_dmabuf(
        img: &Arc<VulkanDmabuf>,
        copy: &Arc<VulkanBuffer>,
        pool: &Arc<VulkanCommandPool>,
        segments: &[(u32, u32, u32)],
        view_row_length: Option<u32>,
        wait_semaphores: &[(Arc<VulkanTimelineSemaphore>, u64)],
    ) -> Result<VulkanCopyHandle, String> {
        unreachable!();
    }
    pub fn vulkan_get_cmd_pool(
        vulk: &Arc<VulkanDevice>,
    ) -> Result<Arc<VulkanCommandPool>, &'static str> {
        unreachable!();
    }
    pub fn vulkan_import_timeline(
        vulk: &Arc<VulkanDevice>,
        fd: OwnedFd,
    ) -> Result<Arc<VulkanTimelineSemaphore>, String> {
        unreachable!();
    }
    pub fn vulkan_create_timeline(
        vulk: &Arc<VulkanDevice>,
        start_pt: u64,
    ) -> Result<(Arc<VulkanTimelineSemaphore>, OwnedFd), String> {
        unreachable!();
    }
    pub fn get_dev_for_drm_node_path(path: &PathBuf) -> Result<u64, &'static str> {
        unreachable!();
    }
    pub fn vulkan_get_buffer(
        vulk: &Arc<VulkanDevice>,
        nom_len: usize,
        read_optimized: bool,
    ) -> Result<VulkanBuffer, &'static str> {
        unreachable!();
    }
    pub fn vulkan_create_dmabuf(
        vulk: &Arc<VulkanDevice>,
        width: u32,
        height: u32,
        drm_format: u32,
        modifier_options: &[u64],
        can_store_and_sample: bool,
    ) -> Result<(Arc<VulkanDmabuf>, Vec<AddDmabufPlane>), String> {
        unreachable!();
    }
    pub fn vulkan_import_dmabuf(
        vulk: &Arc<VulkanDevice>,
        planes: Vec<AddDmabufPlane>,
        width: u32,
        height: u32,
        drm_format: u32,
        can_store_and_sample: bool,
    ) -> Result<Arc<VulkanDmabuf>, String> {
        unreachable!();
    }
    impl VulkanBuffer {
        pub fn prepare_read(self: &VulkanBuffer) -> Result<(), &'static str> {
            unreachable!();
        }
        pub fn complete_write(self: &VulkanBuffer) -> Result<(), &'static str> {
            unreachable!();
        }
        pub fn get_read_view(self: &VulkanBuffer) -> VulkanBufferReadView<'_> {
            unreachable!();
        }
        pub fn get_write_view(self: &VulkanBuffer) -> VulkanBufferWriteView<'_> {
            unreachable!();
        }
    }
    impl VulkanCopyHandle {
        pub fn get_timeline_point(self: &VulkanCopyHandle) -> u64 {
            unreachable!();
        }
    }
    impl VulkanInstance {
        pub fn has_device(&self, main_device: Option<u64>) -> bool {
            unreachable!();
        }
        pub fn device_supports_timeline_import_export(&self, main_device: Option<u64>) -> bool {
            unreachable!();
        }
    }
    pub struct VulkanImageParameterMismatch(());
    impl std::fmt::Display for VulkanImageParameterMismatch {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            unreachable!();
        }
    }
    impl VulkanDevice {
        pub fn wait_for_timeline_pt(&self, pt: u64, max_wait: u64) -> Result<bool, String> {
            unreachable!();
        }
        pub fn get_device(&self) -> u64 {
            unreachable!();
        }
        pub fn get_event_fd(&self, timeline_point: u64) -> Result<Option<BorrowedFd<'_>>, String> {
            unreachable!();
        }
        pub fn get_current_timeline_pt(&self) -> Result<u64, String> {
            unreachable!();
        }
        pub fn supports_format(&self, drm_format: u32, drm_modifier: u64) -> bool {
            unreachable!();
        }
        pub fn get_supported_modifiers(&self, drm_format: u32) -> &[u64] {
            unreachable!();
        }
        pub fn can_import_image(
            &self,
            drm_format: u32,
            width: u32,
            height: u32,
            planes: &[AddDmabufPlane],
            can_store_and_sample: bool,
        ) -> Result<(), VulkanImageParameterMismatch> {
            unreachable!();
        }
        pub fn supports_binary_semaphore_import(&self) -> bool {
            unreachable!();
        }
        pub fn supports_timeline_import_export(&self) -> bool {
            unreachable!();
        }
    }

    impl VulkanDmabuf {
        pub fn nominal_size(self: &VulkanDmabuf, view_row_length: Option<u32>) -> usize {
            unreachable!();
        }
        pub fn get_bpp(&self) -> u32 {
            unreachable!();
        }
        pub fn export_sync_file(&self) -> Result<Option<VulkanSyncFile>, String> {
            unreachable!();
        }
    }
    impl VulkanSyncFile {
        pub fn export_binary_semaphore(&self) -> Result<VulkanBinarySemaphore, String> {
            unreachable!();
        }
    }
    impl VulkanTimelineSemaphore {
        pub fn get_current_pt(self: &VulkanTimelineSemaphore) -> Result<u64, String> {
            unreachable!();
        }

        pub fn get_event_fd(self: &VulkanTimelineSemaphore) -> BorrowedFd<'_> {
            unreachable!();
        }
        pub fn link_event_fd(
            self: &VulkanTimelineSemaphore,
            timeline_point: u64,
        ) -> Result<BorrowedFd<'_>, String> {
            unreachable!();
        }
        pub fn signal_timeline_pt(self: &VulkanTimelineSemaphore, pt: u64) -> Result<(), String> {
            unreachable!();
        }
    }
}
#[cfg(not(feature = "dmabuf"))]
pub use dmabuf_stub::*;

#[cfg(not(feature = "gbmfallback"))]
mod gbm_stub {
    use crate::util::AddDmabufPlane;
    use std::rc::Rc;

    pub struct GBMDevice {}
    pub struct GBMDmabuf {
        pub width: u32,
        pub height: u32,
    }
    pub fn setup_gbm_device(device: Option<u64>) -> Result<Option<Rc<GBMDevice>>, String> {
        Ok(None)
    }
    pub fn gbm_create_dmabuf(
        device: &Rc<GBMDevice>,
        width: u32,
        height: u32,
        format: u32,
        modifier_options: &[u64],
    ) -> Result<(GBMDmabuf, Vec<AddDmabufPlane>), String> {
        unreachable!();
    }
    pub fn gbm_import_dmabuf(
        device: &Rc<GBMDevice>,
        planes: Vec<AddDmabufPlane>,
        width: u32,
        height: u32,
        drm_format: u32,
    ) -> Result<GBMDmabuf, String> {
        unreachable!();
    }
    pub fn gbm_supported_modifiers(device: &Rc<GBMDevice>, format: u32) -> &[u64] {
        unreachable!();
    }
    pub fn gbm_get_device_id(device: &Rc<GBMDevice>) -> u64 {
        unreachable!();
    }

    impl GBMDmabuf {
        pub fn copy_from_dmabuf(
            &mut self,
            view_row_stride: Option<u32>,
            data: &mut [u8],
        ) -> Result<(), String> {
            unreachable!();
        }
        pub fn copy_onto_dmabuf(
            &mut self,
            view_row_stride: Option<u32>,
            data: &[u8],
        ) -> Result<(), String> {
            unreachable!();
        }
        pub fn nominal_size(&self, view_row_length: Option<u32>) -> usize {
            unreachable!();
        }
        pub fn get_bpp(&self) -> u32 {
            unreachable!();
        }
    }
}
#[cfg(not(feature = "gbmfallback"))]
pub use gbm_stub::*;

#[cfg(not(feature = "video"))]
mod video_stub {
    use std::sync::Arc;

    #[cfg(feature = "dmabuf")]
    use crate::dmabuf::*;
    #[cfg(not(feature = "dmabuf"))]
    use crate::stub::*;
    use crate::VideoFormat;

    pub struct VideoEncodeState(());
    pub struct VideoDecodeState(());
    pub struct VulkanDecodeOpHandle(());

    pub fn start_dmavid_apply(
        state: &Arc<VideoDecodeState>,
        pool: &Arc<VulkanCommandPool>,
        packet: &[u8],
    ) -> Result<VulkanDecodeOpHandle, String> {
        unreachable!();
    }
    pub fn start_dmavid_encode(
        state: &Arc<VideoEncodeState>,
        pool: &Arc<VulkanCommandPool>,
        wait_semaphores: &[(Arc<VulkanTimelineSemaphore>, u64)],
        wait_binary_semaphores: &[VulkanBinarySemaphore],
    ) -> Result<Vec<u8>, String> {
        unreachable!();
    }
    pub fn setup_video_decode(
        img: &Arc<VulkanDmabuf>,
        fmt: VideoFormat,
    ) -> Result<VideoDecodeState, &'static str> {
        unreachable!();
    }
    pub fn setup_video_encode(
        img: &Arc<VulkanDmabuf>,
        fmt: VideoFormat,
        bpf: Option<f32>,
    ) -> Result<VideoEncodeState, &'static str> {
        unreachable!();
    }
    pub fn supports_video_format(
        vulk: &VulkanDevice,
        fmt: VideoFormat,
        drm_format: u32,
        width: u32,
        height: u32,
    ) -> bool {
        false
    }
    impl VulkanDecodeOpHandle {
        pub fn get_timeline_point(&self) -> u64 {
            unreachable!();
        }
    }
}
#[cfg(not(feature = "video"))]
pub use video_stub::*;
