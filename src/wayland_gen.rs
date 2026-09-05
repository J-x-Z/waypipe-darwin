/*! Wayland protocol interface and method data and functions. Code automatically generated from protocols/ folder. */
#![allow(clippy::all, dead_code, missing_docs)]
use crate::wayland::WaylandArgument::*;
use crate::wayland::*;
use WaylandInterface::*;

pub fn write_req_wp_color_manager_v1_get_output(
    dst: &mut &mut [u8],
    for_id: ObjId,
    id: ObjId,
    output: ObjId,
) {
    let l = length_req_wp_color_manager_v1_get_output();
    write_header(dst, for_id, l, 1, 0);
    write_obj(dst, id);
    write_obj(dst, output);
}
pub fn length_req_wp_color_manager_v1_get_output() -> usize {
    16
}
pub fn parse_req_wp_color_manager_v1_get_output<'a>(
    mut msg: &'a [u8],
) -> Result<(ObjId, ObjId), &'static str> {
    msg = msg.get(8..).ok_or(PARSE_ERROR)?;
    let arg1 = parse_obj(&mut msg)?;
    let arg2 = parse_obj(&mut msg)?;
    if !msg.is_empty() {
        return Err(PARSE_ERROR);
    }
    Ok((arg1, arg2))
}
pub const OPCODE_WP_COLOR_MANAGER_V1_GET_OUTPUT: MethodId = MethodId::Request(1);
pub fn write_req_wp_color_manager_v1_create_icc_creator(
    dst: &mut &mut [u8],
    for_id: ObjId,
    obj: ObjId,
) {
    let l = length_req_wp_color_manager_v1_create_icc_creator();
    write_header(dst, for_id, l, 4, 0);
    write_obj(dst, obj);
}
pub fn length_req_wp_color_manager_v1_create_icc_creator() -> usize {
    12
}
pub fn parse_req_wp_color_manager_v1_create_icc_creator<'a>(
    mut msg: &'a [u8],
) -> Result<ObjId, &'static str> {
    msg = msg.get(8..).ok_or(PARSE_ERROR)?;
    let arg1 = parse_obj(&mut msg)?;
    if !msg.is_empty() {
        return Err(PARSE_ERROR);
    }
    Ok(arg1)
}
pub const OPCODE_WP_COLOR_MANAGER_V1_CREATE_ICC_CREATOR: MethodId = MethodId::Request(4);
const DATA_WP_COLOR_MANAGER_V1: WaylandData = WaylandData {
    name: "wp_color_manager_v1",
    evts: &[
        WaylandMethod {
            name: "supported_intent",
            sig: &[Uint],
            destructor: false,
        },
        WaylandMethod {
            name: "supported_feature",
            sig: &[Uint],
            destructor: false,
        },
        WaylandMethod {
            name: "supported_tf_named",
            sig: &[Uint],
            destructor: false,
        },
        WaylandMethod {
            name: "supported_primaries_named",
            sig: &[Uint],
            destructor: false,
        },
        WaylandMethod {
            name: "done",
            sig: &[],
            destructor: false,
        },
    ],
    reqs: &[
        WaylandMethod {
            name: "destroy",
            sig: &[],
            destructor: true,
        },
        WaylandMethod {
            name: "get_output",
            sig: &[NewId(WpColorManagementOutputV1), Object(WlOutput)],
            destructor: false,
        },
        WaylandMethod {
            name: "get_surface",
            sig: &[NewId(WpColorManagementSurfaceV1), Object(WlSurface)],
            destructor: false,
        },
        WaylandMethod {
            name: "get_surface_feedback",
            sig: &[NewId(WpColorManagementSurfaceFeedbackV1), Object(WlSurface)],
            destructor: false,
        },
        WaylandMethod {
            name: "create_icc_creator",
            sig: &[NewId(WpImageDescriptionCreatorIccV1)],
            destructor: false,
        },
        WaylandMethod {
            name: "create_parametric_creator",
            sig: &[NewId(WpImageDescriptionCreatorParamsV1)],
            destructor: false,
        },
        WaylandMethod {
            name: "create_windows_scrgb",
            sig: &[NewId(WpImageDescriptionV1)],
            destructor: false,
        },
        WaylandMethod {
            name: "get_image_description",
            sig: &[
                NewId(WpImageDescriptionV1),
                Object(WpImageDescriptionReferenceV1),
            ],
            destructor: false,
        },
    ],
    version: 2,
};
pub const WP_COLOR_MANAGER_V1: &[u8] = DATA_WP_COLOR_MANAGER_V1.name.as_bytes();
pub fn write_req_wp_color_management_output_v1_get_image_description(
    dst: &mut &mut [u8],
    for_id: ObjId,
    image_description: ObjId,
) {
    let l = length_req_wp_color_management_output_v1_get_image_description();
    write_header(dst, for_id, l, 1, 0);
    write_obj(dst, image_description);
}
pub fn length_req_wp_color_management_output_v1_get_image_description() -> usize {
    12
}
pub fn parse_req_wp_color_management_output_v1_get_image_description<'a>(
    mut msg: &'a [u8],
) -> Result<ObjId, &'static str> {
    msg = msg.get(8..).ok_or(PARSE_ERROR)?;
    let arg1 = parse_obj(&mut msg)?;
    if !msg.is_empty() {
        return Err(PARSE_ERROR);
    }
    Ok(arg1)
}
pub const OPCODE_WP_COLOR_MANAGEMENT_OUTPUT_V1_GET_IMAGE_DESCRIPTION: MethodId =
    MethodId::Request(1);
const DATA_WP_COLOR_MANAGEMENT_OUTPUT_V1: WaylandData = WaylandData {
    name: "wp_color_management_output_v1",
    evts: &[WaylandMethod {
        name: "image_description_changed",
        sig: &[],
        destructor: false,
    }],
    reqs: &[
        WaylandMethod {
            name: "destroy",
            sig: &[],
            destructor: true,
        },
        WaylandMethod {
            name: "get_image_description",
            sig: &[NewId(WpImageDescriptionV1)],
            destructor: false,
        },
    ],
    version: 2,
};
pub const WP_COLOR_MANAGEMENT_OUTPUT_V1: &[u8] = DATA_WP_COLOR_MANAGEMENT_OUTPUT_V1.name.as_bytes();
const DATA_WP_COLOR_MANAGEMENT_SURFACE_V1: WaylandData = WaylandData {
    name: "wp_color_management_surface_v1",
    evts: &[],
    reqs: &[
        WaylandMethod {
            name: "destroy",
            sig: &[],
            destructor: true,
        },
        WaylandMethod {
            name: "set_image_description",
            sig: &[Object(WpImageDescriptionV1), Uint],
            destructor: false,
        },
        WaylandMethod {
            name: "unset_image_description",
            sig: &[],
            destructor: false,
        },
    ],
    version: 2,
};
pub const WP_COLOR_MANAGEMENT_SURFACE_V1: &[u8] =
    DATA_WP_COLOR_MANAGEMENT_SURFACE_V1.name.as_bytes();
const DATA_WP_COLOR_MANAGEMENT_SURFACE_FEEDBACK_V1: WaylandData = WaylandData {
    name: "wp_color_management_surface_feedback_v1",
    evts: &[
        WaylandMethod {
            name: "preferred_changed",
            sig: &[Uint],
            destructor: false,
        },
        WaylandMethod {
            name: "preferred_changed2",
            sig: &[Uint, Uint],
            destructor: false,
        },
    ],
    reqs: &[
        WaylandMethod {
            name: "destroy",
            sig: &[],
            destructor: true,
        },
        WaylandMethod {
            name: "get_preferred",
            sig: &[NewId(WpImageDescriptionV1)],
            destructor: false,
        },
        WaylandMethod {
            name: "get_preferred_parametric",
            sig: &[NewId(WpImageDescriptionV1)],
            destructor: false,
        },
    ],
    version: 2,
};
pub const WP_COLOR_MANAGEMENT_SURFACE_FEEDBACK_V1: &[u8] =
    DATA_WP_COLOR_MANAGEMENT_SURFACE_FEEDBACK_V1.name.as_bytes();
pub fn write_req_wp_image_description_creator_icc_v1_set_icc_file(
    dst: &mut &mut [u8],
    for_id: ObjId,
    tag_fds: bool,
    offset: u32,
    length: u32,
) {
    let l = length_req_wp_image_description_creator_icc_v1_set_icc_file();
    write_header(dst, for_id, l, 1, if tag_fds { 1 } else { 0 });
    write_u32(dst, offset);
    write_u32(dst, length);
}
pub fn length_req_wp_image_description_creator_icc_v1_set_icc_file() -> usize {
    16
}
pub fn parse_req_wp_image_description_creator_icc_v1_set_icc_file<'a>(
    mut msg: &'a [u8],
) -> Result<(u32, u32), &'static str> {
    msg = msg.get(8..).ok_or(PARSE_ERROR)?;
    let arg2 = parse_u32(&mut msg)?;
    let arg3 = parse_u32(&mut msg)?;
    if !msg.is_empty() {
        return Err(PARSE_ERROR);
    }
    Ok((arg2, arg3))
}
pub const OPCODE_WP_IMAGE_DESCRIPTION_CREATOR_ICC_V1_SET_ICC_FILE: MethodId = MethodId::Request(1);
const DATA_WP_IMAGE_DESCRIPTION_CREATOR_ICC_V1: WaylandData = WaylandData {
    name: "wp_image_description_creator_icc_v1",
    evts: &[],
    reqs: &[
        WaylandMethod {
            name: "create",
            sig: &[NewId(WpImageDescriptionV1)],
            destructor: true,
        },
        WaylandMethod {
            name: "set_icc_file",
            sig: &[Fd, Uint, Uint],
            destructor: false,
        },
    ],
    version: 2,
};
pub const WP_IMAGE_DESCRIPTION_CREATOR_ICC_V1: &[u8] =
    DATA_WP_IMAGE_DESCRIPTION_CREATOR_ICC_V1.name.as_bytes();
const DATA_WP_IMAGE_DESCRIPTION_CREATOR_PARAMS_V1: WaylandData = WaylandData {
    name: "wp_image_description_creator_params_v1",
    evts: &[],
    reqs: &[
        WaylandMethod {
            name: "create",
            sig: &[NewId(WpImageDescriptionV1)],
            destructor: true,
        },
        WaylandMethod {
            name: "set_tf_named",
            sig: &[Uint],
            destructor: false,
        },
        WaylandMethod {
            name: "set_tf_power",
            sig: &[Uint],
            destructor: false,
        },
        WaylandMethod {
            name: "set_primaries_named",
            sig: &[Uint],
            destructor: false,
        },
        WaylandMethod {
            name: "set_primaries",
            sig: &[Int, Int, Int, Int, Int, Int, Int, Int],
            destructor: false,
        },
        WaylandMethod {
            name: "set_luminances",
            sig: &[Uint, Uint, Uint],
            destructor: false,
        },
        WaylandMethod {
            name: "set_mastering_display_primaries",
            sig: &[Int, Int, Int, Int, Int, Int, Int, Int],
            destructor: false,
        },
        WaylandMethod {
            name: "set_mastering_luminance",
            sig: &[Uint, Uint],
            destructor: false,
        },
        WaylandMethod {
            name: "set_max_cll",
            sig: &[Uint],
            destructor: false,
        },
        WaylandMethod {
            name: "set_max_fall",
            sig: &[Uint],
            destructor: false,
        },
    ],
    version: 2,
};
pub const WP_IMAGE_DESCRIPTION_CREATOR_PARAMS_V1: &[u8] =
    DATA_WP_IMAGE_DESCRIPTION_CREATOR_PARAMS_V1.name.as_bytes();
pub fn write_req_wp_image_description_v1_get_information(
    dst: &mut &mut [u8],
    for_id: ObjId,
    information: ObjId,
) {
    let l = length_req_wp_image_description_v1_get_information();
    write_header(dst, for_id, l, 1, 0);
    write_obj(dst, information);
}
pub fn length_req_wp_image_description_v1_get_information() -> usize {
    12
}
pub fn parse_req_wp_image_description_v1_get_information<'a>(
    mut msg: &'a [u8],
) -> Result<ObjId, &'static str> {
    msg = msg.get(8..).ok_or(PARSE_ERROR)?;
    let arg1 = parse_obj(&mut msg)?;
    if !msg.is_empty() {
        return Err(PARSE_ERROR);
    }
    Ok(arg1)
}
pub const OPCODE_WP_IMAGE_DESCRIPTION_V1_GET_INFORMATION: MethodId = MethodId::Request(1);
const DATA_WP_IMAGE_DESCRIPTION_V1: WaylandData = WaylandData {
    name: "wp_image_description_v1",
    evts: &[
        WaylandMethod {
            name: "failed",
            sig: &[Uint, String],
            destructor: false,
        },
        WaylandMethod {
            name: "ready",
            sig: &[Uint],
            destructor: false,
        },
        WaylandMethod {
            name: "ready2",
            sig: &[Uint, Uint],
            destructor: false,
        },
    ],
    reqs: &[
        WaylandMethod {
            name: "destroy",
            sig: &[],
            destructor: true,
        },
        WaylandMethod {
            name: "get_information",
            sig: &[NewId(WpImageDescriptionInfoV1)],
            destructor: false,
        },
    ],
    version: 2,
};
pub const WP_IMAGE_DESCRIPTION_V1: &[u8] = DATA_WP_IMAGE_DESCRIPTION_V1.name.as_bytes();
pub fn write_evt_wp_image_description_info_v1_icc_file(
    dst: &mut &mut [u8],
    for_id: ObjId,
    tag_fds: bool,
    icc_size: u32,
) {
    let l = length_evt_wp_image_description_info_v1_icc_file();
    write_header(dst, for_id, l, 1, if tag_fds { 1 } else { 0 });
    write_u32(dst, icc_size);
}
pub fn length_evt_wp_image_description_info_v1_icc_file() -> usize {
    12
}
pub fn parse_evt_wp_image_description_info_v1_icc_file<'a>(
    mut msg: &'a [u8],
) -> Result<u32, &'static str> {
    msg = msg.get(8..).ok_or(PARSE_ERROR)?;
    let arg2 = parse_u32(&mut msg)?;
    if !msg.is_empty() {
        return Err(PARSE_ERROR);
    }
    Ok(arg2)
}
pub const OPCODE_WP_IMAGE_DESCRIPTION_INFO_V1_ICC_FILE: MethodId = MethodId::Event(1);
const DATA_WP_IMAGE_DESCRIPTION_INFO_V1: WaylandData = WaylandData {
    name: "wp_image_description_info_v1",
    evts: &[
        WaylandMethod {
            name: "done",
            sig: &[],
            destructor: true,
        },
        WaylandMethod {
            name: "icc_file",
            sig: &[Fd, Uint],
            destructor: false,
        },
        WaylandMethod {
            name: "primaries",
            sig: &[Int, Int, Int, Int, Int, Int, Int, Int],
            destructor: false,
        },
        WaylandMethod {
            name: "primaries_named",
            sig: &[Uint],
            destructor: false,
        },
        WaylandMethod {
            name: "tf_power",
            sig: &[Uint],
            destructor: false,
        },
        WaylandMethod {
            name: "tf_named",
            sig: &[Uint],
            destructor: false,
        },
        WaylandMethod {
            name: "luminances",
            sig: &[Uint, Uint, Uint],
            destructor: false,
        },
        WaylandMethod {
            name: "target_primaries",
            sig: &[Int, Int, Int, Int, Int, Int, Int, Int],
            destructor: false,
        },
        WaylandMethod {
            name: "target_luminance",
            sig: &[Uint, Uint],
            destructor: false,
        },
        WaylandMethod {
            name: "target_max_cll",
            sig: &[Uint],
            destructor: false,
        },
        WaylandMethod {
            name: "target_max_fall",
            sig: &[Uint],
            destructor: false,
        },
    ],
    reqs: &[],
    version: 2,
};
pub const WP_IMAGE_DESCRIPTION_INFO_V1: &[u8] = DATA_WP_IMAGE_DESCRIPTION_INFO_V1.name.as_bytes();
const DATA_WP_IMAGE_DESCRIPTION_REFERENCE_V1: WaylandData = WaylandData {
    name: "wp_image_description_reference_v1",
    evts: &[],
    reqs: &[WaylandMethod {
        name: "destroy",
        sig: &[],
        destructor: true,
    }],
    version: 1,
};
pub const WP_IMAGE_DESCRIPTION_REFERENCE_V1: &[u8] =
    DATA_WP_IMAGE_DESCRIPTION_REFERENCE_V1.name.as_bytes();
pub fn write_req_wp_commit_timing_manager_v1_get_timer(
    dst: &mut &mut [u8],
    for_id: ObjId,
    id: ObjId,
    surface: ObjId,
) {
    let l = length_req_wp_commit_timing_manager_v1_get_timer();
    write_header(dst, for_id, l, 1, 0);
    write_obj(dst, id);
    write_obj(dst, surface);
}
pub fn length_req_wp_commit_timing_manager_v1_get_timer() -> usize {
    16
}
pub fn parse_req_wp_commit_timing_manager_v1_get_timer<'a>(
    mut msg: &'a [u8],
) -> Result<(ObjId, ObjId), &'static str> {
    msg = msg.get(8..).ok_or(PARSE_ERROR)?;
    let arg1 = parse_obj(&mut msg)?;
    let arg2 = parse_obj(&mut msg)?;
    if !msg.is_empty() {
        return Err(PARSE_ERROR);
    }
    Ok((arg1, arg2))
}
pub const OPCODE_WP_COMMIT_TIMING_MANAGER_V1_GET_TIMER: MethodId = MethodId::Request(1);
const DATA_WP_COMMIT_TIMING_MANAGER_V1: WaylandData = WaylandData {
    name: "wp_commit_timing_manager_v1",
    evts: &[],
    reqs: &[
        WaylandMethod {
            name: "destroy",
            sig: &[],
            destructor: true,
        },
        WaylandMethod {
            name: "get_timer",
            sig: &[NewId(WpCommitTimerV1), Object(WlSurface)],
            destructor: false,
        },
    ],
    version: 1,
};
pub const WP_COMMIT_TIMING_MANAGER_V1: &[u8] = DATA_WP_COMMIT_TIMING_MANAGER_V1.name.as_bytes();
pub fn write_req_wp_commit_timer_v1_set_timestamp(
    dst: &mut &mut [u8],
    for_id: ObjId,
    tv_sec_hi: u32,
    tv_sec_lo: u32,
    tv_nsec: u32,
) {
    let l = length_req_wp_commit_timer_v1_set_timestamp();
    write_header(dst, for_id, l, 0, 0);
    write_u32(dst, tv_sec_hi);
    write_u32(dst, tv_sec_lo);
    write_u32(dst, tv_nsec);
}
pub fn length_req_wp_commit_timer_v1_set_timestamp() -> usize {
    20
}
pub fn parse_req_wp_commit_timer_v1_set_timestamp<'a>(
    mut msg: &'a [u8],
) -> Result<(u32, u32, u32), &'static str> {
    msg = msg.get(8..).ok_or(PARSE_ERROR)?;
    let arg1 = parse_u32(&mut msg)?;
    let arg2 = parse_u32(&mut msg)?;
    let arg3 = parse_u32(&mut msg)?;
    if !msg.is_empty() {
        return Err(PARSE_ERROR);
    }
    Ok((arg1, arg2, arg3))
}
pub const OPCODE_WP_COMMIT_TIMER_V1_SET_TIMESTAMP: MethodId = MethodId::Request(0);
const DATA_WP_COMMIT_TIMER_V1: WaylandData = WaylandData {
    name: "wp_commit_timer_v1",
    evts: &[],
    reqs: &[
        WaylandMethod {
            name: "set_timestamp",
            sig: &[Uint, Uint, Uint],
            destructor: false,
        },
        WaylandMethod {
            name: "destroy",
            sig: &[],
            destructor: true,
        },
    ],
    version: 1,
};
pub const WP_COMMIT_TIMER_V1: &[u8] = DATA_WP_COMMIT_TIMER_V1.name.as_bytes();
pub fn write_req_ext_data_control_manager_v1_create_data_source(
    dst: &mut &mut [u8],
    for_id: ObjId,
    id: ObjId,
) {
    let l = length_req_ext_data_control_manager_v1_create_data_source();
    write_header(dst, for_id, l, 0, 0);
    write_obj(dst, id);
}
pub fn length_req_ext_data_control_manager_v1_create_data_source() -> usize {
    12
}
pub fn parse_req_ext_data_control_manager_v1_create_data_source<'a>(
    mut msg: &'a [u8],
) -> Result<ObjId, &'static str> {
    msg = msg.get(8..).ok_or(PARSE_ERROR)?;
    let arg1 = parse_obj(&mut msg)?;
    if !msg.is_empty() {
        return Err(PARSE_ERROR);
    }
    Ok(arg1)
}
pub const OPCODE_EXT_DATA_CONTROL_MANAGER_V1_CREATE_DATA_SOURCE: MethodId = MethodId::Request(0);
pub fn write_req_ext_data_control_manager_v1_get_data_device(
    dst: &mut &mut [u8],
    for_id: ObjId,
    id: ObjId,
    seat: ObjId,
) {
    let l = length_req_ext_data_control_manager_v1_get_data_device();
    write_header(dst, for_id, l, 1, 0);
    write_obj(dst, id);
    write_obj(dst, seat);
}
pub fn length_req_ext_data_control_manager_v1_get_data_device() -> usize {
    16
}
pub fn parse_req_ext_data_control_manager_v1_get_data_device<'a>(
    mut msg: &'a [u8],
) -> Result<(ObjId, ObjId), &'static str> {
    msg = msg.get(8..).ok_or(PARSE_ERROR)?;
    let arg1 = parse_obj(&mut msg)?;
    let arg2 = parse_obj(&mut msg)?;
    if !msg.is_empty() {
        return Err(PARSE_ERROR);
    }
    Ok((arg1, arg2))
}
pub const OPCODE_EXT_DATA_CONTROL_MANAGER_V1_GET_DATA_DEVICE: MethodId = MethodId::Request(1);
const DATA_EXT_DATA_CONTROL_MANAGER_V1: WaylandData = WaylandData {
    name: "ext_data_control_manager_v1",
    evts: &[],
    reqs: &[
        WaylandMethod {
            name: "create_data_source",
            sig: &[NewId(ExtDataControlSourceV1)],
            destructor: false,
        },
        WaylandMethod {
            name: "get_data_device",
            sig: &[NewId(ExtDataControlDeviceV1), Object(WlSeat)],
            destructor: false,
        },
        WaylandMethod {
            name: "destroy",
            sig: &[],
            destructor: true,
        },
    ],
    version: 1,
};
pub const EXT_DATA_CONTROL_MANAGER_V1: &[u8] = DATA_EXT_DATA_CONTROL_MANAGER_V1.name.as_bytes();
pub fn write_req_ext_data_control_device_v1_set_selection(
    dst: &mut &mut [u8],
    for_id: ObjId,
    source: ObjId,
) {
    let l = length_req_ext_data_control_device_v1_set_selection();
    write_header(dst, for_id, l, 0, 0);
    write_obj(dst, source);
}
pub fn length_req_ext_data_control_device_v1_set_selection() -> usize {
    12
}
pub fn parse_req_ext_data_control_device_v1_set_selection<'a>(
    mut msg: &'a [u8],
) -> Result<ObjId, &'static str> {
    msg = msg.get(8..).ok_or(PARSE_ERROR)?;
    let arg1 = parse_obj(&mut msg)?;
    if !msg.is_empty() {
        return Err(PARSE_ERROR);
    }
    Ok(arg1)
}
pub const OPCODE_EXT_DATA_CONTROL_DEVICE_V1_SET_SELECTION: MethodId = MethodId::Request(0);
pub fn write_evt_ext_data_control_device_v1_data_offer(
    dst: &mut &mut [u8],
    for_id: ObjId,
    id: ObjId,
) {
    let l = length_evt_ext_data_control_device_v1_data_offer();
    write_header(dst, for_id, l, 0, 0);
    write_obj(dst, id);
}
pub fn length_evt_ext_data_control_device_v1_data_offer() -> usize {
    12
}
pub fn parse_evt_ext_data_control_device_v1_data_offer<'a>(
    mut msg: &'a [u8],
) -> Result<ObjId, &'static str> {
    msg = msg.get(8..).ok_or(PARSE_ERROR)?;
    let arg1 = parse_obj(&mut msg)?;
    if !msg.is_empty() {
        return Err(PARSE_ERROR);
    }
    Ok(arg1)
}
pub const OPCODE_EXT_DATA_CONTROL_DEVICE_V1_DATA_OFFER: MethodId = MethodId::Event(0);
pub fn write_evt_ext_data_control_device_v1_selection(
    dst: &mut &mut [u8],
    for_id: ObjId,
    id: ObjId,
) {
    let l = length_evt_ext_data_control_device_v1_selection();
    write_header(dst, for_id, l, 1, 0);
    write_obj(dst, id);
}
pub fn length_evt_ext_data_control_device_v1_selection() -> usize {
    12
}
pub fn parse_evt_ext_data_control_device_v1_selection<'a>(
    mut msg: &'a [u8],
) -> Result<ObjId, &'static str> {
    msg = msg.get(8..).ok_or(PARSE_ERROR)?;
    let arg1 = parse_obj(&mut msg)?;
    if !msg.is_empty() {
        return Err(PARSE_ERROR);
    }
    Ok(arg1)
}
pub const OPCODE_EXT_DATA_CONTROL_DEVICE_V1_SELECTION: MethodId = MethodId::Event(1);
const DATA_EXT_DATA_CONTROL_DEVICE_V1: WaylandData = WaylandData {
    name: "ext_data_control_device_v1",
    evts: &[
        WaylandMethod {
            name: "data_offer",
            sig: &[NewId(ExtDataControlOfferV1)],
            destructor: false,
        },
        WaylandMethod {
            name: "selection",
            sig: &[Object(ExtDataControlOfferV1)],
            destructor: false,
        },
        WaylandMethod {
            name: "finished",
            sig: &[],
            destructor: false,
        },
        WaylandMethod {
            name: "primary_selection",
            sig: &[Object(ExtDataControlOfferV1)],
            destructor: false,
        },
    ],
    reqs: &[
        WaylandMethod {
            name: "set_selection",
            sig: &[Object(ExtDataControlSourceV1)],
            destructor: false,
        },
        WaylandMethod {
            name: "destroy",
            sig: &[],
            destructor: true,
        },
        WaylandMethod {
            name: "set_primary_selection",
            sig: &[Object(ExtDataControlSourceV1)],
            destructor: false,
        },
    ],
    version: 1,
};
pub const EXT_DATA_CONTROL_DEVICE_V1: &[u8] = DATA_EXT_DATA_CONTROL_DEVICE_V1.name.as_bytes();
pub fn write_req_ext_data_control_source_v1_offer(
    dst: &mut &mut [u8],
    for_id: ObjId,
    mime_type: &[u8],
) {
    let l = length_req_ext_data_control_source_v1_offer(mime_type.len());
    write_header(dst, for_id, l, 0, 0);
    write_string(dst, Some(mime_type));
}
pub fn length_req_ext_data_control_source_v1_offer(mime_type_len: usize) -> usize {
    let mut v = 8;
    v += length_string(mime_type_len);
    v
}
pub fn parse_req_ext_data_control_source_v1_offer<'a>(
    mut msg: &'a [u8],
) -> Result<&'a [u8], &'static str> {
    msg = msg.get(8..).ok_or(PARSE_ERROR)?;
    let arg1 = parse_string(&mut msg)?.ok_or(PARSE_ERROR)?;
    if !msg.is_empty() {
        return Err(PARSE_ERROR);
    }
    Ok(arg1)
}
pub const OPCODE_EXT_DATA_CONTROL_SOURCE_V1_OFFER: MethodId = MethodId::Request(0);
pub fn write_evt_ext_data_control_source_v1_send(
    dst: &mut &mut [u8],
    for_id: ObjId,
    tag_fds: bool,
    mime_type: &[u8],
) {
    let l = length_evt_ext_data_control_source_v1_send(mime_type.len());
    write_header(dst, for_id, l, 0, if tag_fds { 1 } else { 0 });
    write_string(dst, Some(mime_type));
}
pub fn length_evt_ext_data_control_source_v1_send(mime_type_len: usize) -> usize {
    let mut v = 8;
    v += length_string(mime_type_len);
    v
}
pub fn parse_evt_ext_data_control_source_v1_send<'a>(
    mut msg: &'a [u8],
) -> Result<&'a [u8], &'static str> {
    msg = msg.get(8..).ok_or(PARSE_ERROR)?;
    let arg1 = parse_string(&mut msg)?.ok_or(PARSE_ERROR)?;
    if !msg.is_empty() {
        return Err(PARSE_ERROR);
    }
    Ok(arg1)
}
pub const OPCODE_EXT_DATA_CONTROL_SOURCE_V1_SEND: MethodId = MethodId::Event(0);
const DATA_EXT_DATA_CONTROL_SOURCE_V1: WaylandData = WaylandData {
    name: "ext_data_control_source_v1",
    evts: &[
        WaylandMethod {
            name: "send",
            sig: &[String, Fd],
            destructor: false,
        },
        WaylandMethod {
            name: "cancelled",
            sig: &[],
            destructor: false,
        },
    ],
    reqs: &[
        WaylandMethod {
            name: "offer",
            sig: &[String],
            destructor: false,
        },
        WaylandMethod {
            name: "destroy",
            sig: &[],
            destructor: true,
        },
    ],
    version: 1,
};
pub const EXT_DATA_CONTROL_SOURCE_V1: &[u8] = DATA_EXT_DATA_CONTROL_SOURCE_V1.name.as_bytes();
pub fn write_req_ext_data_control_offer_v1_receive(
    dst: &mut &mut [u8],
    for_id: ObjId,
    tag_fds: bool,
    mime_type: &[u8],
) {
    let l = length_req_ext_data_control_offer_v1_receive(mime_type.len());
    write_header(dst, for_id, l, 0, if tag_fds { 1 } else { 0 });
    write_string(dst, Some(mime_type));
}
pub fn length_req_ext_data_control_offer_v1_receive(mime_type_len: usize) -> usize {
    let mut v = 8;
    v += length_string(mime_type_len);
    v
}
pub fn parse_req_ext_data_control_offer_v1_receive<'a>(
    mut msg: &'a [u8],
) -> Result<&'a [u8], &'static str> {
    msg = msg.get(8..).ok_or(PARSE_ERROR)?;
    let arg1 = parse_string(&mut msg)?.ok_or(PARSE_ERROR)?;
    if !msg.is_empty() {
        return Err(PARSE_ERROR);
    }
    Ok(arg1)
}
pub const OPCODE_EXT_DATA_CONTROL_OFFER_V1_RECEIVE: MethodId = MethodId::Request(0);
pub fn write_evt_ext_data_control_offer_v1_offer(
    dst: &mut &mut [u8],
    for_id: ObjId,
    mime_type: &[u8],
) {
    let l = length_evt_ext_data_control_offer_v1_offer(mime_type.len());
    write_header(dst, for_id, l, 0, 0);
    write_string(dst, Some(mime_type));
}
pub fn length_evt_ext_data_control_offer_v1_offer(mime_type_len: usize) -> usize {
    let mut v = 8;
    v += length_string(mime_type_len);
    v
}
pub fn parse_evt_ext_data_control_offer_v1_offer<'a>(
    mut msg: &'a [u8],
) -> Result<&'a [u8], &'static str> {
    msg = msg.get(8..).ok_or(PARSE_ERROR)?;
    let arg1 = parse_string(&mut msg)?.ok_or(PARSE_ERROR)?;
    if !msg.is_empty() {
        return Err(PARSE_ERROR);
    }
    Ok(arg1)
}
pub const OPCODE_EXT_DATA_CONTROL_OFFER_V1_OFFER: MethodId = MethodId::Event(0);
const DATA_EXT_DATA_CONTROL_OFFER_V1: WaylandData = WaylandData {
    name: "ext_data_control_offer_v1",
    evts: &[WaylandMethod {
        name: "offer",
        sig: &[String],
        destructor: false,
    }],
    reqs: &[
        WaylandMethod {
            name: "receive",
            sig: &[String, Fd],
            destructor: false,
        },
        WaylandMethod {
            name: "destroy",
            sig: &[],
            destructor: true,
        },
    ],
    version: 1,
};
pub const EXT_DATA_CONTROL_OFFER_V1: &[u8] = DATA_EXT_DATA_CONTROL_OFFER_V1.name.as_bytes();
const DATA_EXT_FOREIGN_TOPLEVEL_LIST_V1: WaylandData = WaylandData {
    name: "ext_foreign_toplevel_list_v1",
    evts: &[
        WaylandMethod {
            name: "toplevel",
            sig: &[NewId(ExtForeignToplevelHandleV1)],
            destructor: false,
        },
        WaylandMethod {
            name: "finished",
            sig: &[],
            destructor: false,
        },
    ],
    reqs: &[
        WaylandMethod {
            name: "stop",
            sig: &[],
            destructor: false,
        },
        WaylandMethod {
            name: "destroy",
            sig: &[],
            destructor: true,
        },
    ],
    version: 1,
};
pub const EXT_FOREIGN_TOPLEVEL_LIST_V1: &[u8] = DATA_EXT_FOREIGN_TOPLEVEL_LIST_V1.name.as_bytes();
const DATA_EXT_FOREIGN_TOPLEVEL_HANDLE_V1: WaylandData = WaylandData {
    name: "ext_foreign_toplevel_handle_v1",
    evts: &[
        WaylandMethod {
            name: "closed",
            sig: &[],
            destructor: false,
        },
        WaylandMethod {
            name: "done",
            sig: &[],
            destructor: false,
        },
        WaylandMethod {
            name: "title",
            sig: &[String],
            destructor: false,
        },
        WaylandMethod {
            name: "app_id",
            sig: &[String],
            destructor: false,
        },
        WaylandMethod {
            name: "identifier",
            sig: &[String],
            destructor: false,
        },
    ],
    reqs: &[WaylandMethod {
        name: "destroy",
        sig: &[],
        destructor: true,
    }],
    version: 1,
};
pub const EXT_FOREIGN_TOPLEVEL_HANDLE_V1: &[u8] =
    DATA_EXT_FOREIGN_TOPLEVEL_HANDLE_V1.name.as_bytes();
const DATA_EXT_IMAGE_CAPTURE_SOURCE_V1: WaylandData = WaylandData {
    name: "ext_image_capture_source_v1",
    evts: &[],
    reqs: &[WaylandMethod {
        name: "destroy",
        sig: &[],
        destructor: true,
    }],
    version: 1,
};
pub const EXT_IMAGE_CAPTURE_SOURCE_V1: &[u8] = DATA_EXT_IMAGE_CAPTURE_SOURCE_V1.name.as_bytes();
pub fn write_req_ext_output_image_capture_source_manager_v1_create_source(
    dst: &mut &mut [u8],
    for_id: ObjId,
    source: ObjId,
    output: ObjId,
) {
    let l = length_req_ext_output_image_capture_source_manager_v1_create_source();
    write_header(dst, for_id, l, 0, 0);
    write_obj(dst, source);
    write_obj(dst, output);
}
pub fn length_req_ext_output_image_capture_source_manager_v1_create_source() -> usize {
    16
}
pub fn parse_req_ext_output_image_capture_source_manager_v1_create_source<'a>(
    mut msg: &'a [u8],
) -> Result<(ObjId, ObjId), &'static str> {
    msg = msg.get(8..).ok_or(PARSE_ERROR)?;
    let arg1 = parse_obj(&mut msg)?;
    let arg2 = parse_obj(&mut msg)?;
    if !msg.is_empty() {
        return Err(PARSE_ERROR);
    }
    Ok((arg1, arg2))
}
pub const OPCODE_EXT_OUTPUT_IMAGE_CAPTURE_SOURCE_MANAGER_V1_CREATE_SOURCE: MethodId =
    MethodId::Request(0);
const DATA_EXT_OUTPUT_IMAGE_CAPTURE_SOURCE_MANAGER_V1: WaylandData = WaylandData {
    name: "ext_output_image_capture_source_manager_v1",
    evts: &[],
    reqs: &[
        WaylandMethod {
            name: "create_source",
            sig: &[NewId(ExtImageCaptureSourceV1), Object(WlOutput)],
            destructor: false,
        },
        WaylandMethod {
            name: "destroy",
            sig: &[],
            destructor: true,
        },
    ],
    version: 1,
};
pub const EXT_OUTPUT_IMAGE_CAPTURE_SOURCE_MANAGER_V1: &[u8] =
    DATA_EXT_OUTPUT_IMAGE_CAPTURE_SOURCE_MANAGER_V1
        .name
        .as_bytes();
const DATA_EXT_FOREIGN_TOPLEVEL_IMAGE_CAPTURE_SOURCE_MANAGER_V1: WaylandData = WaylandData {
    name: "ext_foreign_toplevel_image_capture_source_manager_v1",
    evts: &[],
    reqs: &[
        WaylandMethod {
            name: "create_source",
            sig: &[
                NewId(ExtImageCaptureSourceV1),
                Object(ExtForeignToplevelHandleV1),
            ],
            destructor: false,
        },
        WaylandMethod {
            name: "destroy",
            sig: &[],
            destructor: true,
        },
    ],
    version: 1,
};
pub const EXT_FOREIGN_TOPLEVEL_IMAGE_CAPTURE_SOURCE_MANAGER_V1: &[u8] =
    DATA_EXT_FOREIGN_TOPLEVEL_IMAGE_CAPTURE_SOURCE_MANAGER_V1
        .name
        .as_bytes();
pub fn write_req_ext_image_copy_capture_manager_v1_create_session(
    dst: &mut &mut [u8],
    for_id: ObjId,
    session: ObjId,
    source: ObjId,
    options: u32,
) {
    let l = length_req_ext_image_copy_capture_manager_v1_create_session();
    write_header(dst, for_id, l, 0, 0);
    write_obj(dst, session);
    write_obj(dst, source);
    write_u32(dst, options);
}
pub fn length_req_ext_image_copy_capture_manager_v1_create_session() -> usize {
    20
}
pub fn parse_req_ext_image_copy_capture_manager_v1_create_session<'a>(
    mut msg: &'a [u8],
) -> Result<(ObjId, ObjId, u32), &'static str> {
    msg = msg.get(8..).ok_or(PARSE_ERROR)?;
    let arg1 = parse_obj(&mut msg)?;
    let arg2 = parse_obj(&mut msg)?;
    let arg3 = parse_u32(&mut msg)?;
    if !msg.is_empty() {
        return Err(PARSE_ERROR);
    }
    Ok((arg1, arg2, arg3))
}
pub const OPCODE_EXT_IMAGE_COPY_CAPTURE_MANAGER_V1_CREATE_SESSION: MethodId = MethodId::Request(0);
const DATA_EXT_IMAGE_COPY_CAPTURE_MANAGER_V1: WaylandData = WaylandData {
    name: "ext_image_copy_capture_manager_v1",
    evts: &[],
    reqs: &[
        WaylandMethod {
            name: "create_session",
            sig: &[
                NewId(ExtImageCopyCaptureSessionV1),
                Object(ExtImageCaptureSourceV1),
                Uint,
            ],
            destructor: false,
        },
        WaylandMethod {
            name: "create_pointer_cursor_session",
            sig: &[
                NewId(ExtImageCopyCaptureCursorSessionV1),
                Object(ExtImageCaptureSourceV1),
                Object(WlPointer),
            ],
            destructor: false,
        },
        WaylandMethod {
            name: "destroy",
            sig: &[],
            destructor: true,
        },
    ],
    version: 1,
};
pub const EXT_IMAGE_COPY_CAPTURE_MANAGER_V1: &[u8] =
    DATA_EXT_IMAGE_COPY_CAPTURE_MANAGER_V1.name.as_bytes();
pub fn write_evt_ext_image_copy_capture_session_v1_buffer_size(
    dst: &mut &mut [u8],
    for_id: ObjId,
    width: u32,
    height: u32,
) {
    let l = length_evt_ext_image_copy_capture_session_v1_buffer_size();
    write_header(dst, for_id, l, 0, 0);
    write_u32(dst, width);
    write_u32(dst, height);
}
pub fn length_evt_ext_image_copy_capture_session_v1_buffer_size() -> usize {
    16
}
pub fn parse_evt_ext_image_copy_capture_session_v1_buffer_size<'a>(
    mut msg: &'a [u8],
) -> Result<(u32, u32), &'static str> {
    msg = msg.get(8..).ok_or(PARSE_ERROR)?;
    let arg1 = parse_u32(&mut msg)?;
    let arg2 = parse_u32(&mut msg)?;
    if !msg.is_empty() {
        return Err(PARSE_ERROR);
    }
    Ok((arg1, arg2))
}
pub const OPCODE_EXT_IMAGE_COPY_CAPTURE_SESSION_V1_BUFFER_SIZE: MethodId = MethodId::Event(0);
pub fn write_evt_ext_image_copy_capture_session_v1_shm_format(
    dst: &mut &mut [u8],
    for_id: ObjId,
    format: u32,
) {
    let l = length_evt_ext_image_copy_capture_session_v1_shm_format();
    write_header(dst, for_id, l, 1, 0);
    write_u32(dst, format);
}
pub fn length_evt_ext_image_copy_capture_session_v1_shm_format() -> usize {
    12
}
pub fn parse_evt_ext_image_copy_capture_session_v1_shm_format<'a>(
    mut msg: &'a [u8],
) -> Result<u32, &'static str> {
    msg = msg.get(8..).ok_or(PARSE_ERROR)?;
    let arg1 = parse_u32(&mut msg)?;
    if !msg.is_empty() {
        return Err(PARSE_ERROR);
    }
    Ok(arg1)
}
pub const OPCODE_EXT_IMAGE_COPY_CAPTURE_SESSION_V1_SHM_FORMAT: MethodId = MethodId::Event(1);
pub fn write_evt_ext_image_copy_capture_session_v1_dmabuf_device(
    dst: &mut &mut [u8],
    for_id: ObjId,
    device: &[u8],
) {
    let l = length_evt_ext_image_copy_capture_session_v1_dmabuf_device(device.len());
    write_header(dst, for_id, l, 2, 0);
    write_array(dst, device);
}
pub fn length_evt_ext_image_copy_capture_session_v1_dmabuf_device(device_len: usize) -> usize {
    let mut v = 8;
    v += length_array(device_len);
    v
}
pub fn parse_evt_ext_image_copy_capture_session_v1_dmabuf_device<'a>(
    mut msg: &'a [u8],
) -> Result<&'a [u8], &'static str> {
    msg = msg.get(8..).ok_or(PARSE_ERROR)?;
    let arg1 = parse_array(&mut msg)?;
    if !msg.is_empty() {
        return Err(PARSE_ERROR);
    }
    Ok(arg1)
}
pub const OPCODE_EXT_IMAGE_COPY_CAPTURE_SESSION_V1_DMABUF_DEVICE: MethodId = MethodId::Event(2);
pub fn write_evt_ext_image_copy_capture_session_v1_dmabuf_format(
    dst: &mut &mut [u8],
    for_id: ObjId,
    format: u32,
    modifiers: &[u8],
) {
    let l = length_evt_ext_image_copy_capture_session_v1_dmabuf_format(modifiers.len());
    write_header(dst, for_id, l, 3, 0);
    write_u32(dst, format);
    write_array(dst, modifiers);
}
pub fn length_evt_ext_image_copy_capture_session_v1_dmabuf_format(modifiers_len: usize) -> usize {
    let mut v = 12;
    v += length_array(modifiers_len);
    v
}
pub fn parse_evt_ext_image_copy_capture_session_v1_dmabuf_format<'a>(
    mut msg: &'a [u8],
) -> Result<(u32, &'a [u8]), &'static str> {
    msg = msg.get(8..).ok_or(PARSE_ERROR)?;
    let arg1 = parse_u32(&mut msg)?;
    let arg2 = parse_array(&mut msg)?;
    if !msg.is_empty() {
        return Err(PARSE_ERROR);
    }
    Ok((arg1, arg2))
}
pub const OPCODE_EXT_IMAGE_COPY_CAPTURE_SESSION_V1_DMABUF_FORMAT: MethodId = MethodId::Event(3);
pub fn write_evt_ext_image_copy_capture_session_v1_done(dst: &mut &mut [u8], for_id: ObjId) {
    let l = length_evt_ext_image_copy_capture_session_v1_done();
    write_header(dst, for_id, l, 4, 0);
}
pub fn length_evt_ext_image_copy_capture_session_v1_done() -> usize {
    8
}
pub fn parse_evt_ext_image_copy_capture_session_v1_done<'a>(
    msg: &'a [u8],
) -> Result<(), &'static str> {
    if msg.len() != 8 {
        return Err(PARSE_ERROR);
    }
    Ok(())
}
pub const OPCODE_EXT_IMAGE_COPY_CAPTURE_SESSION_V1_DONE: MethodId = MethodId::Event(4);
pub fn write_req_ext_image_copy_capture_session_v1_create_frame(
    dst: &mut &mut [u8],
    for_id: ObjId,
    frame: ObjId,
) {
    let l = length_req_ext_image_copy_capture_session_v1_create_frame();
    write_header(dst, for_id, l, 0, 0);
    write_obj(dst, frame);
}
pub fn length_req_ext_image_copy_capture_session_v1_create_frame() -> usize {
    12
}
pub fn parse_req_ext_image_copy_capture_session_v1_create_frame<'a>(
    mut msg: &'a [u8],
) -> Result<ObjId, &'static str> {
    msg = msg.get(8..).ok_or(PARSE_ERROR)?;
    let arg1 = parse_obj(&mut msg)?;
    if !msg.is_empty() {
        return Err(PARSE_ERROR);
    }
    Ok(arg1)
}
pub const OPCODE_EXT_IMAGE_COPY_CAPTURE_SESSION_V1_CREATE_FRAME: MethodId = MethodId::Request(0);
pub fn write_req_ext_image_copy_capture_session_v1_destroy(dst: &mut &mut [u8], for_id: ObjId) {
    let l = length_req_ext_image_copy_capture_session_v1_destroy();
    write_header(dst, for_id, l, 1, 0);
}
pub fn length_req_ext_image_copy_capture_session_v1_destroy() -> usize {
    8
}
pub fn parse_req_ext_image_copy_capture_session_v1_destroy<'a>(
    msg: &'a [u8],
) -> Result<(), &'static str> {
    if msg.len() != 8 {
        return Err(PARSE_ERROR);
    }
    Ok(())
}
pub const OPCODE_EXT_IMAGE_COPY_CAPTURE_SESSION_V1_DESTROY: MethodId = MethodId::Request(1);
const DATA_EXT_IMAGE_COPY_CAPTURE_SESSION_V1: WaylandData = WaylandData {
    name: "ext_image_copy_capture_session_v1",
    evts: &[
        WaylandMethod {
            name: "buffer_size",
            sig: &[Uint, Uint],
            destructor: false,
        },
        WaylandMethod {
            name: "shm_format",
            sig: &[Uint],
            destructor: false,
        },
        WaylandMethod {
            name: "dmabuf_device",
            sig: &[Array],
            destructor: false,
        },
        WaylandMethod {
            name: "dmabuf_format",
            sig: &[Uint, Array],
            destructor: false,
        },
        WaylandMethod {
            name: "done",
            sig: &[],
            destructor: false,
        },
        WaylandMethod {
            name: "stopped",
            sig: &[],
            destructor: false,
        },
    ],
    reqs: &[
        WaylandMethod {
            name: "create_frame",
            sig: &[NewId(ExtImageCopyCaptureFrameV1)],
            destructor: false,
        },
        WaylandMethod {
            name: "destroy",
            sig: &[],
            destructor: true,
        },
    ],
    version: 1,
};
pub const EXT_IMAGE_COPY_CAPTURE_SESSION_V1: &[u8] =
    DATA_EXT_IMAGE_COPY_CAPTURE_SESSION_V1.name.as_bytes();
pub fn write_req_ext_image_copy_capture_frame_v1_destroy(dst: &mut &mut [u8], for_id: ObjId) {
    let l = length_req_ext_image_copy_capture_frame_v1_destroy();
    write_header(dst, for_id, l, 0, 0);
}
pub fn length_req_ext_image_copy_capture_frame_v1_destroy() -> usize {
    8
}
pub fn parse_req_ext_image_copy_capture_frame_v1_destroy<'a>(
    msg: &'a [u8],
) -> Result<(), &'static str> {
    if msg.len() != 8 {
        return Err(PARSE_ERROR);
    }
    Ok(())
}
pub const OPCODE_EXT_IMAGE_COPY_CAPTURE_FRAME_V1_DESTROY: MethodId = MethodId::Request(0);
pub fn write_req_ext_image_copy_capture_frame_v1_attach_buffer(
    dst: &mut &mut [u8],
    for_id: ObjId,
    buffer: ObjId,
) {
    let l = length_req_ext_image_copy_capture_frame_v1_attach_buffer();
    write_header(dst, for_id, l, 1, 0);
    write_obj(dst, buffer);
}
pub fn length_req_ext_image_copy_capture_frame_v1_attach_buffer() -> usize {
    12
}
pub fn parse_req_ext_image_copy_capture_frame_v1_attach_buffer<'a>(
    mut msg: &'a [u8],
) -> Result<ObjId, &'static str> {
    msg = msg.get(8..).ok_or(PARSE_ERROR)?;
    let arg1 = parse_obj(&mut msg)?;
    if !msg.is_empty() {
        return Err(PARSE_ERROR);
    }
    Ok(arg1)
}
pub const OPCODE_EXT_IMAGE_COPY_CAPTURE_FRAME_V1_ATTACH_BUFFER: MethodId = MethodId::Request(1);
pub fn write_req_ext_image_copy_capture_frame_v1_damage_buffer(
    dst: &mut &mut [u8],
    for_id: ObjId,
    x: i32,
    y: i32,
    width: i32,
    height: i32,
) {
    let l = length_req_ext_image_copy_capture_frame_v1_damage_buffer();
    write_header(dst, for_id, l, 2, 0);
    write_i32(dst, x);
    write_i32(dst, y);
    write_i32(dst, width);
    write_i32(dst, height);
}
pub fn length_req_ext_image_copy_capture_frame_v1_damage_buffer() -> usize {
    24
}
pub fn parse_req_ext_image_copy_capture_frame_v1_damage_buffer<'a>(
    mut msg: &'a [u8],
) -> Result<(i32, i32, i32, i32), &'static str> {
    msg = msg.get(8..).ok_or(PARSE_ERROR)?;
    let arg1 = parse_i32(&mut msg)?;
    let arg2 = parse_i32(&mut msg)?;
    let arg3 = parse_i32(&mut msg)?;
    let arg4 = parse_i32(&mut msg)?;
    if !msg.is_empty() {
        return Err(PARSE_ERROR);
    }
    Ok((arg1, arg2, arg3, arg4))
}
pub const OPCODE_EXT_IMAGE_COPY_CAPTURE_FRAME_V1_DAMAGE_BUFFER: MethodId = MethodId::Request(2);
pub fn write_req_ext_image_copy_capture_frame_v1_capture(dst: &mut &mut [u8], for_id: ObjId) {
    let l = length_req_ext_image_copy_capture_frame_v1_capture();
    write_header(dst, for_id, l, 3, 0);
}
pub fn length_req_ext_image_copy_capture_frame_v1_capture() -> usize {
    8
}
pub fn parse_req_ext_image_copy_capture_frame_v1_capture<'a>(
    msg: &'a [u8],
) -> Result<(), &'static str> {
    if msg.len() != 8 {
        return Err(PARSE_ERROR);
    }
    Ok(())
}
pub const OPCODE_EXT_IMAGE_COPY_CAPTURE_FRAME_V1_CAPTURE: MethodId = MethodId::Request(3);
pub fn write_evt_ext_image_copy_capture_frame_v1_presentation_time(
    dst: &mut &mut [u8],
    for_id: ObjId,
    tv_sec_hi: u32,
    tv_sec_lo: u32,
    tv_nsec: u32,
) {
    let l = length_evt_ext_image_copy_capture_frame_v1_presentation_time();
    write_header(dst, for_id, l, 2, 0);
    write_u32(dst, tv_sec_hi);
    write_u32(dst, tv_sec_lo);
    write_u32(dst, tv_nsec);
}
pub fn length_evt_ext_image_copy_capture_frame_v1_presentation_time() -> usize {
    20
}
pub fn parse_evt_ext_image_copy_capture_frame_v1_presentation_time<'a>(
    mut msg: &'a [u8],
) -> Result<(u32, u32, u32), &'static str> {
    msg = msg.get(8..).ok_or(PARSE_ERROR)?;
    let arg1 = parse_u32(&mut msg)?;
    let arg2 = parse_u32(&mut msg)?;
    let arg3 = parse_u32(&mut msg)?;
    if !msg.is_empty() {
        return Err(PARSE_ERROR);
    }
    Ok((arg1, arg2, arg3))
}
pub const OPCODE_EXT_IMAGE_COPY_CAPTURE_FRAME_V1_PRESENTATION_TIME: MethodId = MethodId::Event(2);
pub fn write_evt_ext_image_copy_capture_frame_v1_ready(dst: &mut &mut [u8], for_id: ObjId) {
    let l = length_evt_ext_image_copy_capture_frame_v1_ready();
    write_header(dst, for_id, l, 3, 0);
}
pub fn length_evt_ext_image_copy_capture_frame_v1_ready() -> usize {
    8
}
pub fn parse_evt_ext_image_copy_capture_frame_v1_ready<'a>(
    msg: &'a [u8],
) -> Result<(), &'static str> {
    if msg.len() != 8 {
        return Err(PARSE_ERROR);
    }
    Ok(())
}
pub const OPCODE_EXT_IMAGE_COPY_CAPTURE_FRAME_V1_READY: MethodId = MethodId::Event(3);
pub fn write_evt_ext_image_copy_capture_frame_v1_failed(
    dst: &mut &mut [u8],
    for_id: ObjId,
    reason: u32,
) {
    let l = length_evt_ext_image_copy_capture_frame_v1_failed();
    write_header(dst, for_id, l, 4, 0);
    write_u32(dst, reason);
}
pub fn length_evt_ext_image_copy_capture_frame_v1_failed() -> usize {
    12
}
pub fn parse_evt_ext_image_copy_capture_frame_v1_failed<'a>(
    mut msg: &'a [u8],
) -> Result<u32, &'static str> {
    msg = msg.get(8..).ok_or(PARSE_ERROR)?;
    let arg1 = parse_u32(&mut msg)?;
    if !msg.is_empty() {
        return Err(PARSE_ERROR);
    }
    Ok(arg1)
}
pub const OPCODE_EXT_IMAGE_COPY_CAPTURE_FRAME_V1_FAILED: MethodId = MethodId::Event(4);
const DATA_EXT_IMAGE_COPY_CAPTURE_FRAME_V1: WaylandData = WaylandData {
    name: "ext_image_copy_capture_frame_v1",
    evts: &[
        WaylandMethod {
            name: "transform",
            sig: &[Uint],
            destructor: false,
        },
        WaylandMethod {
            name: "damage",
            sig: &[Int, Int, Int, Int],
            destructor: false,
        },
        WaylandMethod {
            name: "presentation_time",
            sig: &[Uint, Uint, Uint],
            destructor: false,
        },
        WaylandMethod {
            name: "ready",
            sig: &[],
            destructor: false,
        },
        WaylandMethod {
            name: "failed",
            sig: &[Uint],
            destructor: false,
        },
    ],
    reqs: &[
        WaylandMethod {
            name: "destroy",
            sig: &[],
            destructor: true,
        },
        WaylandMethod {
            name: "attach_buffer",
            sig: &[Object(WlBuffer)],
            destructor: false,
        },
        WaylandMethod {
            name: "damage_buffer",
            sig: &[Int, Int, Int, Int],
            destructor: false,
        },
        WaylandMethod {
            name: "capture",
            sig: &[],
            destructor: false,
        },
    ],
    version: 1,
};
pub const EXT_IMAGE_COPY_CAPTURE_FRAME_V1: &[u8] =
    DATA_EXT_IMAGE_COPY_CAPTURE_FRAME_V1.name.as_bytes();
pub fn write_req_ext_image_copy_capture_cursor_session_v1_get_capture_session(
    dst: &mut &mut [u8],
    for_id: ObjId,
    session: ObjId,
) {
    let l = length_req_ext_image_copy_capture_cursor_session_v1_get_capture_session();
    write_header(dst, for_id, l, 1, 0);
    write_obj(dst, session);
}
pub fn length_req_ext_image_copy_capture_cursor_session_v1_get_capture_session() -> usize {
    12
}
pub fn parse_req_ext_image_copy_capture_cursor_session_v1_get_capture_session<'a>(
    mut msg: &'a [u8],
) -> Result<ObjId, &'static str> {
    msg = msg.get(8..).ok_or(PARSE_ERROR)?;
    let arg1 = parse_obj(&mut msg)?;
    if !msg.is_empty() {
        return Err(PARSE_ERROR);
    }
    Ok(arg1)
}
pub const OPCODE_EXT_IMAGE_COPY_CAPTURE_CURSOR_SESSION_V1_GET_CAPTURE_SESSION: MethodId =
    MethodId::Request(1);
const DATA_EXT_IMAGE_COPY_CAPTURE_CURSOR_SESSION_V1: WaylandData = WaylandData {
    name: "ext_image_copy_capture_cursor_session_v1",
    evts: &[
        WaylandMethod {
            name: "enter",
            sig: &[],
            destructor: false,
        },
        WaylandMethod {
            name: "leave",
            sig: &[],
            destructor: false,
        },
        WaylandMethod {
            name: "position",
            sig: &[Int, Int],
            destructor: false,
        },
        WaylandMethod {
            name: "hotspot",
            sig: &[Int, Int],
            destructor: false,
        },
    ],
    reqs: &[
        WaylandMethod {
            name: "destroy",
            sig: &[],
            destructor: true,
        },
        WaylandMethod {
            name: "get_capture_session",
            sig: &[NewId(ExtImageCopyCaptureSessionV1)],
            destructor: false,
        },
    ],
    version: 1,
};
pub const EXT_IMAGE_COPY_CAPTURE_CURSOR_SESSION_V1: &[u8] =
    DATA_EXT_IMAGE_COPY_CAPTURE_CURSOR_SESSION_V1
        .name
        .as_bytes();
pub fn write_req_gtk_primary_selection_device_manager_create_source(
    dst: &mut &mut [u8],
    for_id: ObjId,
    id: ObjId,
) {
    let l = length_req_gtk_primary_selection_device_manager_create_source();
    write_header(dst, for_id, l, 0, 0);
    write_obj(dst, id);
}
pub fn length_req_gtk_primary_selection_device_manager_create_source() -> usize {
    12
}
pub fn parse_req_gtk_primary_selection_device_manager_create_source<'a>(
    mut msg: &'a [u8],
) -> Result<ObjId, &'static str> {
    msg = msg.get(8..).ok_or(PARSE_ERROR)?;
    let arg1 = parse_obj(&mut msg)?;
    if !msg.is_empty() {
        return Err(PARSE_ERROR);
    }
    Ok(arg1)
}
pub const OPCODE_GTK_PRIMARY_SELECTION_DEVICE_MANAGER_CREATE_SOURCE: MethodId =
    MethodId::Request(0);
pub fn write_req_gtk_primary_selection_device_manager_get_device(
    dst: &mut &mut [u8],
    for_id: ObjId,
    id: ObjId,
    seat: ObjId,
) {
    let l = length_req_gtk_primary_selection_device_manager_get_device();
    write_header(dst, for_id, l, 1, 0);
    write_obj(dst, id);
    write_obj(dst, seat);
}
pub fn length_req_gtk_primary_selection_device_manager_get_device() -> usize {
    16
}
pub fn parse_req_gtk_primary_selection_device_manager_get_device<'a>(
    mut msg: &'a [u8],
) -> Result<(ObjId, ObjId), &'static str> {
    msg = msg.get(8..).ok_or(PARSE_ERROR)?;
    let arg1 = parse_obj(&mut msg)?;
    let arg2 = parse_obj(&mut msg)?;
    if !msg.is_empty() {
        return Err(PARSE_ERROR);
    }
    Ok((arg1, arg2))
}
pub const OPCODE_GTK_PRIMARY_SELECTION_DEVICE_MANAGER_GET_DEVICE: MethodId = MethodId::Request(1);
const DATA_GTK_PRIMARY_SELECTION_DEVICE_MANAGER: WaylandData = WaylandData {
    name: "gtk_primary_selection_device_manager",
    evts: &[],
    reqs: &[
        WaylandMethod {
            name: "create_source",
            sig: &[NewId(GtkPrimarySelectionSource)],
            destructor: false,
        },
        WaylandMethod {
            name: "get_device",
            sig: &[NewId(GtkPrimarySelectionDevice), Object(WlSeat)],
            destructor: false,
        },
        WaylandMethod {
            name: "destroy",
            sig: &[],
            destructor: true,
        },
    ],
    version: 1,
};
pub const GTK_PRIMARY_SELECTION_DEVICE_MANAGER: &[u8] =
    DATA_GTK_PRIMARY_SELECTION_DEVICE_MANAGER.name.as_bytes();
pub fn write_req_gtk_primary_selection_device_set_selection(
    dst: &mut &mut [u8],
    for_id: ObjId,
    source: ObjId,
    serial: u32,
) {
    let l = length_req_gtk_primary_selection_device_set_selection();
    write_header(dst, for_id, l, 0, 0);
    write_obj(dst, source);
    write_u32(dst, serial);
}
pub fn length_req_gtk_primary_selection_device_set_selection() -> usize {
    16
}
pub fn parse_req_gtk_primary_selection_device_set_selection<'a>(
    mut msg: &'a [u8],
) -> Result<(ObjId, u32), &'static str> {
    msg = msg.get(8..).ok_or(PARSE_ERROR)?;
    let arg1 = parse_obj(&mut msg)?;
    let arg2 = parse_u32(&mut msg)?;
    if !msg.is_empty() {
        return Err(PARSE_ERROR);
    }
    Ok((arg1, arg2))
}
pub const OPCODE_GTK_PRIMARY_SELECTION_DEVICE_SET_SELECTION: MethodId = MethodId::Request(0);
pub fn write_evt_gtk_primary_selection_device_data_offer(
    dst: &mut &mut [u8],
    for_id: ObjId,
    offer: ObjId,
) {
    let l = length_evt_gtk_primary_selection_device_data_offer();
    write_header(dst, for_id, l, 0, 0);
    write_obj(dst, offer);
}
pub fn length_evt_gtk_primary_selection_device_data_offer() -> usize {
    12
}
pub fn parse_evt_gtk_primary_selection_device_data_offer<'a>(
    mut msg: &'a [u8],
) -> Result<ObjId, &'static str> {
    msg = msg.get(8..).ok_or(PARSE_ERROR)?;
    let arg1 = parse_obj(&mut msg)?;
    if !msg.is_empty() {
        return Err(PARSE_ERROR);
    }
    Ok(arg1)
}
pub const OPCODE_GTK_PRIMARY_SELECTION_DEVICE_DATA_OFFER: MethodId = MethodId::Event(0);
pub fn write_evt_gtk_primary_selection_device_selection(
    dst: &mut &mut [u8],
    for_id: ObjId,
    id: ObjId,
) {
    let l = length_evt_gtk_primary_selection_device_selection();
    write_header(dst, for_id, l, 1, 0);
    write_obj(dst, id);
}
pub fn length_evt_gtk_primary_selection_device_selection() -> usize {
    12
}
pub fn parse_evt_gtk_primary_selection_device_selection<'a>(
    mut msg: &'a [u8],
) -> Result<ObjId, &'static str> {
    msg = msg.get(8..).ok_or(PARSE_ERROR)?;
    let arg1 = parse_obj(&mut msg)?;
    if !msg.is_empty() {
        return Err(PARSE_ERROR);
    }
    Ok(arg1)
}
pub const OPCODE_GTK_PRIMARY_SELECTION_DEVICE_SELECTION: MethodId = MethodId::Event(1);
const DATA_GTK_PRIMARY_SELECTION_DEVICE: WaylandData = WaylandData {
    name: "gtk_primary_selection_device",
    evts: &[
        WaylandMethod {
            name: "data_offer",
            sig: &[NewId(GtkPrimarySelectionOffer)],
            destructor: false,
        },
        WaylandMethod {
            name: "selection",
            sig: &[Object(GtkPrimarySelectionOffer)],
            destructor: false,
        },
    ],
    reqs: &[
        WaylandMethod {
            name: "set_selection",
            sig: &[Object(GtkPrimarySelectionSource), Uint],
            destructor: false,
        },
        WaylandMethod {
            name: "destroy",
            sig: &[],
            destructor: true,
        },
    ],
    version: 1,
};
pub const GTK_PRIMARY_SELECTION_DEVICE: &[u8] = DATA_GTK_PRIMARY_SELECTION_DEVICE.name.as_bytes();
pub fn write_req_gtk_primary_selection_offer_receive(
    dst: &mut &mut [u8],
    for_id: ObjId,
    tag_fds: bool,
    mime_type: &[u8],
) {
    let l = length_req_gtk_primary_selection_offer_receive(mime_type.len());
    write_header(dst, for_id, l, 0, if tag_fds { 1 } else { 0 });
    write_string(dst, Some(mime_type));
}
pub fn length_req_gtk_primary_selection_offer_receive(mime_type_len: usize) -> usize {
    let mut v = 8;
    v += length_string(mime_type_len);
    v
}
pub fn parse_req_gtk_primary_selection_offer_receive<'a>(
    mut msg: &'a [u8],
) -> Result<&'a [u8], &'static str> {
    msg = msg.get(8..).ok_or(PARSE_ERROR)?;
    let arg1 = parse_string(&mut msg)?.ok_or(PARSE_ERROR)?;
    if !msg.is_empty() {
        return Err(PARSE_ERROR);
    }
    Ok(arg1)
}
pub const OPCODE_GTK_PRIMARY_SELECTION_OFFER_RECEIVE: MethodId = MethodId::Request(0);
pub fn write_evt_gtk_primary_selection_offer_offer(
    dst: &mut &mut [u8],
    for_id: ObjId,
    mime_type: &[u8],
) {
    let l = length_evt_gtk_primary_selection_offer_offer(mime_type.len());
    write_header(dst, for_id, l, 0, 0);
    write_string(dst, Some(mime_type));
}
pub fn length_evt_gtk_primary_selection_offer_offer(mime_type_len: usize) -> usize {
    let mut v = 8;
    v += length_string(mime_type_len);
    v
}
pub fn parse_evt_gtk_primary_selection_offer_offer<'a>(
    mut msg: &'a [u8],
) -> Result<&'a [u8], &'static str> {
    msg = msg.get(8..).ok_or(PARSE_ERROR)?;
    let arg1 = parse_string(&mut msg)?.ok_or(PARSE_ERROR)?;
    if !msg.is_empty() {
        return Err(PARSE_ERROR);
    }
    Ok(arg1)
}
pub const OPCODE_GTK_PRIMARY_SELECTION_OFFER_OFFER: MethodId = MethodId::Event(0);
const DATA_GTK_PRIMARY_SELECTION_OFFER: WaylandData = WaylandData {
    name: "gtk_primary_selection_offer",
    evts: &[WaylandMethod {
        name: "offer",
        sig: &[String],
        destructor: false,
    }],
    reqs: &[
        WaylandMethod {
            name: "receive",
            sig: &[String, Fd],
            destructor: false,
        },
        WaylandMethod {
            name: "destroy",
            sig: &[],
            destructor: true,
        },
    ],
    version: 1,
};
pub const GTK_PRIMARY_SELECTION_OFFER: &[u8] = DATA_GTK_PRIMARY_SELECTION_OFFER.name.as_bytes();
pub fn write_req_gtk_primary_selection_source_offer(
    dst: &mut &mut [u8],
    for_id: ObjId,
    mime_type: &[u8],
) {
    let l = length_req_gtk_primary_selection_source_offer(mime_type.len());
    write_header(dst, for_id, l, 0, 0);
    write_string(dst, Some(mime_type));
}
pub fn length_req_gtk_primary_selection_source_offer(mime_type_len: usize) -> usize {
    let mut v = 8;
    v += length_string(mime_type_len);
    v
}
pub fn parse_req_gtk_primary_selection_source_offer<'a>(
    mut msg: &'a [u8],
) -> Result<&'a [u8], &'static str> {
    msg = msg.get(8..).ok_or(PARSE_ERROR)?;
    let arg1 = parse_string(&mut msg)?.ok_or(PARSE_ERROR)?;
    if !msg.is_empty() {
        return Err(PARSE_ERROR);
    }
    Ok(arg1)
}
pub const OPCODE_GTK_PRIMARY_SELECTION_SOURCE_OFFER: MethodId = MethodId::Request(0);
pub fn write_evt_gtk_primary_selection_source_send(
    dst: &mut &mut [u8],
    for_id: ObjId,
    tag_fds: bool,
    mime_type: &[u8],
) {
    let l = length_evt_gtk_primary_selection_source_send(mime_type.len());
    write_header(dst, for_id, l, 0, if tag_fds { 1 } else { 0 });
    write_string(dst, Some(mime_type));
}
pub fn length_evt_gtk_primary_selection_source_send(mime_type_len: usize) -> usize {
    let mut v = 8;
    v += length_string(mime_type_len);
    v
}
pub fn parse_evt_gtk_primary_selection_source_send<'a>(
    mut msg: &'a [u8],
) -> Result<&'a [u8], &'static str> {
    msg = msg.get(8..).ok_or(PARSE_ERROR)?;
    let arg1 = parse_string(&mut msg)?.ok_or(PARSE_ERROR)?;
    if !msg.is_empty() {
        return Err(PARSE_ERROR);
    }
    Ok(arg1)
}
pub const OPCODE_GTK_PRIMARY_SELECTION_SOURCE_SEND: MethodId = MethodId::Event(0);
const DATA_GTK_PRIMARY_SELECTION_SOURCE: WaylandData = WaylandData {
    name: "gtk_primary_selection_source",
    evts: &[
        WaylandMethod {
            name: "send",
            sig: &[String, Fd],
            destructor: false,
        },
        WaylandMethod {
            name: "cancelled",
            sig: &[],
            destructor: false,
        },
    ],
    reqs: &[
        WaylandMethod {
            name: "offer",
            sig: &[String],
            destructor: false,
        },
        WaylandMethod {
            name: "destroy",
            sig: &[],
            destructor: true,
        },
    ],
    version: 1,
};
pub const GTK_PRIMARY_SELECTION_SOURCE: &[u8] = DATA_GTK_PRIMARY_SELECTION_SOURCE.name.as_bytes();
const DATA_ZWP_INPUT_METHOD_V2: WaylandData = WaylandData {
    name: "zwp_input_method_v2",
    evts: &[
        WaylandMethod {
            name: "activate",
            sig: &[],
            destructor: false,
        },
        WaylandMethod {
            name: "deactivate",
            sig: &[],
            destructor: false,
        },
        WaylandMethod {
            name: "surrounding_text",
            sig: &[String, Uint, Uint],
            destructor: false,
        },
        WaylandMethod {
            name: "text_change_cause",
            sig: &[Uint],
            destructor: false,
        },
        WaylandMethod {
            name: "content_type",
            sig: &[Uint, Uint],
            destructor: false,
        },
        WaylandMethod {
            name: "done",
            sig: &[],
            destructor: false,
        },
        WaylandMethod {
            name: "unavailable",
            sig: &[],
            destructor: false,
        },
    ],
    reqs: &[
        WaylandMethod {
            name: "commit_string",
            sig: &[String],
            destructor: false,
        },
        WaylandMethod {
            name: "set_preedit_string",
            sig: &[String, Int, Int],
            destructor: false,
        },
        WaylandMethod {
            name: "delete_surrounding_text",
            sig: &[Uint, Uint],
            destructor: false,
        },
        WaylandMethod {
            name: "commit",
            sig: &[Uint],
            destructor: false,
        },
        WaylandMethod {
            name: "get_input_popup_surface",
            sig: &[NewId(ZwpInputPopupSurfaceV2), Object(WlSurface)],
            destructor: false,
        },
        WaylandMethod {
            name: "grab_keyboard",
            sig: &[NewId(ZwpInputMethodKeyboardGrabV2)],
            destructor: false,
        },
        WaylandMethod {
            name: "destroy",
            sig: &[],
            destructor: true,
        },
    ],
    version: 1,
};
pub const ZWP_INPUT_METHOD_V2: &[u8] = DATA_ZWP_INPUT_METHOD_V2.name.as_bytes();
const DATA_ZWP_INPUT_POPUP_SURFACE_V2: WaylandData = WaylandData {
    name: "zwp_input_popup_surface_v2",
    evts: &[WaylandMethod {
        name: "text_input_rectangle",
        sig: &[Int, Int, Int, Int],
        destructor: false,
    }],
    reqs: &[WaylandMethod {
        name: "destroy",
        sig: &[],
        destructor: true,
    }],
    version: 1,
};
pub const ZWP_INPUT_POPUP_SURFACE_V2: &[u8] = DATA_ZWP_INPUT_POPUP_SURFACE_V2.name.as_bytes();
const DATA_ZWP_INPUT_METHOD_KEYBOARD_GRAB_V2: WaylandData = WaylandData {
    name: "zwp_input_method_keyboard_grab_v2",
    evts: &[
        WaylandMethod {
            name: "keymap",
            sig: &[Uint, Fd, Uint],
            destructor: false,
        },
        WaylandMethod {
            name: "key",
            sig: &[Uint, Uint, Uint, Uint],
            destructor: false,
        },
        WaylandMethod {
            name: "modifiers",
            sig: &[Uint, Uint, Uint, Uint, Uint],
            destructor: false,
        },
        WaylandMethod {
            name: "repeat_info",
            sig: &[Int, Int],
            destructor: false,
        },
    ],
    reqs: &[WaylandMethod {
        name: "release",
        sig: &[],
        destructor: true,
    }],
    version: 1,
};
pub const ZWP_INPUT_METHOD_KEYBOARD_GRAB_V2: &[u8] =
    DATA_ZWP_INPUT_METHOD_KEYBOARD_GRAB_V2.name.as_bytes();
const DATA_ZWP_INPUT_METHOD_MANAGER_V2: WaylandData = WaylandData {
    name: "zwp_input_method_manager_v2",
    evts: &[],
    reqs: &[
        WaylandMethod {
            name: "get_input_method",
            sig: &[Object(WlSeat), NewId(ZwpInputMethodV2)],
            destructor: false,
        },
        WaylandMethod {
            name: "destroy",
            sig: &[],
            destructor: true,
        },
    ],
    version: 1,
};
pub const ZWP_INPUT_METHOD_MANAGER_V2: &[u8] = DATA_ZWP_INPUT_METHOD_MANAGER_V2.name.as_bytes();
pub fn write_req_zwp_linux_dmabuf_v1_create_params(
    dst: &mut &mut [u8],
    for_id: ObjId,
    params_id: ObjId,
) {
    let l = length_req_zwp_linux_dmabuf_v1_create_params();
    write_header(dst, for_id, l, 1, 0);
    write_obj(dst, params_id);
}
pub fn length_req_zwp_linux_dmabuf_v1_create_params() -> usize {
    12
}
pub fn parse_req_zwp_linux_dmabuf_v1_create_params<'a>(
    mut msg: &'a [u8],
) -> Result<ObjId, &'static str> {
    msg = msg.get(8..).ok_or(PARSE_ERROR)?;
    let arg1 = parse_obj(&mut msg)?;
    if !msg.is_empty() {
        return Err(PARSE_ERROR);
    }
    Ok(arg1)
}
pub const OPCODE_ZWP_LINUX_DMABUF_V1_CREATE_PARAMS: MethodId = MethodId::Request(1);
pub fn write_evt_zwp_linux_dmabuf_v1_format(dst: &mut &mut [u8], for_id: ObjId, format: u32) {
    let l = length_evt_zwp_linux_dmabuf_v1_format();
    write_header(dst, for_id, l, 0, 0);
    write_u32(dst, format);
}
pub fn length_evt_zwp_linux_dmabuf_v1_format() -> usize {
    12
}
pub fn parse_evt_zwp_linux_dmabuf_v1_format<'a>(mut msg: &'a [u8]) -> Result<u32, &'static str> {
    msg = msg.get(8..).ok_or(PARSE_ERROR)?;
    let arg1 = parse_u32(&mut msg)?;
    if !msg.is_empty() {
        return Err(PARSE_ERROR);
    }
    Ok(arg1)
}
pub const OPCODE_ZWP_LINUX_DMABUF_V1_FORMAT: MethodId = MethodId::Event(0);
pub fn write_evt_zwp_linux_dmabuf_v1_modifier(
    dst: &mut &mut [u8],
    for_id: ObjId,
    format: u32,
    modifier_hi: u32,
    modifier_lo: u32,
) {
    let l = length_evt_zwp_linux_dmabuf_v1_modifier();
    write_header(dst, for_id, l, 1, 0);
    write_u32(dst, format);
    write_u32(dst, modifier_hi);
    write_u32(dst, modifier_lo);
}
pub fn length_evt_zwp_linux_dmabuf_v1_modifier() -> usize {
    20
}
pub fn parse_evt_zwp_linux_dmabuf_v1_modifier<'a>(
    mut msg: &'a [u8],
) -> Result<(u32, u32, u32), &'static str> {
    msg = msg.get(8..).ok_or(PARSE_ERROR)?;
    let arg1 = parse_u32(&mut msg)?;
    let arg2 = parse_u32(&mut msg)?;
    let arg3 = parse_u32(&mut msg)?;
    if !msg.is_empty() {
        return Err(PARSE_ERROR);
    }
    Ok((arg1, arg2, arg3))
}
pub const OPCODE_ZWP_LINUX_DMABUF_V1_MODIFIER: MethodId = MethodId::Event(1);
pub fn write_req_zwp_linux_dmabuf_v1_get_default_feedback(
    dst: &mut &mut [u8],
    for_id: ObjId,
    id: ObjId,
) {
    let l = length_req_zwp_linux_dmabuf_v1_get_default_feedback();
    write_header(dst, for_id, l, 2, 0);
    write_obj(dst, id);
}
pub fn length_req_zwp_linux_dmabuf_v1_get_default_feedback() -> usize {
    12
}
pub fn parse_req_zwp_linux_dmabuf_v1_get_default_feedback<'a>(
    mut msg: &'a [u8],
) -> Result<ObjId, &'static str> {
    msg = msg.get(8..).ok_or(PARSE_ERROR)?;
    let arg1 = parse_obj(&mut msg)?;
    if !msg.is_empty() {
        return Err(PARSE_ERROR);
    }
    Ok(arg1)
}
pub const OPCODE_ZWP_LINUX_DMABUF_V1_GET_DEFAULT_FEEDBACK: MethodId = MethodId::Request(2);
pub fn write_req_zwp_linux_dmabuf_v1_get_surface_feedback(
    dst: &mut &mut [u8],
    for_id: ObjId,
    id: ObjId,
    surface: ObjId,
) {
    let l = length_req_zwp_linux_dmabuf_v1_get_surface_feedback();
    write_header(dst, for_id, l, 3, 0);
    write_obj(dst, id);
    write_obj(dst, surface);
}
pub fn length_req_zwp_linux_dmabuf_v1_get_surface_feedback() -> usize {
    16
}
pub fn parse_req_zwp_linux_dmabuf_v1_get_surface_feedback<'a>(
    mut msg: &'a [u8],
) -> Result<(ObjId, ObjId), &'static str> {
    msg = msg.get(8..).ok_or(PARSE_ERROR)?;
    let arg1 = parse_obj(&mut msg)?;
    let arg2 = parse_obj(&mut msg)?;
    if !msg.is_empty() {
        return Err(PARSE_ERROR);
    }
    Ok((arg1, arg2))
}
pub const OPCODE_ZWP_LINUX_DMABUF_V1_GET_SURFACE_FEEDBACK: MethodId = MethodId::Request(3);
const DATA_ZWP_LINUX_DMABUF_V1: WaylandData = WaylandData {
    name: "zwp_linux_dmabuf_v1",
    evts: &[
        WaylandMethod {
            name: "format",
            sig: &[Uint],
            destructor: false,
        },
        WaylandMethod {
            name: "modifier",
            sig: &[Uint, Uint, Uint],
            destructor: false,
        },
    ],
    reqs: &[
        WaylandMethod {
            name: "destroy",
            sig: &[],
            destructor: true,
        },
        WaylandMethod {
            name: "create_params",
            sig: &[NewId(ZwpLinuxBufferParamsV1)],
            destructor: false,
        },
        WaylandMethod {
            name: "get_default_feedback",
            sig: &[NewId(ZwpLinuxDmabufFeedbackV1)],
            destructor: false,
        },
        WaylandMethod {
            name: "get_surface_feedback",
            sig: &[NewId(ZwpLinuxDmabufFeedbackV1), Object(WlSurface)],
            destructor: false,
        },
    ],
    version: 5,
};
pub const ZWP_LINUX_DMABUF_V1: &[u8] = DATA_ZWP_LINUX_DMABUF_V1.name.as_bytes();
pub fn write_req_zwp_linux_buffer_params_v1_destroy(dst: &mut &mut [u8], for_id: ObjId) {
    let l = length_req_zwp_linux_buffer_params_v1_destroy();
    write_header(dst, for_id, l, 0, 0);
}
pub fn length_req_zwp_linux_buffer_params_v1_destroy() -> usize {
    8
}
pub fn parse_req_zwp_linux_buffer_params_v1_destroy<'a>(msg: &'a [u8]) -> Result<(), &'static str> {
    if msg.len() != 8 {
        return Err(PARSE_ERROR);
    }
    Ok(())
}
pub const OPCODE_ZWP_LINUX_BUFFER_PARAMS_V1_DESTROY: MethodId = MethodId::Request(0);
pub fn write_req_zwp_linux_buffer_params_v1_add(
    dst: &mut &mut [u8],
    for_id: ObjId,
    tag_fds: bool,
    plane_idx: u32,
    offset: u32,
    stride: u32,
    modifier_hi: u32,
    modifier_lo: u32,
) {
    let l = length_req_zwp_linux_buffer_params_v1_add();
    write_header(dst, for_id, l, 1, if tag_fds { 1 } else { 0 });
    write_u32(dst, plane_idx);
    write_u32(dst, offset);
    write_u32(dst, stride);
    write_u32(dst, modifier_hi);
    write_u32(dst, modifier_lo);
}
pub fn length_req_zwp_linux_buffer_params_v1_add() -> usize {
    28
}
pub fn parse_req_zwp_linux_buffer_params_v1_add<'a>(
    mut msg: &'a [u8],
) -> Result<(u32, u32, u32, u32, u32), &'static str> {
    msg = msg.get(8..).ok_or(PARSE_ERROR)?;
    let arg2 = parse_u32(&mut msg)?;
    let arg3 = parse_u32(&mut msg)?;
    let arg4 = parse_u32(&mut msg)?;
    let arg5 = parse_u32(&mut msg)?;
    let arg6 = parse_u32(&mut msg)?;
    if !msg.is_empty() {
        return Err(PARSE_ERROR);
    }
    Ok((arg2, arg3, arg4, arg5, arg6))
}
pub const OPCODE_ZWP_LINUX_BUFFER_PARAMS_V1_ADD: MethodId = MethodId::Request(1);
pub fn write_req_zwp_linux_buffer_params_v1_create(
    dst: &mut &mut [u8],
    for_id: ObjId,
    width: i32,
    height: i32,
    format: u32,
    flags: u32,
) {
    let l = length_req_zwp_linux_buffer_params_v1_create();
    write_header(dst, for_id, l, 2, 0);
    write_i32(dst, width);
    write_i32(dst, height);
    write_u32(dst, format);
    write_u32(dst, flags);
}
pub fn length_req_zwp_linux_buffer_params_v1_create() -> usize {
    24
}
pub fn parse_req_zwp_linux_buffer_params_v1_create<'a>(
    mut msg: &'a [u8],
) -> Result<(i32, i32, u32, u32), &'static str> {
    msg = msg.get(8..).ok_or(PARSE_ERROR)?;
    let arg1 = parse_i32(&mut msg)?;
    let arg2 = parse_i32(&mut msg)?;
    let arg3 = parse_u32(&mut msg)?;
    let arg4 = parse_u32(&mut msg)?;
    if !msg.is_empty() {
        return Err(PARSE_ERROR);
    }
    Ok((arg1, arg2, arg3, arg4))
}
pub const OPCODE_ZWP_LINUX_BUFFER_PARAMS_V1_CREATE: MethodId = MethodId::Request(2);
pub fn write_evt_zwp_linux_buffer_params_v1_created(
    dst: &mut &mut [u8],
    for_id: ObjId,
    buffer: ObjId,
) {
    let l = length_evt_zwp_linux_buffer_params_v1_created();
    write_header(dst, for_id, l, 0, 0);
    write_obj(dst, buffer);
}
pub fn length_evt_zwp_linux_buffer_params_v1_created() -> usize {
    12
}
pub fn parse_evt_zwp_linux_buffer_params_v1_created<'a>(
    mut msg: &'a [u8],
) -> Result<ObjId, &'static str> {
    msg = msg.get(8..).ok_or(PARSE_ERROR)?;
    let arg1 = parse_obj(&mut msg)?;
    if !msg.is_empty() {
        return Err(PARSE_ERROR);
    }
    Ok(arg1)
}
pub const OPCODE_ZWP_LINUX_BUFFER_PARAMS_V1_CREATED: MethodId = MethodId::Event(0);
pub fn write_evt_zwp_linux_buffer_params_v1_failed(dst: &mut &mut [u8], for_id: ObjId) {
    let l = length_evt_zwp_linux_buffer_params_v1_failed();
    write_header(dst, for_id, l, 1, 0);
}
pub fn length_evt_zwp_linux_buffer_params_v1_failed() -> usize {
    8
}
pub fn parse_evt_zwp_linux_buffer_params_v1_failed<'a>(msg: &'a [u8]) -> Result<(), &'static str> {
    if msg.len() != 8 {
        return Err(PARSE_ERROR);
    }
    Ok(())
}
pub const OPCODE_ZWP_LINUX_BUFFER_PARAMS_V1_FAILED: MethodId = MethodId::Event(1);
pub fn write_req_zwp_linux_buffer_params_v1_create_immed(
    dst: &mut &mut [u8],
    for_id: ObjId,
    buffer_id: ObjId,
    width: i32,
    height: i32,
    format: u32,
    flags: u32,
) {
    let l = length_req_zwp_linux_buffer_params_v1_create_immed();
    write_header(dst, for_id, l, 3, 0);
    write_obj(dst, buffer_id);
    write_i32(dst, width);
    write_i32(dst, height);
    write_u32(dst, format);
    write_u32(dst, flags);
}
pub fn length_req_zwp_linux_buffer_params_v1_create_immed() -> usize {
    28
}
pub fn parse_req_zwp_linux_buffer_params_v1_create_immed<'a>(
    mut msg: &'a [u8],
) -> Result<(ObjId, i32, i32, u32, u32), &'static str> {
    msg = msg.get(8..).ok_or(PARSE_ERROR)?;
    let arg1 = parse_obj(&mut msg)?;
    let arg2 = parse_i32(&mut msg)?;
    let arg3 = parse_i32(&mut msg)?;
    let arg4 = parse_u32(&mut msg)?;
    let arg5 = parse_u32(&mut msg)?;
    if !msg.is_empty() {
        return Err(PARSE_ERROR);
    }
    Ok((arg1, arg2, arg3, arg4, arg5))
}
pub const OPCODE_ZWP_LINUX_BUFFER_PARAMS_V1_CREATE_IMMED: MethodId = MethodId::Request(3);
const DATA_ZWP_LINUX_BUFFER_PARAMS_V1: WaylandData = WaylandData {
    name: "zwp_linux_buffer_params_v1",
    evts: &[
        WaylandMethod {
            name: "created",
            sig: &[NewId(WlBuffer)],
            destructor: false,
        },
        WaylandMethod {
            name: "failed",
            sig: &[],
            destructor: false,
        },
    ],
    reqs: &[
        WaylandMethod {
            name: "destroy",
            sig: &[],
            destructor: true,
        },
        WaylandMethod {
            name: "add",
            sig: &[Fd, Uint, Uint, Uint, Uint, Uint],
            destructor: false,
        },
        WaylandMethod {
            name: "create",
            sig: &[Int, Int, Uint, Uint],
            destructor: false,
        },
        WaylandMethod {
            name: "create_immed",
            sig: &[NewId(WlBuffer), Int, Int, Uint, Uint],
            destructor: false,
        },
    ],
    version: 5,
};
pub const ZWP_LINUX_BUFFER_PARAMS_V1: &[u8] = DATA_ZWP_LINUX_BUFFER_PARAMS_V1.name.as_bytes();
pub fn write_evt_zwp_linux_dmabuf_feedback_v1_done(dst: &mut &mut [u8], for_id: ObjId) {
    let l = length_evt_zwp_linux_dmabuf_feedback_v1_done();
    write_header(dst, for_id, l, 0, 0);
}
pub fn length_evt_zwp_linux_dmabuf_feedback_v1_done() -> usize {
    8
}
pub fn parse_evt_zwp_linux_dmabuf_feedback_v1_done<'a>(msg: &'a [u8]) -> Result<(), &'static str> {
    if msg.len() != 8 {
        return Err(PARSE_ERROR);
    }
    Ok(())
}
pub const OPCODE_ZWP_LINUX_DMABUF_FEEDBACK_V1_DONE: MethodId = MethodId::Event(0);
pub fn write_evt_zwp_linux_dmabuf_feedback_v1_format_table(
    dst: &mut &mut [u8],
    for_id: ObjId,
    tag_fds: bool,
    size: u32,
) {
    let l = length_evt_zwp_linux_dmabuf_feedback_v1_format_table();
    write_header(dst, for_id, l, 1, if tag_fds { 1 } else { 0 });
    write_u32(dst, size);
}
pub fn length_evt_zwp_linux_dmabuf_feedback_v1_format_table() -> usize {
    12
}
pub fn parse_evt_zwp_linux_dmabuf_feedback_v1_format_table<'a>(
    mut msg: &'a [u8],
) -> Result<u32, &'static str> {
    msg = msg.get(8..).ok_or(PARSE_ERROR)?;
    let arg2 = parse_u32(&mut msg)?;
    if !msg.is_empty() {
        return Err(PARSE_ERROR);
    }
    Ok(arg2)
}
pub const OPCODE_ZWP_LINUX_DMABUF_FEEDBACK_V1_FORMAT_TABLE: MethodId = MethodId::Event(1);
pub fn write_evt_zwp_linux_dmabuf_feedback_v1_main_device(
    dst: &mut &mut [u8],
    for_id: ObjId,
    device: &[u8],
) {
    let l = length_evt_zwp_linux_dmabuf_feedback_v1_main_device(device.len());
    write_header(dst, for_id, l, 2, 0);
    write_array(dst, device);
}
pub fn length_evt_zwp_linux_dmabuf_feedback_v1_main_device(device_len: usize) -> usize {
    let mut v = 8;
    v += length_array(device_len);
    v
}
pub fn parse_evt_zwp_linux_dmabuf_feedback_v1_main_device<'a>(
    mut msg: &'a [u8],
) -> Result<&'a [u8], &'static str> {
    msg = msg.get(8..).ok_or(PARSE_ERROR)?;
    let arg1 = parse_array(&mut msg)?;
    if !msg.is_empty() {
        return Err(PARSE_ERROR);
    }
    Ok(arg1)
}
pub const OPCODE_ZWP_LINUX_DMABUF_FEEDBACK_V1_MAIN_DEVICE: MethodId = MethodId::Event(2);
pub fn write_evt_zwp_linux_dmabuf_feedback_v1_tranche_done(dst: &mut &mut [u8], for_id: ObjId) {
    let l = length_evt_zwp_linux_dmabuf_feedback_v1_tranche_done();
    write_header(dst, for_id, l, 3, 0);
}
pub fn length_evt_zwp_linux_dmabuf_feedback_v1_tranche_done() -> usize {
    8
}
pub fn parse_evt_zwp_linux_dmabuf_feedback_v1_tranche_done<'a>(
    msg: &'a [u8],
) -> Result<(), &'static str> {
    if msg.len() != 8 {
        return Err(PARSE_ERROR);
    }
    Ok(())
}
pub const OPCODE_ZWP_LINUX_DMABUF_FEEDBACK_V1_TRANCHE_DONE: MethodId = MethodId::Event(3);
pub fn write_evt_zwp_linux_dmabuf_feedback_v1_tranche_target_device(
    dst: &mut &mut [u8],
    for_id: ObjId,
    device: &[u8],
) {
    let l = length_evt_zwp_linux_dmabuf_feedback_v1_tranche_target_device(device.len());
    write_header(dst, for_id, l, 4, 0);
    write_array(dst, device);
}
pub fn length_evt_zwp_linux_dmabuf_feedback_v1_tranche_target_device(device_len: usize) -> usize {
    let mut v = 8;
    v += length_array(device_len);
    v
}
pub fn parse_evt_zwp_linux_dmabuf_feedback_v1_tranche_target_device<'a>(
    mut msg: &'a [u8],
) -> Result<&'a [u8], &'static str> {
    msg = msg.get(8..).ok_or(PARSE_ERROR)?;
    let arg1 = parse_array(&mut msg)?;
    if !msg.is_empty() {
        return Err(PARSE_ERROR);
    }
    Ok(arg1)
}
pub const OPCODE_ZWP_LINUX_DMABUF_FEEDBACK_V1_TRANCHE_TARGET_DEVICE: MethodId = MethodId::Event(4);
pub fn write_evt_zwp_linux_dmabuf_feedback_v1_tranche_formats(
    dst: &mut &mut [u8],
    for_id: ObjId,
    indices: &[u8],
) {
    let l = length_evt_zwp_linux_dmabuf_feedback_v1_tranche_formats(indices.len());
    write_header(dst, for_id, l, 5, 0);
    write_array(dst, indices);
}
pub fn length_evt_zwp_linux_dmabuf_feedback_v1_tranche_formats(indices_len: usize) -> usize {
    let mut v = 8;
    v += length_array(indices_len);
    v
}
pub fn parse_evt_zwp_linux_dmabuf_feedback_v1_tranche_formats<'a>(
    mut msg: &'a [u8],
) -> Result<&'a [u8], &'static str> {
    msg = msg.get(8..).ok_or(PARSE_ERROR)?;
    let arg1 = parse_array(&mut msg)?;
    if !msg.is_empty() {
        return Err(PARSE_ERROR);
    }
    Ok(arg1)
}
pub const OPCODE_ZWP_LINUX_DMABUF_FEEDBACK_V1_TRANCHE_FORMATS: MethodId = MethodId::Event(5);
pub fn write_evt_zwp_linux_dmabuf_feedback_v1_tranche_flags(
    dst: &mut &mut [u8],
    for_id: ObjId,
    flags: u32,
) {
    let l = length_evt_zwp_linux_dmabuf_feedback_v1_tranche_flags();
    write_header(dst, for_id, l, 6, 0);
    write_u32(dst, flags);
}
pub fn length_evt_zwp_linux_dmabuf_feedback_v1_tranche_flags() -> usize {
    12
}
pub fn parse_evt_zwp_linux_dmabuf_feedback_v1_tranche_flags<'a>(
    mut msg: &'a [u8],
) -> Result<u32, &'static str> {
    msg = msg.get(8..).ok_or(PARSE_ERROR)?;
    let arg1 = parse_u32(&mut msg)?;
    if !msg.is_empty() {
        return Err(PARSE_ERROR);
    }
    Ok(arg1)
}
pub const OPCODE_ZWP_LINUX_DMABUF_FEEDBACK_V1_TRANCHE_FLAGS: MethodId = MethodId::Event(6);
const DATA_ZWP_LINUX_DMABUF_FEEDBACK_V1: WaylandData = WaylandData {
    name: "zwp_linux_dmabuf_feedback_v1",
    evts: &[
        WaylandMethod {
            name: "done",
            sig: &[],
            destructor: false,
        },
        WaylandMethod {
            name: "format_table",
            sig: &[Fd, Uint],
            destructor: false,
        },
        WaylandMethod {
            name: "main_device",
            sig: &[Array],
            destructor: false,
        },
        WaylandMethod {
            name: "tranche_done",
            sig: &[],
            destructor: false,
        },
        WaylandMethod {
            name: "tranche_target_device",
            sig: &[Array],
            destructor: false,
        },
        WaylandMethod {
            name: "tranche_formats",
            sig: &[Array],
            destructor: false,
        },
        WaylandMethod {
            name: "tranche_flags",
            sig: &[Uint],
            destructor: false,
        },
    ],
    reqs: &[WaylandMethod {
        name: "destroy",
        sig: &[],
        destructor: true,
    }],
    version: 5,
};
pub const ZWP_LINUX_DMABUF_FEEDBACK_V1: &[u8] = DATA_ZWP_LINUX_DMABUF_FEEDBACK_V1.name.as_bytes();
pub fn write_req_wp_linux_drm_syncobj_manager_v1_get_surface(
    dst: &mut &mut [u8],
    for_id: ObjId,
    id: ObjId,
    surface: ObjId,
) {
    let l = length_req_wp_linux_drm_syncobj_manager_v1_get_surface();
    write_header(dst, for_id, l, 1, 0);
    write_obj(dst, id);
    write_obj(dst, surface);
}
pub fn length_req_wp_linux_drm_syncobj_manager_v1_get_surface() -> usize {
    16
}
pub fn parse_req_wp_linux_drm_syncobj_manager_v1_get_surface<'a>(
    mut msg: &'a [u8],
) -> Result<(ObjId, ObjId), &'static str> {
    msg = msg.get(8..).ok_or(PARSE_ERROR)?;
    let arg1 = parse_obj(&mut msg)?;
    let arg2 = parse_obj(&mut msg)?;
    if !msg.is_empty() {
        return Err(PARSE_ERROR);
    }
    Ok((arg1, arg2))
}
pub const OPCODE_WP_LINUX_DRM_SYNCOBJ_MANAGER_V1_GET_SURFACE: MethodId = MethodId::Request(1);
pub fn write_req_wp_linux_drm_syncobj_manager_v1_import_timeline(
    dst: &mut &mut [u8],
    for_id: ObjId,
    tag_fds: bool,
    id: ObjId,
) {
    let l = length_req_wp_linux_drm_syncobj_manager_v1_import_timeline();
    write_header(dst, for_id, l, 2, if tag_fds { 1 } else { 0 });
    write_obj(dst, id);
}
pub fn length_req_wp_linux_drm_syncobj_manager_v1_import_timeline() -> usize {
    12
}
pub fn parse_req_wp_linux_drm_syncobj_manager_v1_import_timeline<'a>(
    mut msg: &'a [u8],
) -> Result<ObjId, &'static str> {
    msg = msg.get(8..).ok_or(PARSE_ERROR)?;
    let arg1 = parse_obj(&mut msg)?;
    if !msg.is_empty() {
        return Err(PARSE_ERROR);
    }
    Ok(arg1)
}
pub const OPCODE_WP_LINUX_DRM_SYNCOBJ_MANAGER_V1_IMPORT_TIMELINE: MethodId = MethodId::Request(2);
const DATA_WP_LINUX_DRM_SYNCOBJ_MANAGER_V1: WaylandData = WaylandData {
    name: "wp_linux_drm_syncobj_manager_v1",
    evts: &[],
    reqs: &[
        WaylandMethod {
            name: "destroy",
            sig: &[],
            destructor: true,
        },
        WaylandMethod {
            name: "get_surface",
            sig: &[NewId(WpLinuxDrmSyncobjSurfaceV1), Object(WlSurface)],
            destructor: false,
        },
        WaylandMethod {
            name: "import_timeline",
            sig: &[NewId(WpLinuxDrmSyncobjTimelineV1), Fd],
            destructor: false,
        },
    ],
    version: 1,
};
pub const WP_LINUX_DRM_SYNCOBJ_MANAGER_V1: &[u8] =
    DATA_WP_LINUX_DRM_SYNCOBJ_MANAGER_V1.name.as_bytes();
const DATA_WP_LINUX_DRM_SYNCOBJ_TIMELINE_V1: WaylandData = WaylandData {
    name: "wp_linux_drm_syncobj_timeline_v1",
    evts: &[],
    reqs: &[WaylandMethod {
        name: "destroy",
        sig: &[],
        destructor: true,
    }],
    version: 1,
};
pub const WP_LINUX_DRM_SYNCOBJ_TIMELINE_V1: &[u8] =
    DATA_WP_LINUX_DRM_SYNCOBJ_TIMELINE_V1.name.as_bytes();
pub fn write_req_wp_linux_drm_syncobj_surface_v1_set_acquire_point(
    dst: &mut &mut [u8],
    for_id: ObjId,
    timeline: ObjId,
    point_hi: u32,
    point_lo: u32,
) {
    let l = length_req_wp_linux_drm_syncobj_surface_v1_set_acquire_point();
    write_header(dst, for_id, l, 1, 0);
    write_obj(dst, timeline);
    write_u32(dst, point_hi);
    write_u32(dst, point_lo);
}
pub fn length_req_wp_linux_drm_syncobj_surface_v1_set_acquire_point() -> usize {
    20
}
pub fn parse_req_wp_linux_drm_syncobj_surface_v1_set_acquire_point<'a>(
    mut msg: &'a [u8],
) -> Result<(ObjId, u32, u32), &'static str> {
    msg = msg.get(8..).ok_or(PARSE_ERROR)?;
    let arg1 = parse_obj(&mut msg)?;
    let arg2 = parse_u32(&mut msg)?;
    let arg3 = parse_u32(&mut msg)?;
    if !msg.is_empty() {
        return Err(PARSE_ERROR);
    }
    Ok((arg1, arg2, arg3))
}
pub const OPCODE_WP_LINUX_DRM_SYNCOBJ_SURFACE_V1_SET_ACQUIRE_POINT: MethodId = MethodId::Request(1);
pub fn write_req_wp_linux_drm_syncobj_surface_v1_set_release_point(
    dst: &mut &mut [u8],
    for_id: ObjId,
    timeline: ObjId,
    point_hi: u32,
    point_lo: u32,
) {
    let l = length_req_wp_linux_drm_syncobj_surface_v1_set_release_point();
    write_header(dst, for_id, l, 2, 0);
    write_obj(dst, timeline);
    write_u32(dst, point_hi);
    write_u32(dst, point_lo);
}
pub fn length_req_wp_linux_drm_syncobj_surface_v1_set_release_point() -> usize {
    20
}
pub fn parse_req_wp_linux_drm_syncobj_surface_v1_set_release_point<'a>(
    mut msg: &'a [u8],
) -> Result<(ObjId, u32, u32), &'static str> {
    msg = msg.get(8..).ok_or(PARSE_ERROR)?;
    let arg1 = parse_obj(&mut msg)?;
    let arg2 = parse_u32(&mut msg)?;
    let arg3 = parse_u32(&mut msg)?;
    if !msg.is_empty() {
        return Err(PARSE_ERROR);
    }
    Ok((arg1, arg2, arg3))
}
pub const OPCODE_WP_LINUX_DRM_SYNCOBJ_SURFACE_V1_SET_RELEASE_POINT: MethodId = MethodId::Request(2);
const DATA_WP_LINUX_DRM_SYNCOBJ_SURFACE_V1: WaylandData = WaylandData {
    name: "wp_linux_drm_syncobj_surface_v1",
    evts: &[],
    reqs: &[
        WaylandMethod {
            name: "destroy",
            sig: &[],
            destructor: true,
        },
        WaylandMethod {
            name: "set_acquire_point",
            sig: &[Object(WpLinuxDrmSyncobjTimelineV1), Uint, Uint],
            destructor: false,
        },
        WaylandMethod {
            name: "set_release_point",
            sig: &[Object(WpLinuxDrmSyncobjTimelineV1), Uint, Uint],
            destructor: false,
        },
    ],
    version: 1,
};
pub const WP_LINUX_DRM_SYNCOBJ_SURFACE_V1: &[u8] =
    DATA_WP_LINUX_DRM_SYNCOBJ_SURFACE_V1.name.as_bytes();
pub fn write_req_wp_presentation_destroy(dst: &mut &mut [u8], for_id: ObjId) {
    let l = length_req_wp_presentation_destroy();
    write_header(dst, for_id, l, 0, 0);
}
pub fn length_req_wp_presentation_destroy() -> usize {
    8
}
pub fn parse_req_wp_presentation_destroy<'a>(msg: &'a [u8]) -> Result<(), &'static str> {
    if msg.len() != 8 {
        return Err(PARSE_ERROR);
    }
    Ok(())
}
pub const OPCODE_WP_PRESENTATION_DESTROY: MethodId = MethodId::Request(0);
pub fn write_req_wp_presentation_feedback(
    dst: &mut &mut [u8],
    for_id: ObjId,
    surface: ObjId,
    callback: ObjId,
) {
    let l = length_req_wp_presentation_feedback();
    write_header(dst, for_id, l, 1, 0);
    write_obj(dst, surface);
    write_obj(dst, callback);
}
pub fn length_req_wp_presentation_feedback() -> usize {
    16
}
pub fn parse_req_wp_presentation_feedback<'a>(
    mut msg: &'a [u8],
) -> Result<(ObjId, ObjId), &'static str> {
    msg = msg.get(8..).ok_or(PARSE_ERROR)?;
    let arg1 = parse_obj(&mut msg)?;
    let arg2 = parse_obj(&mut msg)?;
    if !msg.is_empty() {
        return Err(PARSE_ERROR);
    }
    Ok((arg1, arg2))
}
pub const OPCODE_WP_PRESENTATION_FEEDBACK: MethodId = MethodId::Request(1);
pub fn write_evt_wp_presentation_clock_id(dst: &mut &mut [u8], for_id: ObjId, clk_id: u32) {
    let l = length_evt_wp_presentation_clock_id();
    write_header(dst, for_id, l, 0, 0);
    write_u32(dst, clk_id);
}
pub fn length_evt_wp_presentation_clock_id() -> usize {
    12
}
pub fn parse_evt_wp_presentation_clock_id<'a>(mut msg: &'a [u8]) -> Result<u32, &'static str> {
    msg = msg.get(8..).ok_or(PARSE_ERROR)?;
    let arg1 = parse_u32(&mut msg)?;
    if !msg.is_empty() {
        return Err(PARSE_ERROR);
    }
    Ok(arg1)
}
pub const OPCODE_WP_PRESENTATION_CLOCK_ID: MethodId = MethodId::Event(0);
const DATA_WP_PRESENTATION: WaylandData = WaylandData {
    name: "wp_presentation",
    evts: &[WaylandMethod {
        name: "clock_id",
        sig: &[Uint],
        destructor: false,
    }],
    reqs: &[
        WaylandMethod {
            name: "destroy",
            sig: &[],
            destructor: true,
        },
        WaylandMethod {
            name: "feedback",
            sig: &[Object(WlSurface), NewId(WpPresentationFeedback)],
            destructor: false,
        },
    ],
    version: 2,
};
pub const WP_PRESENTATION: &[u8] = DATA_WP_PRESENTATION.name.as_bytes();
pub fn write_evt_wp_presentation_feedback_presented(
    dst: &mut &mut [u8],
    for_id: ObjId,
    tv_sec_hi: u32,
    tv_sec_lo: u32,
    tv_nsec: u32,
    refresh: u32,
    seq_hi: u32,
    seq_lo: u32,
    flags: u32,
) {
    let l = length_evt_wp_presentation_feedback_presented();
    write_header(dst, for_id, l, 1, 0);
    write_u32(dst, tv_sec_hi);
    write_u32(dst, tv_sec_lo);
    write_u32(dst, tv_nsec);
    write_u32(dst, refresh);
    write_u32(dst, seq_hi);
    write_u32(dst, seq_lo);
    write_u32(dst, flags);
}
pub fn length_evt_wp_presentation_feedback_presented() -> usize {
    36
}
pub fn parse_evt_wp_presentation_feedback_presented<'a>(
    mut msg: &'a [u8],
) -> Result<(u32, u32, u32, u32, u32, u32, u32), &'static str> {
    msg = msg.get(8..).ok_or(PARSE_ERROR)?;
    let arg1 = parse_u32(&mut msg)?;
    let arg2 = parse_u32(&mut msg)?;
    let arg3 = parse_u32(&mut msg)?;
    let arg4 = parse_u32(&mut msg)?;
    let arg5 = parse_u32(&mut msg)?;
    let arg6 = parse_u32(&mut msg)?;
    let arg7 = parse_u32(&mut msg)?;
    if !msg.is_empty() {
        return Err(PARSE_ERROR);
    }
    Ok((arg1, arg2, arg3, arg4, arg5, arg6, arg7))
}
pub const OPCODE_WP_PRESENTATION_FEEDBACK_PRESENTED: MethodId = MethodId::Event(1);
const DATA_WP_PRESENTATION_FEEDBACK: WaylandData = WaylandData {
    name: "wp_presentation_feedback",
    evts: &[
        WaylandMethod {
            name: "sync_output",
            sig: &[Object(WlOutput)],
            destructor: false,
        },
        WaylandMethod {
            name: "presented",
            sig: &[Uint, Uint, Uint, Uint, Uint, Uint, Uint],
            destructor: true,
        },
        WaylandMethod {
            name: "discarded",
            sig: &[],
            destructor: true,
        },
    ],
    reqs: &[],
    version: 2,
};
pub const WP_PRESENTATION_FEEDBACK: &[u8] = DATA_WP_PRESENTATION_FEEDBACK.name.as_bytes();
pub fn write_req_zwp_primary_selection_device_manager_v1_create_source(
    dst: &mut &mut [u8],
    for_id: ObjId,
    id: ObjId,
) {
    let l = length_req_zwp_primary_selection_device_manager_v1_create_source();
    write_header(dst, for_id, l, 0, 0);
    write_obj(dst, id);
}
pub fn length_req_zwp_primary_selection_device_manager_v1_create_source() -> usize {
    12
}
pub fn parse_req_zwp_primary_selection_device_manager_v1_create_source<'a>(
    mut msg: &'a [u8],
) -> Result<ObjId, &'static str> {
    msg = msg.get(8..).ok_or(PARSE_ERROR)?;
    let arg1 = parse_obj(&mut msg)?;
    if !msg.is_empty() {
        return Err(PARSE_ERROR);
    }
    Ok(arg1)
}
pub const OPCODE_ZWP_PRIMARY_SELECTION_DEVICE_MANAGER_V1_CREATE_SOURCE: MethodId =
    MethodId::Request(0);
pub fn write_req_zwp_primary_selection_device_manager_v1_get_device(
    dst: &mut &mut [u8],
    for_id: ObjId,
    id: ObjId,
    seat: ObjId,
) {
    let l = length_req_zwp_primary_selection_device_manager_v1_get_device();
    write_header(dst, for_id, l, 1, 0);
    write_obj(dst, id);
    write_obj(dst, seat);
}
pub fn length_req_zwp_primary_selection_device_manager_v1_get_device() -> usize {
    16
}
pub fn parse_req_zwp_primary_selection_device_manager_v1_get_device<'a>(
    mut msg: &'a [u8],
) -> Result<(ObjId, ObjId), &'static str> {
    msg = msg.get(8..).ok_or(PARSE_ERROR)?;
    let arg1 = parse_obj(&mut msg)?;
    let arg2 = parse_obj(&mut msg)?;
    if !msg.is_empty() {
        return Err(PARSE_ERROR);
    }
    Ok((arg1, arg2))
}
pub const OPCODE_ZWP_PRIMARY_SELECTION_DEVICE_MANAGER_V1_GET_DEVICE: MethodId =
    MethodId::Request(1);
const DATA_ZWP_PRIMARY_SELECTION_DEVICE_MANAGER_V1: WaylandData = WaylandData {
    name: "zwp_primary_selection_device_manager_v1",
    evts: &[],
    reqs: &[
        WaylandMethod {
            name: "create_source",
            sig: &[NewId(ZwpPrimarySelectionSourceV1)],
            destructor: false,
        },
        WaylandMethod {
            name: "get_device",
            sig: &[NewId(ZwpPrimarySelectionDeviceV1), Object(WlSeat)],
            destructor: false,
        },
        WaylandMethod {
            name: "destroy",
            sig: &[],
            destructor: true,
        },
    ],
    version: 1,
};
pub const ZWP_PRIMARY_SELECTION_DEVICE_MANAGER_V1: &[u8] =
    DATA_ZWP_PRIMARY_SELECTION_DEVICE_MANAGER_V1.name.as_bytes();
pub fn write_req_zwp_primary_selection_device_v1_set_selection(
    dst: &mut &mut [u8],
    for_id: ObjId,
    source: ObjId,
    serial: u32,
) {
    let l = length_req_zwp_primary_selection_device_v1_set_selection();
    write_header(dst, for_id, l, 0, 0);
    write_obj(dst, source);
    write_u32(dst, serial);
}
pub fn length_req_zwp_primary_selection_device_v1_set_selection() -> usize {
    16
}
pub fn parse_req_zwp_primary_selection_device_v1_set_selection<'a>(
    mut msg: &'a [u8],
) -> Result<(ObjId, u32), &'static str> {
    msg = msg.get(8..).ok_or(PARSE_ERROR)?;
    let arg1 = parse_obj(&mut msg)?;
    let arg2 = parse_u32(&mut msg)?;
    if !msg.is_empty() {
        return Err(PARSE_ERROR);
    }
    Ok((arg1, arg2))
}
pub const OPCODE_ZWP_PRIMARY_SELECTION_DEVICE_V1_SET_SELECTION: MethodId = MethodId::Request(0);
pub fn write_evt_zwp_primary_selection_device_v1_data_offer(
    dst: &mut &mut [u8],
    for_id: ObjId,
    offer: ObjId,
) {
    let l = length_evt_zwp_primary_selection_device_v1_data_offer();
    write_header(dst, for_id, l, 0, 0);
    write_obj(dst, offer);
}
pub fn length_evt_zwp_primary_selection_device_v1_data_offer() -> usize {
    12
}
pub fn parse_evt_zwp_primary_selection_device_v1_data_offer<'a>(
    mut msg: &'a [u8],
) -> Result<ObjId, &'static str> {
    msg = msg.get(8..).ok_or(PARSE_ERROR)?;
    let arg1 = parse_obj(&mut msg)?;
    if !msg.is_empty() {
        return Err(PARSE_ERROR);
    }
    Ok(arg1)
}
pub const OPCODE_ZWP_PRIMARY_SELECTION_DEVICE_V1_DATA_OFFER: MethodId = MethodId::Event(0);
pub fn write_evt_zwp_primary_selection_device_v1_selection(
    dst: &mut &mut [u8],
    for_id: ObjId,
    id: ObjId,
) {
    let l = length_evt_zwp_primary_selection_device_v1_selection();
    write_header(dst, for_id, l, 1, 0);
    write_obj(dst, id);
}
pub fn length_evt_zwp_primary_selection_device_v1_selection() -> usize {
    12
}
pub fn parse_evt_zwp_primary_selection_device_v1_selection<'a>(
    mut msg: &'a [u8],
) -> Result<ObjId, &'static str> {
    msg = msg.get(8..).ok_or(PARSE_ERROR)?;
    let arg1 = parse_obj(&mut msg)?;
    if !msg.is_empty() {
        return Err(PARSE_ERROR);
    }
    Ok(arg1)
}
pub const OPCODE_ZWP_PRIMARY_SELECTION_DEVICE_V1_SELECTION: MethodId = MethodId::Event(1);
const DATA_ZWP_PRIMARY_SELECTION_DEVICE_V1: WaylandData = WaylandData {
    name: "zwp_primary_selection_device_v1",
    evts: &[
        WaylandMethod {
            name: "data_offer",
            sig: &[NewId(ZwpPrimarySelectionOfferV1)],
            destructor: false,
        },
        WaylandMethod {
            name: "selection",
            sig: &[Object(ZwpPrimarySelectionOfferV1)],
            destructor: false,
        },
    ],
    reqs: &[
        WaylandMethod {
            name: "set_selection",
            sig: &[Object(ZwpPrimarySelectionSourceV1), Uint],
            destructor: false,
        },
        WaylandMethod {
            name: "destroy",
            sig: &[],
            destructor: true,
        },
    ],
    version: 1,
};
pub const ZWP_PRIMARY_SELECTION_DEVICE_V1: &[u8] =
    DATA_ZWP_PRIMARY_SELECTION_DEVICE_V1.name.as_bytes();
pub fn write_req_zwp_primary_selection_offer_v1_receive(
    dst: &mut &mut [u8],
    for_id: ObjId,
    tag_fds: bool,
    mime_type: &[u8],
) {
    let l = length_req_zwp_primary_selection_offer_v1_receive(mime_type.len());
    write_header(dst, for_id, l, 0, if tag_fds { 1 } else { 0 });
    write_string(dst, Some(mime_type));
}
pub fn length_req_zwp_primary_selection_offer_v1_receive(mime_type_len: usize) -> usize {
    let mut v = 8;
    v += length_string(mime_type_len);
    v
}
pub fn parse_req_zwp_primary_selection_offer_v1_receive<'a>(
    mut msg: &'a [u8],
) -> Result<&'a [u8], &'static str> {
    msg = msg.get(8..).ok_or(PARSE_ERROR)?;
    let arg1 = parse_string(&mut msg)?.ok_or(PARSE_ERROR)?;
    if !msg.is_empty() {
        return Err(PARSE_ERROR);
    }
    Ok(arg1)
}
pub const OPCODE_ZWP_PRIMARY_SELECTION_OFFER_V1_RECEIVE: MethodId = MethodId::Request(0);
pub fn write_evt_zwp_primary_selection_offer_v1_offer(
    dst: &mut &mut [u8],
    for_id: ObjId,
    mime_type: &[u8],
) {
    let l = length_evt_zwp_primary_selection_offer_v1_offer(mime_type.len());
    write_header(dst, for_id, l, 0, 0);
    write_string(dst, Some(mime_type));
}
pub fn length_evt_zwp_primary_selection_offer_v1_offer(mime_type_len: usize) -> usize {
    let mut v = 8;
    v += length_string(mime_type_len);
    v
}
pub fn parse_evt_zwp_primary_selection_offer_v1_offer<'a>(
    mut msg: &'a [u8],
) -> Result<&'a [u8], &'static str> {
    msg = msg.get(8..).ok_or(PARSE_ERROR)?;
    let arg1 = parse_string(&mut msg)?.ok_or(PARSE_ERROR)?;
    if !msg.is_empty() {
        return Err(PARSE_ERROR);
    }
    Ok(arg1)
}
pub const OPCODE_ZWP_PRIMARY_SELECTION_OFFER_V1_OFFER: MethodId = MethodId::Event(0);
const DATA_ZWP_PRIMARY_SELECTION_OFFER_V1: WaylandData = WaylandData {
    name: "zwp_primary_selection_offer_v1",
    evts: &[WaylandMethod {
        name: "offer",
        sig: &[String],
        destructor: false,
    }],
    reqs: &[
        WaylandMethod {
            name: "receive",
            sig: &[String, Fd],
            destructor: false,
        },
        WaylandMethod {
            name: "destroy",
            sig: &[],
            destructor: true,
        },
    ],
    version: 1,
};
pub const ZWP_PRIMARY_SELECTION_OFFER_V1: &[u8] =
    DATA_ZWP_PRIMARY_SELECTION_OFFER_V1.name.as_bytes();
pub fn write_req_zwp_primary_selection_source_v1_offer(
    dst: &mut &mut [u8],
    for_id: ObjId,
    mime_type: &[u8],
) {
    let l = length_req_zwp_primary_selection_source_v1_offer(mime_type.len());
    write_header(dst, for_id, l, 0, 0);
    write_string(dst, Some(mime_type));
}
pub fn length_req_zwp_primary_selection_source_v1_offer(mime_type_len: usize) -> usize {
    let mut v = 8;
    v += length_string(mime_type_len);
    v
}
pub fn parse_req_zwp_primary_selection_source_v1_offer<'a>(
    mut msg: &'a [u8],
) -> Result<&'a [u8], &'static str> {
    msg = msg.get(8..).ok_or(PARSE_ERROR)?;
    let arg1 = parse_string(&mut msg)?.ok_or(PARSE_ERROR)?;
    if !msg.is_empty() {
        return Err(PARSE_ERROR);
    }
    Ok(arg1)
}
pub const OPCODE_ZWP_PRIMARY_SELECTION_SOURCE_V1_OFFER: MethodId = MethodId::Request(0);
pub fn write_evt_zwp_primary_selection_source_v1_send(
    dst: &mut &mut [u8],
    for_id: ObjId,
    tag_fds: bool,
    mime_type: &[u8],
) {
    let l = length_evt_zwp_primary_selection_source_v1_send(mime_type.len());
    write_header(dst, for_id, l, 0, if tag_fds { 1 } else { 0 });
    write_string(dst, Some(mime_type));
}
pub fn length_evt_zwp_primary_selection_source_v1_send(mime_type_len: usize) -> usize {
    let mut v = 8;
    v += length_string(mime_type_len);
    v
}
pub fn parse_evt_zwp_primary_selection_source_v1_send<'a>(
    mut msg: &'a [u8],
) -> Result<&'a [u8], &'static str> {
    msg = msg.get(8..).ok_or(PARSE_ERROR)?;
    let arg1 = parse_string(&mut msg)?.ok_or(PARSE_ERROR)?;
    if !msg.is_empty() {
        return Err(PARSE_ERROR);
    }
    Ok(arg1)
}
pub const OPCODE_ZWP_PRIMARY_SELECTION_SOURCE_V1_SEND: MethodId = MethodId::Event(0);
const DATA_ZWP_PRIMARY_SELECTION_SOURCE_V1: WaylandData = WaylandData {
    name: "zwp_primary_selection_source_v1",
    evts: &[
        WaylandMethod {
            name: "send",
            sig: &[String, Fd],
            destructor: false,
        },
        WaylandMethod {
            name: "cancelled",
            sig: &[],
            destructor: false,
        },
    ],
    reqs: &[
        WaylandMethod {
            name: "offer",
            sig: &[String],
            destructor: false,
        },
        WaylandMethod {
            name: "destroy",
            sig: &[],
            destructor: true,
        },
    ],
    version: 1,
};
pub const ZWP_PRIMARY_SELECTION_SOURCE_V1: &[u8] =
    DATA_ZWP_PRIMARY_SELECTION_SOURCE_V1.name.as_bytes();
pub fn write_req_wp_security_context_manager_v1_create_listener(
    dst: &mut &mut [u8],
    for_id: ObjId,
    tag_fds: bool,
    id: ObjId,
) {
    let l = length_req_wp_security_context_manager_v1_create_listener();
    write_header(dst, for_id, l, 1, if tag_fds { 2 } else { 0 });
    write_obj(dst, id);
}
pub fn length_req_wp_security_context_manager_v1_create_listener() -> usize {
    12
}
pub fn parse_req_wp_security_context_manager_v1_create_listener<'a>(
    mut msg: &'a [u8],
) -> Result<ObjId, &'static str> {
    msg = msg.get(8..).ok_or(PARSE_ERROR)?;
    let arg1 = parse_obj(&mut msg)?;
    if !msg.is_empty() {
        return Err(PARSE_ERROR);
    }
    Ok(arg1)
}
pub const OPCODE_WP_SECURITY_CONTEXT_MANAGER_V1_CREATE_LISTENER: MethodId = MethodId::Request(1);
const DATA_WP_SECURITY_CONTEXT_MANAGER_V1: WaylandData = WaylandData {
    name: "wp_security_context_manager_v1",
    evts: &[],
    reqs: &[
        WaylandMethod {
            name: "destroy",
            sig: &[],
            destructor: true,
        },
        WaylandMethod {
            name: "create_listener",
            sig: &[NewId(WpSecurityContextV1), Fd, Fd],
            destructor: false,
        },
    ],
    version: 1,
};
pub const WP_SECURITY_CONTEXT_MANAGER_V1: &[u8] =
    DATA_WP_SECURITY_CONTEXT_MANAGER_V1.name.as_bytes();
pub fn write_req_wp_security_context_v1_set_sandbox_engine(
    dst: &mut &mut [u8],
    for_id: ObjId,
    name: &[u8],
) {
    let l = length_req_wp_security_context_v1_set_sandbox_engine(name.len());
    write_header(dst, for_id, l, 1, 0);
    write_string(dst, Some(name));
}
pub fn length_req_wp_security_context_v1_set_sandbox_engine(name_len: usize) -> usize {
    let mut v = 8;
    v += length_string(name_len);
    v
}
pub fn parse_req_wp_security_context_v1_set_sandbox_engine<'a>(
    mut msg: &'a [u8],
) -> Result<&'a [u8], &'static str> {
    msg = msg.get(8..).ok_or(PARSE_ERROR)?;
    let arg1 = parse_string(&mut msg)?.ok_or(PARSE_ERROR)?;
    if !msg.is_empty() {
        return Err(PARSE_ERROR);
    }
    Ok(arg1)
}
pub const OPCODE_WP_SECURITY_CONTEXT_V1_SET_SANDBOX_ENGINE: MethodId = MethodId::Request(1);
pub fn write_req_wp_security_context_v1_set_app_id(
    dst: &mut &mut [u8],
    for_id: ObjId,
    app_id: &[u8],
) {
    let l = length_req_wp_security_context_v1_set_app_id(app_id.len());
    write_header(dst, for_id, l, 2, 0);
    write_string(dst, Some(app_id));
}
pub fn length_req_wp_security_context_v1_set_app_id(app_id_len: usize) -> usize {
    let mut v = 8;
    v += length_string(app_id_len);
    v
}
pub fn parse_req_wp_security_context_v1_set_app_id<'a>(
    mut msg: &'a [u8],
) -> Result<&'a [u8], &'static str> {
    msg = msg.get(8..).ok_or(PARSE_ERROR)?;
    let arg1 = parse_string(&mut msg)?.ok_or(PARSE_ERROR)?;
    if !msg.is_empty() {
        return Err(PARSE_ERROR);
    }
    Ok(arg1)
}
pub const OPCODE_WP_SECURITY_CONTEXT_V1_SET_APP_ID: MethodId = MethodId::Request(2);
pub fn write_req_wp_security_context_v1_set_instance_id(
    dst: &mut &mut [u8],
    for_id: ObjId,
    instance_id: &[u8],
) {
    let l = length_req_wp_security_context_v1_set_instance_id(instance_id.len());
    write_header(dst, for_id, l, 3, 0);
    write_string(dst, Some(instance_id));
}
pub fn length_req_wp_security_context_v1_set_instance_id(instance_id_len: usize) -> usize {
    let mut v = 8;
    v += length_string(instance_id_len);
    v
}
pub fn parse_req_wp_security_context_v1_set_instance_id<'a>(
    mut msg: &'a [u8],
) -> Result<&'a [u8], &'static str> {
    msg = msg.get(8..).ok_or(PARSE_ERROR)?;
    let arg1 = parse_string(&mut msg)?.ok_or(PARSE_ERROR)?;
    if !msg.is_empty() {
        return Err(PARSE_ERROR);
    }
    Ok(arg1)
}
pub const OPCODE_WP_SECURITY_CONTEXT_V1_SET_INSTANCE_ID: MethodId = MethodId::Request(3);
pub fn write_req_wp_security_context_v1_commit(dst: &mut &mut [u8], for_id: ObjId) {
    let l = length_req_wp_security_context_v1_commit();
    write_header(dst, for_id, l, 4, 0);
}
pub fn length_req_wp_security_context_v1_commit() -> usize {
    8
}
pub fn parse_req_wp_security_context_v1_commit<'a>(msg: &'a [u8]) -> Result<(), &'static str> {
    if msg.len() != 8 {
        return Err(PARSE_ERROR);
    }
    Ok(())
}
pub const OPCODE_WP_SECURITY_CONTEXT_V1_COMMIT: MethodId = MethodId::Request(4);
const DATA_WP_SECURITY_CONTEXT_V1: WaylandData = WaylandData {
    name: "wp_security_context_v1",
    evts: &[],
    reqs: &[
        WaylandMethod {
            name: "destroy",
            sig: &[],
            destructor: true,
        },
        WaylandMethod {
            name: "set_sandbox_engine",
            sig: &[String],
            destructor: false,
        },
        WaylandMethod {
            name: "set_app_id",
            sig: &[String],
            destructor: false,
        },
        WaylandMethod {
            name: "set_instance_id",
            sig: &[String],
            destructor: false,
        },
        WaylandMethod {
            name: "commit",
            sig: &[],
            destructor: false,
        },
    ],
    version: 1,
};
pub const WP_SECURITY_CONTEXT_V1: &[u8] = DATA_WP_SECURITY_CONTEXT_V1.name.as_bytes();
pub fn write_req_wp_viewporter_get_viewport(
    dst: &mut &mut [u8],
    for_id: ObjId,
    id: ObjId,
    surface: ObjId,
) {
    let l = length_req_wp_viewporter_get_viewport();
    write_header(dst, for_id, l, 1, 0);
    write_obj(dst, id);
    write_obj(dst, surface);
}
pub fn length_req_wp_viewporter_get_viewport() -> usize {
    16
}
pub fn parse_req_wp_viewporter_get_viewport<'a>(
    mut msg: &'a [u8],
) -> Result<(ObjId, ObjId), &'static str> {
    msg = msg.get(8..).ok_or(PARSE_ERROR)?;
    let arg1 = parse_obj(&mut msg)?;
    let arg2 = parse_obj(&mut msg)?;
    if !msg.is_empty() {
        return Err(PARSE_ERROR);
    }
    Ok((arg1, arg2))
}
pub const OPCODE_WP_VIEWPORTER_GET_VIEWPORT: MethodId = MethodId::Request(1);
const DATA_WP_VIEWPORTER: WaylandData = WaylandData {
    name: "wp_viewporter",
    evts: &[],
    reqs: &[
        WaylandMethod {
            name: "destroy",
            sig: &[],
            destructor: true,
        },
        WaylandMethod {
            name: "get_viewport",
            sig: &[NewId(WpViewport), Object(WlSurface)],
            destructor: false,
        },
    ],
    version: 1,
};
pub const WP_VIEWPORTER: &[u8] = DATA_WP_VIEWPORTER.name.as_bytes();
pub fn write_req_wp_viewport_destroy(dst: &mut &mut [u8], for_id: ObjId) {
    let l = length_req_wp_viewport_destroy();
    write_header(dst, for_id, l, 0, 0);
}
pub fn length_req_wp_viewport_destroy() -> usize {
    8
}
pub fn parse_req_wp_viewport_destroy<'a>(msg: &'a [u8]) -> Result<(), &'static str> {
    if msg.len() != 8 {
        return Err(PARSE_ERROR);
    }
    Ok(())
}
pub const OPCODE_WP_VIEWPORT_DESTROY: MethodId = MethodId::Request(0);
pub fn write_req_wp_viewport_set_source(
    dst: &mut &mut [u8],
    for_id: ObjId,
    x: i32,
    y: i32,
    width: i32,
    height: i32,
) {
    let l = length_req_wp_viewport_set_source();
    write_header(dst, for_id, l, 1, 0);
    write_i32(dst, x);
    write_i32(dst, y);
    write_i32(dst, width);
    write_i32(dst, height);
}
pub fn length_req_wp_viewport_set_source() -> usize {
    24
}
pub fn parse_req_wp_viewport_set_source<'a>(
    mut msg: &'a [u8],
) -> Result<(i32, i32, i32, i32), &'static str> {
    msg = msg.get(8..).ok_or(PARSE_ERROR)?;
    let arg1 = parse_i32(&mut msg)?;
    let arg2 = parse_i32(&mut msg)?;
    let arg3 = parse_i32(&mut msg)?;
    let arg4 = parse_i32(&mut msg)?;
    if !msg.is_empty() {
        return Err(PARSE_ERROR);
    }
    Ok((arg1, arg2, arg3, arg4))
}
pub const OPCODE_WP_VIEWPORT_SET_SOURCE: MethodId = MethodId::Request(1);
pub fn write_req_wp_viewport_set_destination(
    dst: &mut &mut [u8],
    for_id: ObjId,
    width: i32,
    height: i32,
) {
    let l = length_req_wp_viewport_set_destination();
    write_header(dst, for_id, l, 2, 0);
    write_i32(dst, width);
    write_i32(dst, height);
}
pub fn length_req_wp_viewport_set_destination() -> usize {
    16
}
pub fn parse_req_wp_viewport_set_destination<'a>(
    mut msg: &'a [u8],
) -> Result<(i32, i32), &'static str> {
    msg = msg.get(8..).ok_or(PARSE_ERROR)?;
    let arg1 = parse_i32(&mut msg)?;
    let arg2 = parse_i32(&mut msg)?;
    if !msg.is_empty() {
        return Err(PARSE_ERROR);
    }
    Ok((arg1, arg2))
}
pub const OPCODE_WP_VIEWPORT_SET_DESTINATION: MethodId = MethodId::Request(2);
const DATA_WP_VIEWPORT: WaylandData = WaylandData {
    name: "wp_viewport",
    evts: &[],
    reqs: &[
        WaylandMethod {
            name: "destroy",
            sig: &[],
            destructor: true,
        },
        WaylandMethod {
            name: "set_source",
            sig: &[Fixed, Fixed, Fixed, Fixed],
            destructor: false,
        },
        WaylandMethod {
            name: "set_destination",
            sig: &[Int, Int],
            destructor: false,
        },
    ],
    version: 1,
};
pub const WP_VIEWPORT: &[u8] = DATA_WP_VIEWPORT.name.as_bytes();
const DATA_ZWP_VIRTUAL_KEYBOARD_V1: WaylandData = WaylandData {
    name: "zwp_virtual_keyboard_v1",
    evts: &[],
    reqs: &[
        WaylandMethod {
            name: "keymap",
            sig: &[Uint, Fd, Uint],
            destructor: false,
        },
        WaylandMethod {
            name: "key",
            sig: &[Uint, Uint, Uint],
            destructor: false,
        },
        WaylandMethod {
            name: "modifiers",
            sig: &[Uint, Uint, Uint, Uint],
            destructor: false,
        },
        WaylandMethod {
            name: "destroy",
            sig: &[],
            destructor: true,
        },
    ],
    version: 1,
};
pub const ZWP_VIRTUAL_KEYBOARD_V1: &[u8] = DATA_ZWP_VIRTUAL_KEYBOARD_V1.name.as_bytes();
const DATA_ZWP_VIRTUAL_KEYBOARD_MANAGER_V1: WaylandData = WaylandData {
    name: "zwp_virtual_keyboard_manager_v1",
    evts: &[],
    reqs: &[WaylandMethod {
        name: "create_virtual_keyboard",
        sig: &[Object(WlSeat), NewId(ZwpVirtualKeyboardV1)],
        destructor: false,
    }],
    version: 1,
};
pub const ZWP_VIRTUAL_KEYBOARD_MANAGER_V1: &[u8] =
    DATA_ZWP_VIRTUAL_KEYBOARD_MANAGER_V1.name.as_bytes();
const DATA_WL_DRM: WaylandData = WaylandData {
    name: "wl_drm",
    evts: &[
        WaylandMethod {
            name: "device",
            sig: &[String],
            destructor: false,
        },
        WaylandMethod {
            name: "format",
            sig: &[Uint],
            destructor: false,
        },
        WaylandMethod {
            name: "authenticated",
            sig: &[],
            destructor: false,
        },
        WaylandMethod {
            name: "capabilities",
            sig: &[Uint],
            destructor: false,
        },
    ],
    reqs: &[
        WaylandMethod {
            name: "authenticate",
            sig: &[Uint],
            destructor: false,
        },
        WaylandMethod {
            name: "create_buffer",
            sig: &[NewId(WlBuffer), Uint, Int, Int, Uint, Uint],
            destructor: false,
        },
        WaylandMethod {
            name: "create_planar_buffer",
            sig: &[
                NewId(WlBuffer),
                Uint,
                Int,
                Int,
                Uint,
                Int,
                Int,
                Int,
                Int,
                Int,
                Int,
            ],
            destructor: false,
        },
        WaylandMethod {
            name: "create_prime_buffer",
            sig: &[
                NewId(WlBuffer),
                Fd,
                Int,
                Int,
                Uint,
                Int,
                Int,
                Int,
                Int,
                Int,
                Int,
            ],
            destructor: false,
        },
    ],
    version: 2,
};
pub const WL_DRM: &[u8] = DATA_WL_DRM.name.as_bytes();
pub fn write_req_wl_display_sync(dst: &mut &mut [u8], for_id: ObjId, callback: ObjId) {
    let l = length_req_wl_display_sync();
    write_header(dst, for_id, l, 0, 0);
    write_obj(dst, callback);
}
pub fn length_req_wl_display_sync() -> usize {
    12
}
pub fn parse_req_wl_display_sync<'a>(mut msg: &'a [u8]) -> Result<ObjId, &'static str> {
    msg = msg.get(8..).ok_or(PARSE_ERROR)?;
    let arg1 = parse_obj(&mut msg)?;
    if !msg.is_empty() {
        return Err(PARSE_ERROR);
    }
    Ok(arg1)
}
pub const OPCODE_WL_DISPLAY_SYNC: MethodId = MethodId::Request(0);
pub fn write_req_wl_display_get_registry(dst: &mut &mut [u8], for_id: ObjId, registry: ObjId) {
    let l = length_req_wl_display_get_registry();
    write_header(dst, for_id, l, 1, 0);
    write_obj(dst, registry);
}
pub fn length_req_wl_display_get_registry() -> usize {
    12
}
pub fn parse_req_wl_display_get_registry<'a>(mut msg: &'a [u8]) -> Result<ObjId, &'static str> {
    msg = msg.get(8..).ok_or(PARSE_ERROR)?;
    let arg1 = parse_obj(&mut msg)?;
    if !msg.is_empty() {
        return Err(PARSE_ERROR);
    }
    Ok(arg1)
}
pub const OPCODE_WL_DISPLAY_GET_REGISTRY: MethodId = MethodId::Request(1);
pub fn write_evt_wl_display_error(
    dst: &mut &mut [u8],
    for_id: ObjId,
    object_id: ObjId,
    code: u32,
    message: &[u8],
) {
    let l = length_evt_wl_display_error(message.len());
    write_header(dst, for_id, l, 0, 0);
    write_obj(dst, object_id);
    write_u32(dst, code);
    write_string(dst, Some(message));
}
pub fn length_evt_wl_display_error(message_len: usize) -> usize {
    let mut v = 16;
    v += length_string(message_len);
    v
}
pub fn parse_evt_wl_display_error<'a>(
    mut msg: &'a [u8],
) -> Result<(ObjId, u32, &'a [u8]), &'static str> {
    msg = msg.get(8..).ok_or(PARSE_ERROR)?;
    let arg1 = parse_obj(&mut msg)?;
    let arg2 = parse_u32(&mut msg)?;
    let arg3 = parse_string(&mut msg)?.ok_or(PARSE_ERROR)?;
    if !msg.is_empty() {
        return Err(PARSE_ERROR);
    }
    Ok((arg1, arg2, arg3))
}
pub const OPCODE_WL_DISPLAY_ERROR: MethodId = MethodId::Event(0);
pub fn write_evt_wl_display_delete_id(dst: &mut &mut [u8], for_id: ObjId, id: u32) {
    let l = length_evt_wl_display_delete_id();
    write_header(dst, for_id, l, 1, 0);
    write_u32(dst, id);
}
pub fn length_evt_wl_display_delete_id() -> usize {
    12
}
pub fn parse_evt_wl_display_delete_id<'a>(mut msg: &'a [u8]) -> Result<u32, &'static str> {
    msg = msg.get(8..).ok_or(PARSE_ERROR)?;
    let arg1 = parse_u32(&mut msg)?;
    if !msg.is_empty() {
        return Err(PARSE_ERROR);
    }
    Ok(arg1)
}
pub const OPCODE_WL_DISPLAY_DELETE_ID: MethodId = MethodId::Event(1);
const DATA_WL_DISPLAY: WaylandData = WaylandData {
    name: "wl_display",
    evts: &[
        WaylandMethod {
            name: "error",
            sig: &[GenericObject, Uint, String],
            destructor: false,
        },
        WaylandMethod {
            name: "delete_id",
            sig: &[Uint],
            destructor: false,
        },
    ],
    reqs: &[
        WaylandMethod {
            name: "sync",
            sig: &[NewId(WlCallback)],
            destructor: false,
        },
        WaylandMethod {
            name: "get_registry",
            sig: &[NewId(WlRegistry)],
            destructor: false,
        },
    ],
    version: 1,
};
pub const WL_DISPLAY: &[u8] = DATA_WL_DISPLAY.name.as_bytes();
pub fn write_req_wl_registry_bind(
    dst: &mut &mut [u8],
    for_id: ObjId,
    name: u32,
    id_iface_name: &[u8],
    id_version: u32,
    id: ObjId,
) {
    let l = length_req_wl_registry_bind(id_iface_name.len());
    write_header(dst, for_id, l, 0, 0);
    write_u32(dst, name);
    write_string(dst, Some(id_iface_name));
    write_u32(dst, id_version);
    write_obj(dst, id);
}
pub fn length_req_wl_registry_bind(id_iface_name_len: usize) -> usize {
    let mut v = 20;
    v += length_string(id_iface_name_len);
    v
}
pub fn parse_req_wl_registry_bind<'a>(
    mut msg: &'a [u8],
) -> Result<(u32, &'a [u8], u32, ObjId), &'static str> {
    msg = msg.get(8..).ok_or(PARSE_ERROR)?;
    let arg1 = parse_u32(&mut msg)?;
    let arg2_iface_name = parse_string(&mut msg)?.ok_or(PARSE_ERROR)?;
    let arg2_version = parse_u32(&mut msg)?;
    let arg2 = parse_obj(&mut msg)?;
    if !msg.is_empty() {
        return Err(PARSE_ERROR);
    }
    Ok((arg1, arg2_iface_name, arg2_version, arg2))
}
pub const OPCODE_WL_REGISTRY_BIND: MethodId = MethodId::Request(0);
pub fn write_evt_wl_registry_global(
    dst: &mut &mut [u8],
    for_id: ObjId,
    name: u32,
    interface: &[u8],
    version: u32,
) {
    let l = length_evt_wl_registry_global(interface.len());
    write_header(dst, for_id, l, 0, 0);
    write_u32(dst, name);
    write_string(dst, Some(interface));
    write_u32(dst, version);
}
pub fn length_evt_wl_registry_global(interface_len: usize) -> usize {
    let mut v = 16;
    v += length_string(interface_len);
    v
}
pub fn parse_evt_wl_registry_global<'a>(
    mut msg: &'a [u8],
) -> Result<(u32, &'a [u8], u32), &'static str> {
    msg = msg.get(8..).ok_or(PARSE_ERROR)?;
    let arg1 = parse_u32(&mut msg)?;
    let arg2 = parse_string(&mut msg)?.ok_or(PARSE_ERROR)?;
    let arg3 = parse_u32(&mut msg)?;
    if !msg.is_empty() {
        return Err(PARSE_ERROR);
    }
    Ok((arg1, arg2, arg3))
}
pub const OPCODE_WL_REGISTRY_GLOBAL: MethodId = MethodId::Event(0);
pub fn write_evt_wl_registry_global_remove(dst: &mut &mut [u8], for_id: ObjId, name: u32) {
    let l = length_evt_wl_registry_global_remove();
    write_header(dst, for_id, l, 1, 0);
    write_u32(dst, name);
}
pub fn length_evt_wl_registry_global_remove() -> usize {
    12
}
pub fn parse_evt_wl_registry_global_remove<'a>(mut msg: &'a [u8]) -> Result<u32, &'static str> {
    msg = msg.get(8..).ok_or(PARSE_ERROR)?;
    let arg1 = parse_u32(&mut msg)?;
    if !msg.is_empty() {
        return Err(PARSE_ERROR);
    }
    Ok(arg1)
}
pub const OPCODE_WL_REGISTRY_GLOBAL_REMOVE: MethodId = MethodId::Event(1);
const DATA_WL_REGISTRY: WaylandData = WaylandData {
    name: "wl_registry",
    evts: &[
        WaylandMethod {
            name: "global",
            sig: &[Uint, String, Uint],
            destructor: false,
        },
        WaylandMethod {
            name: "global_remove",
            sig: &[Uint],
            destructor: false,
        },
    ],
    reqs: &[WaylandMethod {
        name: "bind",
        sig: &[Uint, GenericNewId],
        destructor: false,
    }],
    version: 1,
};
pub const WL_REGISTRY: &[u8] = DATA_WL_REGISTRY.name.as_bytes();
pub fn write_evt_wl_callback_done(dst: &mut &mut [u8], for_id: ObjId, callback_data: u32) {
    let l = length_evt_wl_callback_done();
    write_header(dst, for_id, l, 0, 0);
    write_u32(dst, callback_data);
}
pub fn length_evt_wl_callback_done() -> usize {
    12
}
pub fn parse_evt_wl_callback_done<'a>(mut msg: &'a [u8]) -> Result<u32, &'static str> {
    msg = msg.get(8..).ok_or(PARSE_ERROR)?;
    let arg1 = parse_u32(&mut msg)?;
    if !msg.is_empty() {
        return Err(PARSE_ERROR);
    }
    Ok(arg1)
}
pub const OPCODE_WL_CALLBACK_DONE: MethodId = MethodId::Event(0);
const DATA_WL_CALLBACK: WaylandData = WaylandData {
    name: "wl_callback",
    evts: &[WaylandMethod {
        name: "done",
        sig: &[Uint],
        destructor: true,
    }],
    reqs: &[],
    version: 1,
};
pub const WL_CALLBACK: &[u8] = DATA_WL_CALLBACK.name.as_bytes();
pub fn write_req_wl_compositor_create_surface(dst: &mut &mut [u8], for_id: ObjId, id: ObjId) {
    let l = length_req_wl_compositor_create_surface();
    write_header(dst, for_id, l, 0, 0);
    write_obj(dst, id);
}
pub fn length_req_wl_compositor_create_surface() -> usize {
    12
}
pub fn parse_req_wl_compositor_create_surface<'a>(
    mut msg: &'a [u8],
) -> Result<ObjId, &'static str> {
    msg = msg.get(8..).ok_or(PARSE_ERROR)?;
    let arg1 = parse_obj(&mut msg)?;
    if !msg.is_empty() {
        return Err(PARSE_ERROR);
    }
    Ok(arg1)
}
pub const OPCODE_WL_COMPOSITOR_CREATE_SURFACE: MethodId = MethodId::Request(0);
const DATA_WL_COMPOSITOR: WaylandData = WaylandData {
    name: "wl_compositor",
    evts: &[],
    reqs: &[
        WaylandMethod {
            name: "create_surface",
            sig: &[NewId(WlSurface)],
            destructor: false,
        },
        WaylandMethod {
            name: "create_region",
            sig: &[NewId(WlRegion)],
            destructor: false,
        },
    ],
    version: 6,
};
pub const WL_COMPOSITOR: &[u8] = DATA_WL_COMPOSITOR.name.as_bytes();
pub fn write_req_wl_shm_pool_create_buffer(
    dst: &mut &mut [u8],
    for_id: ObjId,
    id: ObjId,
    offset: i32,
    width: i32,
    height: i32,
    stride: i32,
    format: u32,
) {
    let l = length_req_wl_shm_pool_create_buffer();
    write_header(dst, for_id, l, 0, 0);
    write_obj(dst, id);
    write_i32(dst, offset);
    write_i32(dst, width);
    write_i32(dst, height);
    write_i32(dst, stride);
    write_u32(dst, format);
}
pub fn length_req_wl_shm_pool_create_buffer() -> usize {
    32
}
pub fn parse_req_wl_shm_pool_create_buffer<'a>(
    mut msg: &'a [u8],
) -> Result<(ObjId, i32, i32, i32, i32, u32), &'static str> {
    msg = msg.get(8..).ok_or(PARSE_ERROR)?;
    let arg1 = parse_obj(&mut msg)?;
    let arg2 = parse_i32(&mut msg)?;
    let arg3 = parse_i32(&mut msg)?;
    let arg4 = parse_i32(&mut msg)?;
    let arg5 = parse_i32(&mut msg)?;
    let arg6 = parse_u32(&mut msg)?;
    if !msg.is_empty() {
        return Err(PARSE_ERROR);
    }
    Ok((arg1, arg2, arg3, arg4, arg5, arg6))
}
pub const OPCODE_WL_SHM_POOL_CREATE_BUFFER: MethodId = MethodId::Request(0);
pub fn write_req_wl_shm_pool_destroy(dst: &mut &mut [u8], for_id: ObjId) {
    let l = length_req_wl_shm_pool_destroy();
    write_header(dst, for_id, l, 1, 0);
}
pub fn length_req_wl_shm_pool_destroy() -> usize {
    8
}
pub fn parse_req_wl_shm_pool_destroy<'a>(msg: &'a [u8]) -> Result<(), &'static str> {
    if msg.len() != 8 {
        return Err(PARSE_ERROR);
    }
    Ok(())
}
pub const OPCODE_WL_SHM_POOL_DESTROY: MethodId = MethodId::Request(1);
pub fn write_req_wl_shm_pool_resize(dst: &mut &mut [u8], for_id: ObjId, size: i32) {
    let l = length_req_wl_shm_pool_resize();
    write_header(dst, for_id, l, 2, 0);
    write_i32(dst, size);
}
pub fn length_req_wl_shm_pool_resize() -> usize {
    12
}
pub fn parse_req_wl_shm_pool_resize<'a>(mut msg: &'a [u8]) -> Result<i32, &'static str> {
    msg = msg.get(8..).ok_or(PARSE_ERROR)?;
    let arg1 = parse_i32(&mut msg)?;
    if !msg.is_empty() {
        return Err(PARSE_ERROR);
    }
    Ok(arg1)
}
pub const OPCODE_WL_SHM_POOL_RESIZE: MethodId = MethodId::Request(2);
const DATA_WL_SHM_POOL: WaylandData = WaylandData {
    name: "wl_shm_pool",
    evts: &[],
    reqs: &[
        WaylandMethod {
            name: "create_buffer",
            sig: &[NewId(WlBuffer), Int, Int, Int, Int, Uint],
            destructor: false,
        },
        WaylandMethod {
            name: "destroy",
            sig: &[],
            destructor: true,
        },
        WaylandMethod {
            name: "resize",
            sig: &[Int],
            destructor: false,
        },
    ],
    version: 2,
};
pub const WL_SHM_POOL: &[u8] = DATA_WL_SHM_POOL.name.as_bytes();
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WlShmFormat {
    Argb8888 = 0,
    Xrgb8888 = 1,
    C8 = 0x20203843,
    Rgb332 = 0x38424752,
    Bgr233 = 0x38524742,
    Xrgb4444 = 0x32315258,
    Xbgr4444 = 0x32314258,
    Rgbx4444 = 0x32315852,
    Bgrx4444 = 0x32315842,
    Argb4444 = 0x32315241,
    Abgr4444 = 0x32314241,
    Rgba4444 = 0x32314152,
    Bgra4444 = 0x32314142,
    Xrgb1555 = 0x35315258,
    Xbgr1555 = 0x35314258,
    Rgbx5551 = 0x35315852,
    Bgrx5551 = 0x35315842,
    Argb1555 = 0x35315241,
    Abgr1555 = 0x35314241,
    Rgba5551 = 0x35314152,
    Bgra5551 = 0x35314142,
    Rgb565 = 0x36314752,
    Bgr565 = 0x36314742,
    Rgb888 = 0x34324752,
    Bgr888 = 0x34324742,
    Xbgr8888 = 0x34324258,
    Rgbx8888 = 0x34325852,
    Bgrx8888 = 0x34325842,
    Abgr8888 = 0x34324241,
    Rgba8888 = 0x34324152,
    Bgra8888 = 0x34324142,
    Xrgb2101010 = 0x30335258,
    Xbgr2101010 = 0x30334258,
    Rgbx1010102 = 0x30335852,
    Bgrx1010102 = 0x30335842,
    Argb2101010 = 0x30335241,
    Abgr2101010 = 0x30334241,
    Rgba1010102 = 0x30334152,
    Bgra1010102 = 0x30334142,
    Yuyv = 0x56595559,
    Yvyu = 0x55595659,
    Uyvy = 0x59565955,
    Vyuy = 0x59555956,
    Ayuv = 0x56555941,
    Nv12 = 0x3231564e,
    Nv21 = 0x3132564e,
    Nv16 = 0x3631564e,
    Nv61 = 0x3136564e,
    Yuv410 = 0x39565559,
    Yvu410 = 0x39555659,
    Yuv411 = 0x31315559,
    Yvu411 = 0x31315659,
    Yuv420 = 0x32315559,
    Yvu420 = 0x32315659,
    Yuv422 = 0x36315559,
    Yvu422 = 0x36315659,
    Yuv444 = 0x34325559,
    Yvu444 = 0x34325659,
    R8 = 0x20203852,
    R16 = 0x20363152,
    Rg88 = 0x38384752,
    Gr88 = 0x38385247,
    Rg1616 = 0x32334752,
    Gr1616 = 0x32335247,
    Xrgb16161616f = 0x48345258,
    Xbgr16161616f = 0x48344258,
    Argb16161616f = 0x48345241,
    Abgr16161616f = 0x48344241,
    Xyuv8888 = 0x56555958,
    Vuy888 = 0x34325556,
    Vuy101010 = 0x30335556,
    Y210 = 0x30313259,
    Y212 = 0x32313259,
    Y216 = 0x36313259,
    Y410 = 0x30313459,
    Y412 = 0x32313459,
    Y416 = 0x36313459,
    Xvyu2101010 = 0x30335658,
    Xvyu1216161616 = 0x36335658,
    Xvyu16161616 = 0x38345658,
    Y0l0 = 0x304c3059,
    X0l0 = 0x304c3058,
    Y0l2 = 0x324c3059,
    X0l2 = 0x324c3058,
    Yuv4208bit = 0x38305559,
    Yuv42010bit = 0x30315559,
    Xrgb8888A8 = 0x38415258,
    Xbgr8888A8 = 0x38414258,
    Rgbx8888A8 = 0x38415852,
    Bgrx8888A8 = 0x38415842,
    Rgb888A8 = 0x38413852,
    Bgr888A8 = 0x38413842,
    Rgb565A8 = 0x38413552,
    Bgr565A8 = 0x38413542,
    Nv24 = 0x3432564e,
    Nv42 = 0x3234564e,
    P210 = 0x30313250,
    P010 = 0x30313050,
    P012 = 0x32313050,
    P016 = 0x36313050,
    Axbxgxrx106106106106 = 0x30314241,
    Nv15 = 0x3531564e,
    Q410 = 0x30313451,
    Q401 = 0x31303451,
    Xrgb16161616 = 0x38345258,
    Xbgr16161616 = 0x38344258,
    Argb16161616 = 0x38345241,
    Abgr16161616 = 0x38344241,
    C1 = 0x20203143,
    C2 = 0x20203243,
    C4 = 0x20203443,
    D1 = 0x20203144,
    D2 = 0x20203244,
    D4 = 0x20203444,
    D8 = 0x20203844,
    R1 = 0x20203152,
    R2 = 0x20203252,
    R4 = 0x20203452,
    R10 = 0x20303152,
    R12 = 0x20323152,
    Avuy8888 = 0x59555641,
    Xvuy8888 = 0x59555658,
    P030 = 0x30333050,
}
impl TryFrom<u32> for WlShmFormat {
    type Error = ();
    fn try_from(v: u32) -> Result<Self, Self::Error> {
        Ok(match v {
            0 => WlShmFormat::Argb8888,
            1 => WlShmFormat::Xrgb8888,
            0x20203843 => WlShmFormat::C8,
            0x38424752 => WlShmFormat::Rgb332,
            0x38524742 => WlShmFormat::Bgr233,
            0x32315258 => WlShmFormat::Xrgb4444,
            0x32314258 => WlShmFormat::Xbgr4444,
            0x32315852 => WlShmFormat::Rgbx4444,
            0x32315842 => WlShmFormat::Bgrx4444,
            0x32315241 => WlShmFormat::Argb4444,
            0x32314241 => WlShmFormat::Abgr4444,
            0x32314152 => WlShmFormat::Rgba4444,
            0x32314142 => WlShmFormat::Bgra4444,
            0x35315258 => WlShmFormat::Xrgb1555,
            0x35314258 => WlShmFormat::Xbgr1555,
            0x35315852 => WlShmFormat::Rgbx5551,
            0x35315842 => WlShmFormat::Bgrx5551,
            0x35315241 => WlShmFormat::Argb1555,
            0x35314241 => WlShmFormat::Abgr1555,
            0x35314152 => WlShmFormat::Rgba5551,
            0x35314142 => WlShmFormat::Bgra5551,
            0x36314752 => WlShmFormat::Rgb565,
            0x36314742 => WlShmFormat::Bgr565,
            0x34324752 => WlShmFormat::Rgb888,
            0x34324742 => WlShmFormat::Bgr888,
            0x34324258 => WlShmFormat::Xbgr8888,
            0x34325852 => WlShmFormat::Rgbx8888,
            0x34325842 => WlShmFormat::Bgrx8888,
            0x34324241 => WlShmFormat::Abgr8888,
            0x34324152 => WlShmFormat::Rgba8888,
            0x34324142 => WlShmFormat::Bgra8888,
            0x30335258 => WlShmFormat::Xrgb2101010,
            0x30334258 => WlShmFormat::Xbgr2101010,
            0x30335852 => WlShmFormat::Rgbx1010102,
            0x30335842 => WlShmFormat::Bgrx1010102,
            0x30335241 => WlShmFormat::Argb2101010,
            0x30334241 => WlShmFormat::Abgr2101010,
            0x30334152 => WlShmFormat::Rgba1010102,
            0x30334142 => WlShmFormat::Bgra1010102,
            0x56595559 => WlShmFormat::Yuyv,
            0x55595659 => WlShmFormat::Yvyu,
            0x59565955 => WlShmFormat::Uyvy,
            0x59555956 => WlShmFormat::Vyuy,
            0x56555941 => WlShmFormat::Ayuv,
            0x3231564e => WlShmFormat::Nv12,
            0x3132564e => WlShmFormat::Nv21,
            0x3631564e => WlShmFormat::Nv16,
            0x3136564e => WlShmFormat::Nv61,
            0x39565559 => WlShmFormat::Yuv410,
            0x39555659 => WlShmFormat::Yvu410,
            0x31315559 => WlShmFormat::Yuv411,
            0x31315659 => WlShmFormat::Yvu411,
            0x32315559 => WlShmFormat::Yuv420,
            0x32315659 => WlShmFormat::Yvu420,
            0x36315559 => WlShmFormat::Yuv422,
            0x36315659 => WlShmFormat::Yvu422,
            0x34325559 => WlShmFormat::Yuv444,
            0x34325659 => WlShmFormat::Yvu444,
            0x20203852 => WlShmFormat::R8,
            0x20363152 => WlShmFormat::R16,
            0x38384752 => WlShmFormat::Rg88,
            0x38385247 => WlShmFormat::Gr88,
            0x32334752 => WlShmFormat::Rg1616,
            0x32335247 => WlShmFormat::Gr1616,
            0x48345258 => WlShmFormat::Xrgb16161616f,
            0x48344258 => WlShmFormat::Xbgr16161616f,
            0x48345241 => WlShmFormat::Argb16161616f,
            0x48344241 => WlShmFormat::Abgr16161616f,
            0x56555958 => WlShmFormat::Xyuv8888,
            0x34325556 => WlShmFormat::Vuy888,
            0x30335556 => WlShmFormat::Vuy101010,
            0x30313259 => WlShmFormat::Y210,
            0x32313259 => WlShmFormat::Y212,
            0x36313259 => WlShmFormat::Y216,
            0x30313459 => WlShmFormat::Y410,
            0x32313459 => WlShmFormat::Y412,
            0x36313459 => WlShmFormat::Y416,
            0x30335658 => WlShmFormat::Xvyu2101010,
            0x36335658 => WlShmFormat::Xvyu1216161616,
            0x38345658 => WlShmFormat::Xvyu16161616,
            0x304c3059 => WlShmFormat::Y0l0,
            0x304c3058 => WlShmFormat::X0l0,
            0x324c3059 => WlShmFormat::Y0l2,
            0x324c3058 => WlShmFormat::X0l2,
            0x38305559 => WlShmFormat::Yuv4208bit,
            0x30315559 => WlShmFormat::Yuv42010bit,
            0x38415258 => WlShmFormat::Xrgb8888A8,
            0x38414258 => WlShmFormat::Xbgr8888A8,
            0x38415852 => WlShmFormat::Rgbx8888A8,
            0x38415842 => WlShmFormat::Bgrx8888A8,
            0x38413852 => WlShmFormat::Rgb888A8,
            0x38413842 => WlShmFormat::Bgr888A8,
            0x38413552 => WlShmFormat::Rgb565A8,
            0x38413542 => WlShmFormat::Bgr565A8,
            0x3432564e => WlShmFormat::Nv24,
            0x3234564e => WlShmFormat::Nv42,
            0x30313250 => WlShmFormat::P210,
            0x30313050 => WlShmFormat::P010,
            0x32313050 => WlShmFormat::P012,
            0x36313050 => WlShmFormat::P016,
            0x30314241 => WlShmFormat::Axbxgxrx106106106106,
            0x3531564e => WlShmFormat::Nv15,
            0x30313451 => WlShmFormat::Q410,
            0x31303451 => WlShmFormat::Q401,
            0x38345258 => WlShmFormat::Xrgb16161616,
            0x38344258 => WlShmFormat::Xbgr16161616,
            0x38345241 => WlShmFormat::Argb16161616,
            0x38344241 => WlShmFormat::Abgr16161616,
            0x20203143 => WlShmFormat::C1,
            0x20203243 => WlShmFormat::C2,
            0x20203443 => WlShmFormat::C4,
            0x20203144 => WlShmFormat::D1,
            0x20203244 => WlShmFormat::D2,
            0x20203444 => WlShmFormat::D4,
            0x20203844 => WlShmFormat::D8,
            0x20203152 => WlShmFormat::R1,
            0x20203252 => WlShmFormat::R2,
            0x20203452 => WlShmFormat::R4,
            0x20303152 => WlShmFormat::R10,
            0x20323152 => WlShmFormat::R12,
            0x59555641 => WlShmFormat::Avuy8888,
            0x59555658 => WlShmFormat::Xvuy8888,
            0x30333050 => WlShmFormat::P030,
            _ => return Err(()),
        })
    }
}
pub fn write_req_wl_shm_create_pool(
    dst: &mut &mut [u8],
    for_id: ObjId,
    tag_fds: bool,
    id: ObjId,
    size: i32,
) {
    let l = length_req_wl_shm_create_pool();
    write_header(dst, for_id, l, 0, if tag_fds { 1 } else { 0 });
    write_obj(dst, id);
    write_i32(dst, size);
}
pub fn length_req_wl_shm_create_pool() -> usize {
    16
}
pub fn parse_req_wl_shm_create_pool<'a>(mut msg: &'a [u8]) -> Result<(ObjId, i32), &'static str> {
    msg = msg.get(8..).ok_or(PARSE_ERROR)?;
    let arg1 = parse_obj(&mut msg)?;
    let arg3 = parse_i32(&mut msg)?;
    if !msg.is_empty() {
        return Err(PARSE_ERROR);
    }
    Ok((arg1, arg3))
}
pub const OPCODE_WL_SHM_CREATE_POOL: MethodId = MethodId::Request(0);
const DATA_WL_SHM: WaylandData = WaylandData {
    name: "wl_shm",
    evts: &[WaylandMethod {
        name: "format",
        sig: &[Uint],
        destructor: false,
    }],
    reqs: &[
        WaylandMethod {
            name: "create_pool",
            sig: &[NewId(WlShmPool), Fd, Int],
            destructor: false,
        },
        WaylandMethod {
            name: "release",
            sig: &[],
            destructor: true,
        },
    ],
    version: 2,
};
pub const WL_SHM: &[u8] = DATA_WL_SHM.name.as_bytes();
pub fn write_req_wl_buffer_destroy(dst: &mut &mut [u8], for_id: ObjId) {
    let l = length_req_wl_buffer_destroy();
    write_header(dst, for_id, l, 0, 0);
}
pub fn length_req_wl_buffer_destroy() -> usize {
    8
}
pub fn parse_req_wl_buffer_destroy<'a>(msg: &'a [u8]) -> Result<(), &'static str> {
    if msg.len() != 8 {
        return Err(PARSE_ERROR);
    }
    Ok(())
}
pub const OPCODE_WL_BUFFER_DESTROY: MethodId = MethodId::Request(0);
pub fn write_evt_wl_buffer_release(dst: &mut &mut [u8], for_id: ObjId) {
    let l = length_evt_wl_buffer_release();
    write_header(dst, for_id, l, 0, 0);
}
pub fn length_evt_wl_buffer_release() -> usize {
    8
}
pub fn parse_evt_wl_buffer_release<'a>(msg: &'a [u8]) -> Result<(), &'static str> {
    if msg.len() != 8 {
        return Err(PARSE_ERROR);
    }
    Ok(())
}
pub const OPCODE_WL_BUFFER_RELEASE: MethodId = MethodId::Event(0);
const DATA_WL_BUFFER: WaylandData = WaylandData {
    name: "wl_buffer",
    evts: &[WaylandMethod {
        name: "release",
        sig: &[],
        destructor: false,
    }],
    reqs: &[WaylandMethod {
        name: "destroy",
        sig: &[],
        destructor: true,
    }],
    version: 1,
};
pub const WL_BUFFER: &[u8] = DATA_WL_BUFFER.name.as_bytes();
pub fn write_req_wl_data_offer_receive(
    dst: &mut &mut [u8],
    for_id: ObjId,
    tag_fds: bool,
    mime_type: &[u8],
) {
    let l = length_req_wl_data_offer_receive(mime_type.len());
    write_header(dst, for_id, l, 1, if tag_fds { 1 } else { 0 });
    write_string(dst, Some(mime_type));
}
pub fn length_req_wl_data_offer_receive(mime_type_len: usize) -> usize {
    let mut v = 8;
    v += length_string(mime_type_len);
    v
}
pub fn parse_req_wl_data_offer_receive<'a>(mut msg: &'a [u8]) -> Result<&'a [u8], &'static str> {
    msg = msg.get(8..).ok_or(PARSE_ERROR)?;
    let arg1 = parse_string(&mut msg)?.ok_or(PARSE_ERROR)?;
    if !msg.is_empty() {
        return Err(PARSE_ERROR);
    }
    Ok(arg1)
}
pub const OPCODE_WL_DATA_OFFER_RECEIVE: MethodId = MethodId::Request(1);
pub fn write_evt_wl_data_offer_offer(dst: &mut &mut [u8], for_id: ObjId, mime_type: &[u8]) {
    let l = length_evt_wl_data_offer_offer(mime_type.len());
    write_header(dst, for_id, l, 0, 0);
    write_string(dst, Some(mime_type));
}
pub fn length_evt_wl_data_offer_offer(mime_type_len: usize) -> usize {
    let mut v = 8;
    v += length_string(mime_type_len);
    v
}
pub fn parse_evt_wl_data_offer_offer<'a>(mut msg: &'a [u8]) -> Result<&'a [u8], &'static str> {
    msg = msg.get(8..).ok_or(PARSE_ERROR)?;
    let arg1 = parse_string(&mut msg)?.ok_or(PARSE_ERROR)?;
    if !msg.is_empty() {
        return Err(PARSE_ERROR);
    }
    Ok(arg1)
}
pub const OPCODE_WL_DATA_OFFER_OFFER: MethodId = MethodId::Event(0);
const DATA_WL_DATA_OFFER: WaylandData = WaylandData {
    name: "wl_data_offer",
    evts: &[
        WaylandMethod {
            name: "offer",
            sig: &[String],
            destructor: false,
        },
        WaylandMethod {
            name: "source_actions",
            sig: &[Uint],
            destructor: false,
        },
        WaylandMethod {
            name: "action",
            sig: &[Uint],
            destructor: false,
        },
    ],
    reqs: &[
        WaylandMethod {
            name: "accept",
            sig: &[Uint, OptionalString],
            destructor: false,
        },
        WaylandMethod {
            name: "receive",
            sig: &[String, Fd],
            destructor: false,
        },
        WaylandMethod {
            name: "destroy",
            sig: &[],
            destructor: true,
        },
        WaylandMethod {
            name: "finish",
            sig: &[],
            destructor: false,
        },
        WaylandMethod {
            name: "set_actions",
            sig: &[Uint, Uint],
            destructor: false,
        },
    ],
    version: 3,
};
pub const WL_DATA_OFFER: &[u8] = DATA_WL_DATA_OFFER.name.as_bytes();
pub fn write_req_wl_data_source_offer(dst: &mut &mut [u8], for_id: ObjId, mime_type: &[u8]) {
    let l = length_req_wl_data_source_offer(mime_type.len());
    write_header(dst, for_id, l, 0, 0);
    write_string(dst, Some(mime_type));
}
pub fn length_req_wl_data_source_offer(mime_type_len: usize) -> usize {
    let mut v = 8;
    v += length_string(mime_type_len);
    v
}
pub fn parse_req_wl_data_source_offer<'a>(mut msg: &'a [u8]) -> Result<&'a [u8], &'static str> {
    msg = msg.get(8..).ok_or(PARSE_ERROR)?;
    let arg1 = parse_string(&mut msg)?.ok_or(PARSE_ERROR)?;
    if !msg.is_empty() {
        return Err(PARSE_ERROR);
    }
    Ok(arg1)
}
pub const OPCODE_WL_DATA_SOURCE_OFFER: MethodId = MethodId::Request(0);
pub fn write_evt_wl_data_source_send(
    dst: &mut &mut [u8],
    for_id: ObjId,
    tag_fds: bool,
    mime_type: &[u8],
) {
    let l = length_evt_wl_data_source_send(mime_type.len());
    write_header(dst, for_id, l, 1, if tag_fds { 1 } else { 0 });
    write_string(dst, Some(mime_type));
}
pub fn length_evt_wl_data_source_send(mime_type_len: usize) -> usize {
    let mut v = 8;
    v += length_string(mime_type_len);
    v
}
pub fn parse_evt_wl_data_source_send<'a>(mut msg: &'a [u8]) -> Result<&'a [u8], &'static str> {
    msg = msg.get(8..).ok_or(PARSE_ERROR)?;
    let arg1 = parse_string(&mut msg)?.ok_or(PARSE_ERROR)?;
    if !msg.is_empty() {
        return Err(PARSE_ERROR);
    }
    Ok(arg1)
}
pub const OPCODE_WL_DATA_SOURCE_SEND: MethodId = MethodId::Event(1);
const DATA_WL_DATA_SOURCE: WaylandData = WaylandData {
    name: "wl_data_source",
    evts: &[
        WaylandMethod {
            name: "target",
            sig: &[OptionalString],
            destructor: false,
        },
        WaylandMethod {
            name: "send",
            sig: &[String, Fd],
            destructor: false,
        },
        WaylandMethod {
            name: "cancelled",
            sig: &[],
            destructor: false,
        },
        WaylandMethod {
            name: "dnd_drop_performed",
            sig: &[],
            destructor: false,
        },
        WaylandMethod {
            name: "dnd_finished",
            sig: &[],
            destructor: false,
        },
        WaylandMethod {
            name: "action",
            sig: &[Uint],
            destructor: false,
        },
    ],
    reqs: &[
        WaylandMethod {
            name: "offer",
            sig: &[String],
            destructor: false,
        },
        WaylandMethod {
            name: "destroy",
            sig: &[],
            destructor: true,
        },
        WaylandMethod {
            name: "set_actions",
            sig: &[Uint],
            destructor: false,
        },
    ],
    version: 3,
};
pub const WL_DATA_SOURCE: &[u8] = DATA_WL_DATA_SOURCE.name.as_bytes();
pub fn write_req_wl_data_device_set_selection(
    dst: &mut &mut [u8],
    for_id: ObjId,
    source: ObjId,
    serial: u32,
) {
    let l = length_req_wl_data_device_set_selection();
    write_header(dst, for_id, l, 1, 0);
    write_obj(dst, source);
    write_u32(dst, serial);
}
pub fn length_req_wl_data_device_set_selection() -> usize {
    16
}
pub fn parse_req_wl_data_device_set_selection<'a>(
    mut msg: &'a [u8],
) -> Result<(ObjId, u32), &'static str> {
    msg = msg.get(8..).ok_or(PARSE_ERROR)?;
    let arg1 = parse_obj(&mut msg)?;
    let arg2 = parse_u32(&mut msg)?;
    if !msg.is_empty() {
        return Err(PARSE_ERROR);
    }
    Ok((arg1, arg2))
}
pub const OPCODE_WL_DATA_DEVICE_SET_SELECTION: MethodId = MethodId::Request(1);
pub fn write_evt_wl_data_device_data_offer(dst: &mut &mut [u8], for_id: ObjId, id: ObjId) {
    let l = length_evt_wl_data_device_data_offer();
    write_header(dst, for_id, l, 0, 0);
    write_obj(dst, id);
}
pub fn length_evt_wl_data_device_data_offer() -> usize {
    12
}
pub fn parse_evt_wl_data_device_data_offer<'a>(mut msg: &'a [u8]) -> Result<ObjId, &'static str> {
    msg = msg.get(8..).ok_or(PARSE_ERROR)?;
    let arg1 = parse_obj(&mut msg)?;
    if !msg.is_empty() {
        return Err(PARSE_ERROR);
    }
    Ok(arg1)
}
pub const OPCODE_WL_DATA_DEVICE_DATA_OFFER: MethodId = MethodId::Event(0);
pub fn write_evt_wl_data_device_selection(dst: &mut &mut [u8], for_id: ObjId, id: ObjId) {
    let l = length_evt_wl_data_device_selection();
    write_header(dst, for_id, l, 5, 0);
    write_obj(dst, id);
}
pub fn length_evt_wl_data_device_selection() -> usize {
    12
}
pub fn parse_evt_wl_data_device_selection<'a>(mut msg: &'a [u8]) -> Result<ObjId, &'static str> {
    msg = msg.get(8..).ok_or(PARSE_ERROR)?;
    let arg1 = parse_obj(&mut msg)?;
    if !msg.is_empty() {
        return Err(PARSE_ERROR);
    }
    Ok(arg1)
}
pub const OPCODE_WL_DATA_DEVICE_SELECTION: MethodId = MethodId::Event(5);
const DATA_WL_DATA_DEVICE: WaylandData = WaylandData {
    name: "wl_data_device",
    evts: &[
        WaylandMethod {
            name: "data_offer",
            sig: &[NewId(WlDataOffer)],
            destructor: false,
        },
        WaylandMethod {
            name: "enter",
            sig: &[Uint, Object(WlSurface), Fixed, Fixed, Object(WlDataOffer)],
            destructor: false,
        },
        WaylandMethod {
            name: "leave",
            sig: &[],
            destructor: false,
        },
        WaylandMethod {
            name: "motion",
            sig: &[Uint, Fixed, Fixed],
            destructor: false,
        },
        WaylandMethod {
            name: "drop",
            sig: &[],
            destructor: false,
        },
        WaylandMethod {
            name: "selection",
            sig: &[Object(WlDataOffer)],
            destructor: false,
        },
    ],
    reqs: &[
        WaylandMethod {
            name: "start_drag",
            sig: &[
                Object(WlDataSource),
                Object(WlSurface),
                Object(WlSurface),
                Uint,
            ],
            destructor: false,
        },
        WaylandMethod {
            name: "set_selection",
            sig: &[Object(WlDataSource), Uint],
            destructor: false,
        },
        WaylandMethod {
            name: "release",
            sig: &[],
            destructor: true,
        },
    ],
    version: 3,
};
pub const WL_DATA_DEVICE: &[u8] = DATA_WL_DATA_DEVICE.name.as_bytes();
pub fn write_req_wl_data_device_manager_create_data_source(
    dst: &mut &mut [u8],
    for_id: ObjId,
    id: ObjId,
) {
    let l = length_req_wl_data_device_manager_create_data_source();
    write_header(dst, for_id, l, 0, 0);
    write_obj(dst, id);
}
pub fn length_req_wl_data_device_manager_create_data_source() -> usize {
    12
}
pub fn parse_req_wl_data_device_manager_create_data_source<'a>(
    mut msg: &'a [u8],
) -> Result<ObjId, &'static str> {
    msg = msg.get(8..).ok_or(PARSE_ERROR)?;
    let arg1 = parse_obj(&mut msg)?;
    if !msg.is_empty() {
        return Err(PARSE_ERROR);
    }
    Ok(arg1)
}
pub const OPCODE_WL_DATA_DEVICE_MANAGER_CREATE_DATA_SOURCE: MethodId = MethodId::Request(0);
pub fn write_req_wl_data_device_manager_get_data_device(
    dst: &mut &mut [u8],
    for_id: ObjId,
    id: ObjId,
    seat: ObjId,
) {
    let l = length_req_wl_data_device_manager_get_data_device();
    write_header(dst, for_id, l, 1, 0);
    write_obj(dst, id);
    write_obj(dst, seat);
}
pub fn length_req_wl_data_device_manager_get_data_device() -> usize {
    16
}
pub fn parse_req_wl_data_device_manager_get_data_device<'a>(
    mut msg: &'a [u8],
) -> Result<(ObjId, ObjId), &'static str> {
    msg = msg.get(8..).ok_or(PARSE_ERROR)?;
    let arg1 = parse_obj(&mut msg)?;
    let arg2 = parse_obj(&mut msg)?;
    if !msg.is_empty() {
        return Err(PARSE_ERROR);
    }
    Ok((arg1, arg2))
}
pub const OPCODE_WL_DATA_DEVICE_MANAGER_GET_DATA_DEVICE: MethodId = MethodId::Request(1);
const DATA_WL_DATA_DEVICE_MANAGER: WaylandData = WaylandData {
    name: "wl_data_device_manager",
    evts: &[],
    reqs: &[
        WaylandMethod {
            name: "create_data_source",
            sig: &[NewId(WlDataSource)],
            destructor: false,
        },
        WaylandMethod {
            name: "get_data_device",
            sig: &[NewId(WlDataDevice), Object(WlSeat)],
            destructor: false,
        },
    ],
    version: 3,
};
pub const WL_DATA_DEVICE_MANAGER: &[u8] = DATA_WL_DATA_DEVICE_MANAGER.name.as_bytes();
const DATA_WL_SHELL: WaylandData = WaylandData {
    name: "wl_shell",
    evts: &[],
    reqs: &[WaylandMethod {
        name: "get_shell_surface",
        sig: &[NewId(WlShellSurface), Object(WlSurface)],
        destructor: false,
    }],
    version: 1,
};
pub const WL_SHELL: &[u8] = DATA_WL_SHELL.name.as_bytes();
const DATA_WL_SHELL_SURFACE: WaylandData = WaylandData {
    name: "wl_shell_surface",
    evts: &[
        WaylandMethod {
            name: "ping",
            sig: &[Uint],
            destructor: false,
        },
        WaylandMethod {
            name: "configure",
            sig: &[Uint, Int, Int],
            destructor: false,
        },
        WaylandMethod {
            name: "popup_done",
            sig: &[],
            destructor: false,
        },
    ],
    reqs: &[
        WaylandMethod {
            name: "pong",
            sig: &[Uint],
            destructor: false,
        },
        WaylandMethod {
            name: "move",
            sig: &[Object(WlSeat), Uint],
            destructor: false,
        },
        WaylandMethod {
            name: "resize",
            sig: &[Object(WlSeat), Uint, Uint],
            destructor: false,
        },
        WaylandMethod {
            name: "set_toplevel",
            sig: &[],
            destructor: false,
        },
        WaylandMethod {
            name: "set_transient",
            sig: &[Object(WlSurface), Int, Int, Uint],
            destructor: false,
        },
        WaylandMethod {
            name: "set_fullscreen",
            sig: &[Uint, Uint, Object(WlOutput)],
            destructor: false,
        },
        WaylandMethod {
            name: "set_popup",
            sig: &[Object(WlSeat), Uint, Object(WlSurface), Int, Int, Uint],
            destructor: false,
        },
        WaylandMethod {
            name: "set_maximized",
            sig: &[Object(WlOutput)],
            destructor: false,
        },
        WaylandMethod {
            name: "set_title",
            sig: &[String],
            destructor: false,
        },
        WaylandMethod {
            name: "set_class",
            sig: &[String],
            destructor: false,
        },
    ],
    version: 1,
};
pub const WL_SHELL_SURFACE: &[u8] = DATA_WL_SHELL_SURFACE.name.as_bytes();
pub fn write_req_wl_surface_destroy(dst: &mut &mut [u8], for_id: ObjId) {
    let l = length_req_wl_surface_destroy();
    write_header(dst, for_id, l, 0, 0);
}
pub fn length_req_wl_surface_destroy() -> usize {
    8
}
pub fn parse_req_wl_surface_destroy<'a>(msg: &'a [u8]) -> Result<(), &'static str> {
    if msg.len() != 8 {
        return Err(PARSE_ERROR);
    }
    Ok(())
}
pub const OPCODE_WL_SURFACE_DESTROY: MethodId = MethodId::Request(0);
pub fn write_req_wl_surface_attach(
    dst: &mut &mut [u8],
    for_id: ObjId,
    buffer: ObjId,
    x: i32,
    y: i32,
) {
    let l = length_req_wl_surface_attach();
    write_header(dst, for_id, l, 1, 0);
    write_obj(dst, buffer);
    write_i32(dst, x);
    write_i32(dst, y);
}
pub fn length_req_wl_surface_attach() -> usize {
    20
}
pub fn parse_req_wl_surface_attach<'a>(
    mut msg: &'a [u8],
) -> Result<(ObjId, i32, i32), &'static str> {
    msg = msg.get(8..).ok_or(PARSE_ERROR)?;
    let arg1 = parse_obj(&mut msg)?;
    let arg2 = parse_i32(&mut msg)?;
    let arg3 = parse_i32(&mut msg)?;
    if !msg.is_empty() {
        return Err(PARSE_ERROR);
    }
    Ok((arg1, arg2, arg3))
}
pub const OPCODE_WL_SURFACE_ATTACH: MethodId = MethodId::Request(1);
pub fn write_req_wl_surface_damage(
    dst: &mut &mut [u8],
    for_id: ObjId,
    x: i32,
    y: i32,
    width: i32,
    height: i32,
) {
    let l = length_req_wl_surface_damage();
    write_header(dst, for_id, l, 2, 0);
    write_i32(dst, x);
    write_i32(dst, y);
    write_i32(dst, width);
    write_i32(dst, height);
}
pub fn length_req_wl_surface_damage() -> usize {
    24
}
pub fn parse_req_wl_surface_damage<'a>(
    mut msg: &'a [u8],
) -> Result<(i32, i32, i32, i32), &'static str> {
    msg = msg.get(8..).ok_or(PARSE_ERROR)?;
    let arg1 = parse_i32(&mut msg)?;
    let arg2 = parse_i32(&mut msg)?;
    let arg3 = parse_i32(&mut msg)?;
    let arg4 = parse_i32(&mut msg)?;
    if !msg.is_empty() {
        return Err(PARSE_ERROR);
    }
    Ok((arg1, arg2, arg3, arg4))
}
pub const OPCODE_WL_SURFACE_DAMAGE: MethodId = MethodId::Request(2);
pub fn write_req_wl_surface_commit(dst: &mut &mut [u8], for_id: ObjId) {
    let l = length_req_wl_surface_commit();
    write_header(dst, for_id, l, 6, 0);
}
pub fn length_req_wl_surface_commit() -> usize {
    8
}
pub fn parse_req_wl_surface_commit<'a>(msg: &'a [u8]) -> Result<(), &'static str> {
    if msg.len() != 8 {
        return Err(PARSE_ERROR);
    }
    Ok(())
}
pub const OPCODE_WL_SURFACE_COMMIT: MethodId = MethodId::Request(6);
pub fn write_req_wl_surface_set_buffer_transform(
    dst: &mut &mut [u8],
    for_id: ObjId,
    transform: i32,
) {
    let l = length_req_wl_surface_set_buffer_transform();
    write_header(dst, for_id, l, 7, 0);
    write_i32(dst, transform);
}
pub fn length_req_wl_surface_set_buffer_transform() -> usize {
    12
}
pub fn parse_req_wl_surface_set_buffer_transform<'a>(
    mut msg: &'a [u8],
) -> Result<i32, &'static str> {
    msg = msg.get(8..).ok_or(PARSE_ERROR)?;
    let arg1 = parse_i32(&mut msg)?;
    if !msg.is_empty() {
        return Err(PARSE_ERROR);
    }
    Ok(arg1)
}
pub const OPCODE_WL_SURFACE_SET_BUFFER_TRANSFORM: MethodId = MethodId::Request(7);
pub fn write_req_wl_surface_set_buffer_scale(dst: &mut &mut [u8], for_id: ObjId, scale: i32) {
    let l = length_req_wl_surface_set_buffer_scale();
    write_header(dst, for_id, l, 8, 0);
    write_i32(dst, scale);
}
pub fn length_req_wl_surface_set_buffer_scale() -> usize {
    12
}
pub fn parse_req_wl_surface_set_buffer_scale<'a>(mut msg: &'a [u8]) -> Result<i32, &'static str> {
    msg = msg.get(8..).ok_or(PARSE_ERROR)?;
    let arg1 = parse_i32(&mut msg)?;
    if !msg.is_empty() {
        return Err(PARSE_ERROR);
    }
    Ok(arg1)
}
pub const OPCODE_WL_SURFACE_SET_BUFFER_SCALE: MethodId = MethodId::Request(8);
pub fn write_req_wl_surface_damage_buffer(
    dst: &mut &mut [u8],
    for_id: ObjId,
    x: i32,
    y: i32,
    width: i32,
    height: i32,
) {
    let l = length_req_wl_surface_damage_buffer();
    write_header(dst, for_id, l, 9, 0);
    write_i32(dst, x);
    write_i32(dst, y);
    write_i32(dst, width);
    write_i32(dst, height);
}
pub fn length_req_wl_surface_damage_buffer() -> usize {
    24
}
pub fn parse_req_wl_surface_damage_buffer<'a>(
    mut msg: &'a [u8],
) -> Result<(i32, i32, i32, i32), &'static str> {
    msg = msg.get(8..).ok_or(PARSE_ERROR)?;
    let arg1 = parse_i32(&mut msg)?;
    let arg2 = parse_i32(&mut msg)?;
    let arg3 = parse_i32(&mut msg)?;
    let arg4 = parse_i32(&mut msg)?;
    if !msg.is_empty() {
        return Err(PARSE_ERROR);
    }
    Ok((arg1, arg2, arg3, arg4))
}
pub const OPCODE_WL_SURFACE_DAMAGE_BUFFER: MethodId = MethodId::Request(9);
const DATA_WL_SURFACE: WaylandData = WaylandData {
    name: "wl_surface",
    evts: &[
        WaylandMethod {
            name: "enter",
            sig: &[Object(WlOutput)],
            destructor: false,
        },
        WaylandMethod {
            name: "leave",
            sig: &[Object(WlOutput)],
            destructor: false,
        },
        WaylandMethod {
            name: "preferred_buffer_scale",
            sig: &[Int],
            destructor: false,
        },
        WaylandMethod {
            name: "preferred_buffer_transform",
            sig: &[Uint],
            destructor: false,
        },
    ],
    reqs: &[
        WaylandMethod {
            name: "destroy",
            sig: &[],
            destructor: true,
        },
        WaylandMethod {
            name: "attach",
            sig: &[Object(WlBuffer), Int, Int],
            destructor: false,
        },
        WaylandMethod {
            name: "damage",
            sig: &[Int, Int, Int, Int],
            destructor: false,
        },
        WaylandMethod {
            name: "frame",
            sig: &[NewId(WlCallback)],
            destructor: false,
        },
        WaylandMethod {
            name: "set_opaque_region",
            sig: &[Object(WlRegion)],
            destructor: false,
        },
        WaylandMethod {
            name: "set_input_region",
            sig: &[Object(WlRegion)],
            destructor: false,
        },
        WaylandMethod {
            name: "commit",
            sig: &[],
            destructor: false,
        },
        WaylandMethod {
            name: "set_buffer_transform",
            sig: &[Int],
            destructor: false,
        },
        WaylandMethod {
            name: "set_buffer_scale",
            sig: &[Int],
            destructor: false,
        },
        WaylandMethod {
            name: "damage_buffer",
            sig: &[Int, Int, Int, Int],
            destructor: false,
        },
        WaylandMethod {
            name: "offset",
            sig: &[Int, Int],
            destructor: false,
        },
    ],
    version: 6,
};
pub const WL_SURFACE: &[u8] = DATA_WL_SURFACE.name.as_bytes();
pub fn write_evt_wl_seat_capabilities(dst: &mut &mut [u8], for_id: ObjId, capabilities: u32) {
    let l = length_evt_wl_seat_capabilities();
    write_header(dst, for_id, l, 0, 0);
    write_u32(dst, capabilities);
}
pub fn length_evt_wl_seat_capabilities() -> usize {
    12
}
pub fn parse_evt_wl_seat_capabilities<'a>(mut msg: &'a [u8]) -> Result<u32, &'static str> {
    msg = msg.get(8..).ok_or(PARSE_ERROR)?;
    let arg1 = parse_u32(&mut msg)?;
    if !msg.is_empty() {
        return Err(PARSE_ERROR);
    }
    Ok(arg1)
}
pub const OPCODE_WL_SEAT_CAPABILITIES: MethodId = MethodId::Event(0);
pub fn write_req_wl_seat_get_keyboard(dst: &mut &mut [u8], for_id: ObjId, id: ObjId) {
    let l = length_req_wl_seat_get_keyboard();
    write_header(dst, for_id, l, 1, 0);
    write_obj(dst, id);
}
pub fn length_req_wl_seat_get_keyboard() -> usize {
    12
}
pub fn parse_req_wl_seat_get_keyboard<'a>(mut msg: &'a [u8]) -> Result<ObjId, &'static str> {
    msg = msg.get(8..).ok_or(PARSE_ERROR)?;
    let arg1 = parse_obj(&mut msg)?;
    if !msg.is_empty() {
        return Err(PARSE_ERROR);
    }
    Ok(arg1)
}
pub const OPCODE_WL_SEAT_GET_KEYBOARD: MethodId = MethodId::Request(1);
const DATA_WL_SEAT: WaylandData = WaylandData {
    name: "wl_seat",
    evts: &[
        WaylandMethod {
            name: "capabilities",
            sig: &[Uint],
            destructor: false,
        },
        WaylandMethod {
            name: "name",
            sig: &[String],
            destructor: false,
        },
    ],
    reqs: &[
        WaylandMethod {
            name: "get_pointer",
            sig: &[NewId(WlPointer)],
            destructor: false,
        },
        WaylandMethod {
            name: "get_keyboard",
            sig: &[NewId(WlKeyboard)],
            destructor: false,
        },
        WaylandMethod {
            name: "get_touch",
            sig: &[NewId(WlTouch)],
            destructor: false,
        },
        WaylandMethod {
            name: "release",
            sig: &[],
            destructor: true,
        },
    ],
    version: 10,
};
pub const WL_SEAT: &[u8] = DATA_WL_SEAT.name.as_bytes();
const DATA_WL_POINTER: WaylandData = WaylandData {
    name: "wl_pointer",
    evts: &[
        WaylandMethod {
            name: "enter",
            sig: &[Uint, Object(WlSurface), Fixed, Fixed],
            destructor: false,
        },
        WaylandMethod {
            name: "leave",
            sig: &[Uint, Object(WlSurface)],
            destructor: false,
        },
        WaylandMethod {
            name: "motion",
            sig: &[Uint, Fixed, Fixed],
            destructor: false,
        },
        WaylandMethod {
            name: "button",
            sig: &[Uint, Uint, Uint, Uint],
            destructor: false,
        },
        WaylandMethod {
            name: "axis",
            sig: &[Uint, Uint, Fixed],
            destructor: false,
        },
        WaylandMethod {
            name: "frame",
            sig: &[],
            destructor: false,
        },
        WaylandMethod {
            name: "axis_source",
            sig: &[Uint],
            destructor: false,
        },
        WaylandMethod {
            name: "axis_stop",
            sig: &[Uint, Uint],
            destructor: false,
        },
        WaylandMethod {
            name: "axis_discrete",
            sig: &[Uint, Int],
            destructor: false,
        },
        WaylandMethod {
            name: "axis_value120",
            sig: &[Uint, Int],
            destructor: false,
        },
        WaylandMethod {
            name: "axis_relative_direction",
            sig: &[Uint, Uint],
            destructor: false,
        },
    ],
    reqs: &[
        WaylandMethod {
            name: "set_cursor",
            sig: &[Uint, Object(WlSurface), Int, Int],
            destructor: false,
        },
        WaylandMethod {
            name: "release",
            sig: &[],
            destructor: true,
        },
    ],
    version: 10,
};
pub const WL_POINTER: &[u8] = DATA_WL_POINTER.name.as_bytes();
pub fn write_evt_wl_keyboard_keymap(
    dst: &mut &mut [u8],
    for_id: ObjId,
    tag_fds: bool,
    format: u32,
    size: u32,
) {
    let l = length_evt_wl_keyboard_keymap();
    write_header(dst, for_id, l, 0, if tag_fds { 1 } else { 0 });
    write_u32(dst, format);
    write_u32(dst, size);
}
pub fn length_evt_wl_keyboard_keymap() -> usize {
    16
}
pub fn parse_evt_wl_keyboard_keymap<'a>(mut msg: &'a [u8]) -> Result<(u32, u32), &'static str> {
    msg = msg.get(8..).ok_or(PARSE_ERROR)?;
    let arg1 = parse_u32(&mut msg)?;
    let arg3 = parse_u32(&mut msg)?;
    if !msg.is_empty() {
        return Err(PARSE_ERROR);
    }
    Ok((arg1, arg3))
}
pub const OPCODE_WL_KEYBOARD_KEYMAP: MethodId = MethodId::Event(0);
const DATA_WL_KEYBOARD: WaylandData = WaylandData {
    name: "wl_keyboard",
    evts: &[
        WaylandMethod {
            name: "keymap",
            sig: &[Uint, Fd, Uint],
            destructor: false,
        },
        WaylandMethod {
            name: "enter",
            sig: &[Uint, Object(WlSurface), Array],
            destructor: false,
        },
        WaylandMethod {
            name: "leave",
            sig: &[Uint, Object(WlSurface)],
            destructor: false,
        },
        WaylandMethod {
            name: "key",
            sig: &[Uint, Uint, Uint, Uint],
            destructor: false,
        },
        WaylandMethod {
            name: "modifiers",
            sig: &[Uint, Uint, Uint, Uint, Uint],
            destructor: false,
        },
        WaylandMethod {
            name: "repeat_info",
            sig: &[Int, Int],
            destructor: false,
        },
    ],
    reqs: &[WaylandMethod {
        name: "release",
        sig: &[],
        destructor: true,
    }],
    version: 10,
};
pub const WL_KEYBOARD: &[u8] = DATA_WL_KEYBOARD.name.as_bytes();
const DATA_WL_TOUCH: WaylandData = WaylandData {
    name: "wl_touch",
    evts: &[
        WaylandMethod {
            name: "down",
            sig: &[Uint, Uint, Object(WlSurface), Int, Fixed, Fixed],
            destructor: false,
        },
        WaylandMethod {
            name: "up",
            sig: &[Uint, Uint, Int],
            destructor: false,
        },
        WaylandMethod {
            name: "motion",
            sig: &[Uint, Int, Fixed, Fixed],
            destructor: false,
        },
        WaylandMethod {
            name: "frame",
            sig: &[],
            destructor: false,
        },
        WaylandMethod {
            name: "cancel",
            sig: &[],
            destructor: false,
        },
        WaylandMethod {
            name: "shape",
            sig: &[Int, Fixed, Fixed],
            destructor: false,
        },
        WaylandMethod {
            name: "orientation",
            sig: &[Int, Fixed],
            destructor: false,
        },
    ],
    reqs: &[WaylandMethod {
        name: "release",
        sig: &[],
        destructor: true,
    }],
    version: 10,
};
pub const WL_TOUCH: &[u8] = DATA_WL_TOUCH.name.as_bytes();
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WlOutputTransform {
    Normal = 0,
    Item90 = 1,
    Item180 = 2,
    Item270 = 3,
    Flipped = 4,
    Flipped90 = 5,
    Flipped180 = 6,
    Flipped270 = 7,
}
impl TryFrom<u32> for WlOutputTransform {
    type Error = ();
    fn try_from(v: u32) -> Result<Self, Self::Error> {
        Ok(match v {
            0 => WlOutputTransform::Normal,
            1 => WlOutputTransform::Item90,
            2 => WlOutputTransform::Item180,
            3 => WlOutputTransform::Item270,
            4 => WlOutputTransform::Flipped,
            5 => WlOutputTransform::Flipped90,
            6 => WlOutputTransform::Flipped180,
            7 => WlOutputTransform::Flipped270,
            _ => return Err(()),
        })
    }
}
const DATA_WL_OUTPUT: WaylandData = WaylandData {
    name: "wl_output",
    evts: &[
        WaylandMethod {
            name: "geometry",
            sig: &[Int, Int, Int, Int, Int, String, String, Int],
            destructor: false,
        },
        WaylandMethod {
            name: "mode",
            sig: &[Uint, Int, Int, Int],
            destructor: false,
        },
        WaylandMethod {
            name: "done",
            sig: &[],
            destructor: false,
        },
        WaylandMethod {
            name: "scale",
            sig: &[Int],
            destructor: false,
        },
        WaylandMethod {
            name: "name",
            sig: &[String],
            destructor: false,
        },
        WaylandMethod {
            name: "description",
            sig: &[String],
            destructor: false,
        },
    ],
    reqs: &[WaylandMethod {
        name: "release",
        sig: &[],
        destructor: true,
    }],
    version: 4,
};
pub const WL_OUTPUT: &[u8] = DATA_WL_OUTPUT.name.as_bytes();
const DATA_WL_REGION: WaylandData = WaylandData {
    name: "wl_region",
    evts: &[],
    reqs: &[
        WaylandMethod {
            name: "destroy",
            sig: &[],
            destructor: true,
        },
        WaylandMethod {
            name: "add",
            sig: &[Int, Int, Int, Int],
            destructor: false,
        },
        WaylandMethod {
            name: "subtract",
            sig: &[Int, Int, Int, Int],
            destructor: false,
        },
    ],
    version: 1,
};
pub const WL_REGION: &[u8] = DATA_WL_REGION.name.as_bytes();
const DATA_WL_SUBCOMPOSITOR: WaylandData = WaylandData {
    name: "wl_subcompositor",
    evts: &[],
    reqs: &[
        WaylandMethod {
            name: "destroy",
            sig: &[],
            destructor: true,
        },
        WaylandMethod {
            name: "get_subsurface",
            sig: &[NewId(WlSubsurface), Object(WlSurface), Object(WlSurface)],
            destructor: false,
        },
    ],
    version: 1,
};
pub const WL_SUBCOMPOSITOR: &[u8] = DATA_WL_SUBCOMPOSITOR.name.as_bytes();
const DATA_WL_SUBSURFACE: WaylandData = WaylandData {
    name: "wl_subsurface",
    evts: &[],
    reqs: &[
        WaylandMethod {
            name: "destroy",
            sig: &[],
            destructor: true,
        },
        WaylandMethod {
            name: "set_position",
            sig: &[Int, Int],
            destructor: false,
        },
        WaylandMethod {
            name: "place_above",
            sig: &[Object(WlSurface)],
            destructor: false,
        },
        WaylandMethod {
            name: "place_below",
            sig: &[Object(WlSurface)],
            destructor: false,
        },
        WaylandMethod {
            name: "set_sync",
            sig: &[],
            destructor: false,
        },
        WaylandMethod {
            name: "set_desync",
            sig: &[],
            destructor: false,
        },
    ],
    version: 1,
};
pub const WL_SUBSURFACE: &[u8] = DATA_WL_SUBSURFACE.name.as_bytes();
pub fn write_req_wl_fixes_destroy(dst: &mut &mut [u8], for_id: ObjId) {
    let l = length_req_wl_fixes_destroy();
    write_header(dst, for_id, l, 0, 0);
}
pub fn length_req_wl_fixes_destroy() -> usize {
    8
}
pub fn parse_req_wl_fixes_destroy<'a>(msg: &'a [u8]) -> Result<(), &'static str> {
    if msg.len() != 8 {
        return Err(PARSE_ERROR);
    }
    Ok(())
}
pub const OPCODE_WL_FIXES_DESTROY: MethodId = MethodId::Request(0);
pub fn write_req_wl_fixes_destroy_registry(dst: &mut &mut [u8], for_id: ObjId, registry: ObjId) {
    let l = length_req_wl_fixes_destroy_registry();
    write_header(dst, for_id, l, 1, 0);
    write_obj(dst, registry);
}
pub fn length_req_wl_fixes_destroy_registry() -> usize {
    12
}
pub fn parse_req_wl_fixes_destroy_registry<'a>(mut msg: &'a [u8]) -> Result<ObjId, &'static str> {
    msg = msg.get(8..).ok_or(PARSE_ERROR)?;
    let arg1 = parse_obj(&mut msg)?;
    if !msg.is_empty() {
        return Err(PARSE_ERROR);
    }
    Ok(arg1)
}
pub const OPCODE_WL_FIXES_DESTROY_REGISTRY: MethodId = MethodId::Request(1);
const DATA_WL_FIXES: WaylandData = WaylandData {
    name: "wl_fixes",
    evts: &[],
    reqs: &[
        WaylandMethod {
            name: "destroy",
            sig: &[],
            destructor: true,
        },
        WaylandMethod {
            name: "destroy_registry",
            sig: &[Object(WlRegistry)],
            destructor: false,
        },
    ],
    version: 1,
};
pub const WL_FIXES: &[u8] = DATA_WL_FIXES.name.as_bytes();
pub fn write_req_zwlr_data_control_manager_v1_create_data_source(
    dst: &mut &mut [u8],
    for_id: ObjId,
    id: ObjId,
) {
    let l = length_req_zwlr_data_control_manager_v1_create_data_source();
    write_header(dst, for_id, l, 0, 0);
    write_obj(dst, id);
}
pub fn length_req_zwlr_data_control_manager_v1_create_data_source() -> usize {
    12
}
pub fn parse_req_zwlr_data_control_manager_v1_create_data_source<'a>(
    mut msg: &'a [u8],
) -> Result<ObjId, &'static str> {
    msg = msg.get(8..).ok_or(PARSE_ERROR)?;
    let arg1 = parse_obj(&mut msg)?;
    if !msg.is_empty() {
        return Err(PARSE_ERROR);
    }
    Ok(arg1)
}
pub const OPCODE_ZWLR_DATA_CONTROL_MANAGER_V1_CREATE_DATA_SOURCE: MethodId = MethodId::Request(0);
pub fn write_req_zwlr_data_control_manager_v1_get_data_device(
    dst: &mut &mut [u8],
    for_id: ObjId,
    id: ObjId,
    seat: ObjId,
) {
    let l = length_req_zwlr_data_control_manager_v1_get_data_device();
    write_header(dst, for_id, l, 1, 0);
    write_obj(dst, id);
    write_obj(dst, seat);
}
pub fn length_req_zwlr_data_control_manager_v1_get_data_device() -> usize {
    16
}
pub fn parse_req_zwlr_data_control_manager_v1_get_data_device<'a>(
    mut msg: &'a [u8],
) -> Result<(ObjId, ObjId), &'static str> {
    msg = msg.get(8..).ok_or(PARSE_ERROR)?;
    let arg1 = parse_obj(&mut msg)?;
    let arg2 = parse_obj(&mut msg)?;
    if !msg.is_empty() {
        return Err(PARSE_ERROR);
    }
    Ok((arg1, arg2))
}
pub const OPCODE_ZWLR_DATA_CONTROL_MANAGER_V1_GET_DATA_DEVICE: MethodId = MethodId::Request(1);
const DATA_ZWLR_DATA_CONTROL_MANAGER_V1: WaylandData = WaylandData {
    name: "zwlr_data_control_manager_v1",
    evts: &[],
    reqs: &[
        WaylandMethod {
            name: "create_data_source",
            sig: &[NewId(ZwlrDataControlSourceV1)],
            destructor: false,
        },
        WaylandMethod {
            name: "get_data_device",
            sig: &[NewId(ZwlrDataControlDeviceV1), Object(WlSeat)],
            destructor: false,
        },
        WaylandMethod {
            name: "destroy",
            sig: &[],
            destructor: true,
        },
    ],
    version: 2,
};
pub const ZWLR_DATA_CONTROL_MANAGER_V1: &[u8] = DATA_ZWLR_DATA_CONTROL_MANAGER_V1.name.as_bytes();
pub fn write_req_zwlr_data_control_device_v1_set_selection(
    dst: &mut &mut [u8],
    for_id: ObjId,
    source: ObjId,
) {
    let l = length_req_zwlr_data_control_device_v1_set_selection();
    write_header(dst, for_id, l, 0, 0);
    write_obj(dst, source);
}
pub fn length_req_zwlr_data_control_device_v1_set_selection() -> usize {
    12
}
pub fn parse_req_zwlr_data_control_device_v1_set_selection<'a>(
    mut msg: &'a [u8],
) -> Result<ObjId, &'static str> {
    msg = msg.get(8..).ok_or(PARSE_ERROR)?;
    let arg1 = parse_obj(&mut msg)?;
    if !msg.is_empty() {
        return Err(PARSE_ERROR);
    }
    Ok(arg1)
}
pub const OPCODE_ZWLR_DATA_CONTROL_DEVICE_V1_SET_SELECTION: MethodId = MethodId::Request(0);
pub fn write_evt_zwlr_data_control_device_v1_data_offer(
    dst: &mut &mut [u8],
    for_id: ObjId,
    id: ObjId,
) {
    let l = length_evt_zwlr_data_control_device_v1_data_offer();
    write_header(dst, for_id, l, 0, 0);
    write_obj(dst, id);
}
pub fn length_evt_zwlr_data_control_device_v1_data_offer() -> usize {
    12
}
pub fn parse_evt_zwlr_data_control_device_v1_data_offer<'a>(
    mut msg: &'a [u8],
) -> Result<ObjId, &'static str> {
    msg = msg.get(8..).ok_or(PARSE_ERROR)?;
    let arg1 = parse_obj(&mut msg)?;
    if !msg.is_empty() {
        return Err(PARSE_ERROR);
    }
    Ok(arg1)
}
pub const OPCODE_ZWLR_DATA_CONTROL_DEVICE_V1_DATA_OFFER: MethodId = MethodId::Event(0);
pub fn write_evt_zwlr_data_control_device_v1_selection(
    dst: &mut &mut [u8],
    for_id: ObjId,
    id: ObjId,
) {
    let l = length_evt_zwlr_data_control_device_v1_selection();
    write_header(dst, for_id, l, 1, 0);
    write_obj(dst, id);
}
pub fn length_evt_zwlr_data_control_device_v1_selection() -> usize {
    12
}
pub fn parse_evt_zwlr_data_control_device_v1_selection<'a>(
    mut msg: &'a [u8],
) -> Result<ObjId, &'static str> {
    msg = msg.get(8..).ok_or(PARSE_ERROR)?;
    let arg1 = parse_obj(&mut msg)?;
    if !msg.is_empty() {
        return Err(PARSE_ERROR);
    }
    Ok(arg1)
}
pub const OPCODE_ZWLR_DATA_CONTROL_DEVICE_V1_SELECTION: MethodId = MethodId::Event(1);
const DATA_ZWLR_DATA_CONTROL_DEVICE_V1: WaylandData = WaylandData {
    name: "zwlr_data_control_device_v1",
    evts: &[
        WaylandMethod {
            name: "data_offer",
            sig: &[NewId(ZwlrDataControlOfferV1)],
            destructor: false,
        },
        WaylandMethod {
            name: "selection",
            sig: &[Object(ZwlrDataControlOfferV1)],
            destructor: false,
        },
        WaylandMethod {
            name: "finished",
            sig: &[],
            destructor: false,
        },
        WaylandMethod {
            name: "primary_selection",
            sig: &[Object(ZwlrDataControlOfferV1)],
            destructor: false,
        },
    ],
    reqs: &[
        WaylandMethod {
            name: "set_selection",
            sig: &[Object(ZwlrDataControlSourceV1)],
            destructor: false,
        },
        WaylandMethod {
            name: "destroy",
            sig: &[],
            destructor: true,
        },
        WaylandMethod {
            name: "set_primary_selection",
            sig: &[Object(ZwlrDataControlSourceV1)],
            destructor: false,
        },
    ],
    version: 2,
};
pub const ZWLR_DATA_CONTROL_DEVICE_V1: &[u8] = DATA_ZWLR_DATA_CONTROL_DEVICE_V1.name.as_bytes();
pub fn write_req_zwlr_data_control_source_v1_offer(
    dst: &mut &mut [u8],
    for_id: ObjId,
    mime_type: &[u8],
) {
    let l = length_req_zwlr_data_control_source_v1_offer(mime_type.len());
    write_header(dst, for_id, l, 0, 0);
    write_string(dst, Some(mime_type));
}
pub fn length_req_zwlr_data_control_source_v1_offer(mime_type_len: usize) -> usize {
    let mut v = 8;
    v += length_string(mime_type_len);
    v
}
pub fn parse_req_zwlr_data_control_source_v1_offer<'a>(
    mut msg: &'a [u8],
) -> Result<&'a [u8], &'static str> {
    msg = msg.get(8..).ok_or(PARSE_ERROR)?;
    let arg1 = parse_string(&mut msg)?.ok_or(PARSE_ERROR)?;
    if !msg.is_empty() {
        return Err(PARSE_ERROR);
    }
    Ok(arg1)
}
pub const OPCODE_ZWLR_DATA_CONTROL_SOURCE_V1_OFFER: MethodId = MethodId::Request(0);
pub fn write_evt_zwlr_data_control_source_v1_send(
    dst: &mut &mut [u8],
    for_id: ObjId,
    tag_fds: bool,
    mime_type: &[u8],
) {
    let l = length_evt_zwlr_data_control_source_v1_send(mime_type.len());
    write_header(dst, for_id, l, 0, if tag_fds { 1 } else { 0 });
    write_string(dst, Some(mime_type));
}
pub fn length_evt_zwlr_data_control_source_v1_send(mime_type_len: usize) -> usize {
    let mut v = 8;
    v += length_string(mime_type_len);
    v
}
pub fn parse_evt_zwlr_data_control_source_v1_send<'a>(
    mut msg: &'a [u8],
) -> Result<&'a [u8], &'static str> {
    msg = msg.get(8..).ok_or(PARSE_ERROR)?;
    let arg1 = parse_string(&mut msg)?.ok_or(PARSE_ERROR)?;
    if !msg.is_empty() {
        return Err(PARSE_ERROR);
    }
    Ok(arg1)
}
pub const OPCODE_ZWLR_DATA_CONTROL_SOURCE_V1_SEND: MethodId = MethodId::Event(0);
const DATA_ZWLR_DATA_CONTROL_SOURCE_V1: WaylandData = WaylandData {
    name: "zwlr_data_control_source_v1",
    evts: &[
        WaylandMethod {
            name: "send",
            sig: &[String, Fd],
            destructor: false,
        },
        WaylandMethod {
            name: "cancelled",
            sig: &[],
            destructor: false,
        },
    ],
    reqs: &[
        WaylandMethod {
            name: "offer",
            sig: &[String],
            destructor: false,
        },
        WaylandMethod {
            name: "destroy",
            sig: &[],
            destructor: true,
        },
    ],
    version: 1,
};
pub const ZWLR_DATA_CONTROL_SOURCE_V1: &[u8] = DATA_ZWLR_DATA_CONTROL_SOURCE_V1.name.as_bytes();
pub fn write_req_zwlr_data_control_offer_v1_receive(
    dst: &mut &mut [u8],
    for_id: ObjId,
    tag_fds: bool,
    mime_type: &[u8],
) {
    let l = length_req_zwlr_data_control_offer_v1_receive(mime_type.len());
    write_header(dst, for_id, l, 0, if tag_fds { 1 } else { 0 });
    write_string(dst, Some(mime_type));
}
pub fn length_req_zwlr_data_control_offer_v1_receive(mime_type_len: usize) -> usize {
    let mut v = 8;
    v += length_string(mime_type_len);
    v
}
pub fn parse_req_zwlr_data_control_offer_v1_receive<'a>(
    mut msg: &'a [u8],
) -> Result<&'a [u8], &'static str> {
    msg = msg.get(8..).ok_or(PARSE_ERROR)?;
    let arg1 = parse_string(&mut msg)?.ok_or(PARSE_ERROR)?;
    if !msg.is_empty() {
        return Err(PARSE_ERROR);
    }
    Ok(arg1)
}
pub const OPCODE_ZWLR_DATA_CONTROL_OFFER_V1_RECEIVE: MethodId = MethodId::Request(0);
pub fn write_evt_zwlr_data_control_offer_v1_offer(
    dst: &mut &mut [u8],
    for_id: ObjId,
    mime_type: &[u8],
) {
    let l = length_evt_zwlr_data_control_offer_v1_offer(mime_type.len());
    write_header(dst, for_id, l, 0, 0);
    write_string(dst, Some(mime_type));
}
pub fn length_evt_zwlr_data_control_offer_v1_offer(mime_type_len: usize) -> usize {
    let mut v = 8;
    v += length_string(mime_type_len);
    v
}
pub fn parse_evt_zwlr_data_control_offer_v1_offer<'a>(
    mut msg: &'a [u8],
) -> Result<&'a [u8], &'static str> {
    msg = msg.get(8..).ok_or(PARSE_ERROR)?;
    let arg1 = parse_string(&mut msg)?.ok_or(PARSE_ERROR)?;
    if !msg.is_empty() {
        return Err(PARSE_ERROR);
    }
    Ok(arg1)
}
pub const OPCODE_ZWLR_DATA_CONTROL_OFFER_V1_OFFER: MethodId = MethodId::Event(0);
const DATA_ZWLR_DATA_CONTROL_OFFER_V1: WaylandData = WaylandData {
    name: "zwlr_data_control_offer_v1",
    evts: &[WaylandMethod {
        name: "offer",
        sig: &[String],
        destructor: false,
    }],
    reqs: &[
        WaylandMethod {
            name: "receive",
            sig: &[String, Fd],
            destructor: false,
        },
        WaylandMethod {
            name: "destroy",
            sig: &[],
            destructor: true,
        },
    ],
    version: 1,
};
pub const ZWLR_DATA_CONTROL_OFFER_V1: &[u8] = DATA_ZWLR_DATA_CONTROL_OFFER_V1.name.as_bytes();
const DATA_ZWLR_EXPORT_DMABUF_MANAGER_V1: WaylandData = WaylandData {
    name: "zwlr_export_dmabuf_manager_v1",
    evts: &[],
    reqs: &[
        WaylandMethod {
            name: "capture_output",
            sig: &[NewId(ZwlrExportDmabufFrameV1), Int, Object(WlOutput)],
            destructor: false,
        },
        WaylandMethod {
            name: "destroy",
            sig: &[],
            destructor: true,
        },
    ],
    version: 1,
};
pub const ZWLR_EXPORT_DMABUF_MANAGER_V1: &[u8] = DATA_ZWLR_EXPORT_DMABUF_MANAGER_V1.name.as_bytes();
const DATA_ZWLR_EXPORT_DMABUF_FRAME_V1: WaylandData = WaylandData {
    name: "zwlr_export_dmabuf_frame_v1",
    evts: &[
        WaylandMethod {
            name: "frame",
            sig: &[Uint, Uint, Uint, Uint, Uint, Uint, Uint, Uint, Uint, Uint],
            destructor: false,
        },
        WaylandMethod {
            name: "object",
            sig: &[Uint, Fd, Uint, Uint, Uint, Uint],
            destructor: false,
        },
        WaylandMethod {
            name: "ready",
            sig: &[Uint, Uint, Uint],
            destructor: false,
        },
        WaylandMethod {
            name: "cancel",
            sig: &[Uint],
            destructor: false,
        },
    ],
    reqs: &[WaylandMethod {
        name: "destroy",
        sig: &[],
        destructor: true,
    }],
    version: 1,
};
pub const ZWLR_EXPORT_DMABUF_FRAME_V1: &[u8] = DATA_ZWLR_EXPORT_DMABUF_FRAME_V1.name.as_bytes();
pub fn write_req_zwlr_gamma_control_manager_v1_get_gamma_control(
    dst: &mut &mut [u8],
    for_id: ObjId,
    id: ObjId,
    output: ObjId,
) {
    let l = length_req_zwlr_gamma_control_manager_v1_get_gamma_control();
    write_header(dst, for_id, l, 0, 0);
    write_obj(dst, id);
    write_obj(dst, output);
}
pub fn length_req_zwlr_gamma_control_manager_v1_get_gamma_control() -> usize {
    16
}
pub fn parse_req_zwlr_gamma_control_manager_v1_get_gamma_control<'a>(
    mut msg: &'a [u8],
) -> Result<(ObjId, ObjId), &'static str> {
    msg = msg.get(8..).ok_or(PARSE_ERROR)?;
    let arg1 = parse_obj(&mut msg)?;
    let arg2 = parse_obj(&mut msg)?;
    if !msg.is_empty() {
        return Err(PARSE_ERROR);
    }
    Ok((arg1, arg2))
}
pub const OPCODE_ZWLR_GAMMA_CONTROL_MANAGER_V1_GET_GAMMA_CONTROL: MethodId = MethodId::Request(0);
const DATA_ZWLR_GAMMA_CONTROL_MANAGER_V1: WaylandData = WaylandData {
    name: "zwlr_gamma_control_manager_v1",
    evts: &[],
    reqs: &[
        WaylandMethod {
            name: "get_gamma_control",
            sig: &[NewId(ZwlrGammaControlV1), Object(WlOutput)],
            destructor: false,
        },
        WaylandMethod {
            name: "destroy",
            sig: &[],
            destructor: true,
        },
    ],
    version: 1,
};
pub const ZWLR_GAMMA_CONTROL_MANAGER_V1: &[u8] = DATA_ZWLR_GAMMA_CONTROL_MANAGER_V1.name.as_bytes();
pub fn write_evt_zwlr_gamma_control_v1_gamma_size(dst: &mut &mut [u8], for_id: ObjId, size: u32) {
    let l = length_evt_zwlr_gamma_control_v1_gamma_size();
    write_header(dst, for_id, l, 0, 0);
    write_u32(dst, size);
}
pub fn length_evt_zwlr_gamma_control_v1_gamma_size() -> usize {
    12
}
pub fn parse_evt_zwlr_gamma_control_v1_gamma_size<'a>(
    mut msg: &'a [u8],
) -> Result<u32, &'static str> {
    msg = msg.get(8..).ok_or(PARSE_ERROR)?;
    let arg1 = parse_u32(&mut msg)?;
    if !msg.is_empty() {
        return Err(PARSE_ERROR);
    }
    Ok(arg1)
}
pub const OPCODE_ZWLR_GAMMA_CONTROL_V1_GAMMA_SIZE: MethodId = MethodId::Event(0);
pub fn write_req_zwlr_gamma_control_v1_set_gamma(
    dst: &mut &mut [u8],
    for_id: ObjId,
    tag_fds: bool,
) {
    let l = length_req_zwlr_gamma_control_v1_set_gamma();
    write_header(dst, for_id, l, 0, if tag_fds { 1 } else { 0 });
}
pub fn length_req_zwlr_gamma_control_v1_set_gamma() -> usize {
    8
}
pub fn parse_req_zwlr_gamma_control_v1_set_gamma<'a>(msg: &'a [u8]) -> Result<(), &'static str> {
    if msg.len() != 8 {
        return Err(PARSE_ERROR);
    }
    Ok(())
}
pub const OPCODE_ZWLR_GAMMA_CONTROL_V1_SET_GAMMA: MethodId = MethodId::Request(0);
const DATA_ZWLR_GAMMA_CONTROL_V1: WaylandData = WaylandData {
    name: "zwlr_gamma_control_v1",
    evts: &[
        WaylandMethod {
            name: "gamma_size",
            sig: &[Uint],
            destructor: false,
        },
        WaylandMethod {
            name: "failed",
            sig: &[],
            destructor: false,
        },
    ],
    reqs: &[
        WaylandMethod {
            name: "set_gamma",
            sig: &[Fd],
            destructor: false,
        },
        WaylandMethod {
            name: "destroy",
            sig: &[],
            destructor: true,
        },
    ],
    version: 1,
};
pub const ZWLR_GAMMA_CONTROL_V1: &[u8] = DATA_ZWLR_GAMMA_CONTROL_V1.name.as_bytes();
pub fn write_req_zwlr_screencopy_manager_v1_capture_output(
    dst: &mut &mut [u8],
    for_id: ObjId,
    frame: ObjId,
    overlay_cursor: i32,
    output: ObjId,
) {
    let l = length_req_zwlr_screencopy_manager_v1_capture_output();
    write_header(dst, for_id, l, 0, 0);
    write_obj(dst, frame);
    write_i32(dst, overlay_cursor);
    write_obj(dst, output);
}
pub fn length_req_zwlr_screencopy_manager_v1_capture_output() -> usize {
    20
}
pub fn parse_req_zwlr_screencopy_manager_v1_capture_output<'a>(
    mut msg: &'a [u8],
) -> Result<(ObjId, i32, ObjId), &'static str> {
    msg = msg.get(8..).ok_or(PARSE_ERROR)?;
    let arg1 = parse_obj(&mut msg)?;
    let arg2 = parse_i32(&mut msg)?;
    let arg3 = parse_obj(&mut msg)?;
    if !msg.is_empty() {
        return Err(PARSE_ERROR);
    }
    Ok((arg1, arg2, arg3))
}
pub const OPCODE_ZWLR_SCREENCOPY_MANAGER_V1_CAPTURE_OUTPUT: MethodId = MethodId::Request(0);
pub fn write_req_zwlr_screencopy_manager_v1_capture_output_region(
    dst: &mut &mut [u8],
    for_id: ObjId,
    frame: ObjId,
    overlay_cursor: i32,
    output: ObjId,
    x: i32,
    y: i32,
    width: i32,
    height: i32,
) {
    let l = length_req_zwlr_screencopy_manager_v1_capture_output_region();
    write_header(dst, for_id, l, 1, 0);
    write_obj(dst, frame);
    write_i32(dst, overlay_cursor);
    write_obj(dst, output);
    write_i32(dst, x);
    write_i32(dst, y);
    write_i32(dst, width);
    write_i32(dst, height);
}
pub fn length_req_zwlr_screencopy_manager_v1_capture_output_region() -> usize {
    36
}
pub fn parse_req_zwlr_screencopy_manager_v1_capture_output_region<'a>(
    mut msg: &'a [u8],
) -> Result<(ObjId, i32, ObjId, i32, i32, i32, i32), &'static str> {
    msg = msg.get(8..).ok_or(PARSE_ERROR)?;
    let arg1 = parse_obj(&mut msg)?;
    let arg2 = parse_i32(&mut msg)?;
    let arg3 = parse_obj(&mut msg)?;
    let arg4 = parse_i32(&mut msg)?;
    let arg5 = parse_i32(&mut msg)?;
    let arg6 = parse_i32(&mut msg)?;
    let arg7 = parse_i32(&mut msg)?;
    if !msg.is_empty() {
        return Err(PARSE_ERROR);
    }
    Ok((arg1, arg2, arg3, arg4, arg5, arg6, arg7))
}
pub const OPCODE_ZWLR_SCREENCOPY_MANAGER_V1_CAPTURE_OUTPUT_REGION: MethodId = MethodId::Request(1);
const DATA_ZWLR_SCREENCOPY_MANAGER_V1: WaylandData = WaylandData {
    name: "zwlr_screencopy_manager_v1",
    evts: &[],
    reqs: &[
        WaylandMethod {
            name: "capture_output",
            sig: &[NewId(ZwlrScreencopyFrameV1), Int, Object(WlOutput)],
            destructor: false,
        },
        WaylandMethod {
            name: "capture_output_region",
            sig: &[
                NewId(ZwlrScreencopyFrameV1),
                Int,
                Object(WlOutput),
                Int,
                Int,
                Int,
                Int,
            ],
            destructor: false,
        },
        WaylandMethod {
            name: "destroy",
            sig: &[],
            destructor: true,
        },
    ],
    version: 3,
};
pub const ZWLR_SCREENCOPY_MANAGER_V1: &[u8] = DATA_ZWLR_SCREENCOPY_MANAGER_V1.name.as_bytes();
pub fn write_evt_zwlr_screencopy_frame_v1_buffer(
    dst: &mut &mut [u8],
    for_id: ObjId,
    format: u32,
    width: u32,
    height: u32,
    stride: u32,
) {
    let l = length_evt_zwlr_screencopy_frame_v1_buffer();
    write_header(dst, for_id, l, 0, 0);
    write_u32(dst, format);
    write_u32(dst, width);
    write_u32(dst, height);
    write_u32(dst, stride);
}
pub fn length_evt_zwlr_screencopy_frame_v1_buffer() -> usize {
    24
}
pub fn parse_evt_zwlr_screencopy_frame_v1_buffer<'a>(
    mut msg: &'a [u8],
) -> Result<(u32, u32, u32, u32), &'static str> {
    msg = msg.get(8..).ok_or(PARSE_ERROR)?;
    let arg1 = parse_u32(&mut msg)?;
    let arg2 = parse_u32(&mut msg)?;
    let arg3 = parse_u32(&mut msg)?;
    let arg4 = parse_u32(&mut msg)?;
    if !msg.is_empty() {
        return Err(PARSE_ERROR);
    }
    Ok((arg1, arg2, arg3, arg4))
}
pub const OPCODE_ZWLR_SCREENCOPY_FRAME_V1_BUFFER: MethodId = MethodId::Event(0);
pub fn write_req_zwlr_screencopy_frame_v1_copy(dst: &mut &mut [u8], for_id: ObjId, buffer: ObjId) {
    let l = length_req_zwlr_screencopy_frame_v1_copy();
    write_header(dst, for_id, l, 0, 0);
    write_obj(dst, buffer);
}
pub fn length_req_zwlr_screencopy_frame_v1_copy() -> usize {
    12
}
pub fn parse_req_zwlr_screencopy_frame_v1_copy<'a>(
    mut msg: &'a [u8],
) -> Result<ObjId, &'static str> {
    msg = msg.get(8..).ok_or(PARSE_ERROR)?;
    let arg1 = parse_obj(&mut msg)?;
    if !msg.is_empty() {
        return Err(PARSE_ERROR);
    }
    Ok(arg1)
}
pub const OPCODE_ZWLR_SCREENCOPY_FRAME_V1_COPY: MethodId = MethodId::Request(0);
pub fn write_evt_zwlr_screencopy_frame_v1_ready(
    dst: &mut &mut [u8],
    for_id: ObjId,
    tv_sec_hi: u32,
    tv_sec_lo: u32,
    tv_nsec: u32,
) {
    let l = length_evt_zwlr_screencopy_frame_v1_ready();
    write_header(dst, for_id, l, 2, 0);
    write_u32(dst, tv_sec_hi);
    write_u32(dst, tv_sec_lo);
    write_u32(dst, tv_nsec);
}
pub fn length_evt_zwlr_screencopy_frame_v1_ready() -> usize {
    20
}
pub fn parse_evt_zwlr_screencopy_frame_v1_ready<'a>(
    mut msg: &'a [u8],
) -> Result<(u32, u32, u32), &'static str> {
    msg = msg.get(8..).ok_or(PARSE_ERROR)?;
    let arg1 = parse_u32(&mut msg)?;
    let arg2 = parse_u32(&mut msg)?;
    let arg3 = parse_u32(&mut msg)?;
    if !msg.is_empty() {
        return Err(PARSE_ERROR);
    }
    Ok((arg1, arg2, arg3))
}
pub const OPCODE_ZWLR_SCREENCOPY_FRAME_V1_READY: MethodId = MethodId::Event(2);
pub fn write_evt_zwlr_screencopy_frame_v1_failed(dst: &mut &mut [u8], for_id: ObjId) {
    let l = length_evt_zwlr_screencopy_frame_v1_failed();
    write_header(dst, for_id, l, 3, 0);
}
pub fn length_evt_zwlr_screencopy_frame_v1_failed() -> usize {
    8
}
pub fn parse_evt_zwlr_screencopy_frame_v1_failed<'a>(msg: &'a [u8]) -> Result<(), &'static str> {
    if msg.len() != 8 {
        return Err(PARSE_ERROR);
    }
    Ok(())
}
pub const OPCODE_ZWLR_SCREENCOPY_FRAME_V1_FAILED: MethodId = MethodId::Event(3);
pub fn write_req_zwlr_screencopy_frame_v1_destroy(dst: &mut &mut [u8], for_id: ObjId) {
    let l = length_req_zwlr_screencopy_frame_v1_destroy();
    write_header(dst, for_id, l, 1, 0);
}
pub fn length_req_zwlr_screencopy_frame_v1_destroy() -> usize {
    8
}
pub fn parse_req_zwlr_screencopy_frame_v1_destroy<'a>(msg: &'a [u8]) -> Result<(), &'static str> {
    if msg.len() != 8 {
        return Err(PARSE_ERROR);
    }
    Ok(())
}
pub const OPCODE_ZWLR_SCREENCOPY_FRAME_V1_DESTROY: MethodId = MethodId::Request(1);
pub fn write_req_zwlr_screencopy_frame_v1_copy_with_damage(
    dst: &mut &mut [u8],
    for_id: ObjId,
    buffer: ObjId,
) {
    let l = length_req_zwlr_screencopy_frame_v1_copy_with_damage();
    write_header(dst, for_id, l, 2, 0);
    write_obj(dst, buffer);
}
pub fn length_req_zwlr_screencopy_frame_v1_copy_with_damage() -> usize {
    12
}
pub fn parse_req_zwlr_screencopy_frame_v1_copy_with_damage<'a>(
    mut msg: &'a [u8],
) -> Result<ObjId, &'static str> {
    msg = msg.get(8..).ok_or(PARSE_ERROR)?;
    let arg1 = parse_obj(&mut msg)?;
    if !msg.is_empty() {
        return Err(PARSE_ERROR);
    }
    Ok(arg1)
}
pub const OPCODE_ZWLR_SCREENCOPY_FRAME_V1_COPY_WITH_DAMAGE: MethodId = MethodId::Request(2);
pub fn write_evt_zwlr_screencopy_frame_v1_linux_dmabuf(
    dst: &mut &mut [u8],
    for_id: ObjId,
    format: u32,
    width: u32,
    height: u32,
) {
    let l = length_evt_zwlr_screencopy_frame_v1_linux_dmabuf();
    write_header(dst, for_id, l, 5, 0);
    write_u32(dst, format);
    write_u32(dst, width);
    write_u32(dst, height);
}
pub fn length_evt_zwlr_screencopy_frame_v1_linux_dmabuf() -> usize {
    20
}
pub fn parse_evt_zwlr_screencopy_frame_v1_linux_dmabuf<'a>(
    mut msg: &'a [u8],
) -> Result<(u32, u32, u32), &'static str> {
    msg = msg.get(8..).ok_or(PARSE_ERROR)?;
    let arg1 = parse_u32(&mut msg)?;
    let arg2 = parse_u32(&mut msg)?;
    let arg3 = parse_u32(&mut msg)?;
    if !msg.is_empty() {
        return Err(PARSE_ERROR);
    }
    Ok((arg1, arg2, arg3))
}
pub const OPCODE_ZWLR_SCREENCOPY_FRAME_V1_LINUX_DMABUF: MethodId = MethodId::Event(5);
pub fn write_evt_zwlr_screencopy_frame_v1_buffer_done(dst: &mut &mut [u8], for_id: ObjId) {
    let l = length_evt_zwlr_screencopy_frame_v1_buffer_done();
    write_header(dst, for_id, l, 6, 0);
}
pub fn length_evt_zwlr_screencopy_frame_v1_buffer_done() -> usize {
    8
}
pub fn parse_evt_zwlr_screencopy_frame_v1_buffer_done<'a>(
    msg: &'a [u8],
) -> Result<(), &'static str> {
    if msg.len() != 8 {
        return Err(PARSE_ERROR);
    }
    Ok(())
}
pub const OPCODE_ZWLR_SCREENCOPY_FRAME_V1_BUFFER_DONE: MethodId = MethodId::Event(6);
const DATA_ZWLR_SCREENCOPY_FRAME_V1: WaylandData = WaylandData {
    name: "zwlr_screencopy_frame_v1",
    evts: &[
        WaylandMethod {
            name: "buffer",
            sig: &[Uint, Uint, Uint, Uint],
            destructor: false,
        },
        WaylandMethod {
            name: "flags",
            sig: &[Uint],
            destructor: false,
        },
        WaylandMethod {
            name: "ready",
            sig: &[Uint, Uint, Uint],
            destructor: false,
        },
        WaylandMethod {
            name: "failed",
            sig: &[],
            destructor: false,
        },
        WaylandMethod {
            name: "damage",
            sig: &[Uint, Uint, Uint, Uint],
            destructor: false,
        },
        WaylandMethod {
            name: "linux_dmabuf",
            sig: &[Uint, Uint, Uint],
            destructor: false,
        },
        WaylandMethod {
            name: "buffer_done",
            sig: &[],
            destructor: false,
        },
    ],
    reqs: &[
        WaylandMethod {
            name: "copy",
            sig: &[Object(WlBuffer)],
            destructor: false,
        },
        WaylandMethod {
            name: "destroy",
            sig: &[],
            destructor: true,
        },
        WaylandMethod {
            name: "copy_with_damage",
            sig: &[Object(WlBuffer)],
            destructor: false,
        },
    ],
    version: 3,
};
pub const ZWLR_SCREENCOPY_FRAME_V1: &[u8] = DATA_ZWLR_SCREENCOPY_FRAME_V1.name.as_bytes();
pub fn write_req_xdg_wm_base_get_xdg_surface(
    dst: &mut &mut [u8],
    for_id: ObjId,
    id: ObjId,
    surface: ObjId,
) {
    let l = length_req_xdg_wm_base_get_xdg_surface();
    write_header(dst, for_id, l, 2, 0);
    write_obj(dst, id);
    write_obj(dst, surface);
}
pub fn length_req_xdg_wm_base_get_xdg_surface() -> usize {
    16
}
pub fn parse_req_xdg_wm_base_get_xdg_surface<'a>(
    mut msg: &'a [u8],
) -> Result<(ObjId, ObjId), &'static str> {
    msg = msg.get(8..).ok_or(PARSE_ERROR)?;
    let arg1 = parse_obj(&mut msg)?;
    let arg2 = parse_obj(&mut msg)?;
    if !msg.is_empty() {
        return Err(PARSE_ERROR);
    }
    Ok((arg1, arg2))
}
pub const OPCODE_XDG_WM_BASE_GET_XDG_SURFACE: MethodId = MethodId::Request(2);
const DATA_XDG_WM_BASE: WaylandData = WaylandData {
    name: "xdg_wm_base",
    evts: &[WaylandMethod {
        name: "ping",
        sig: &[Uint],
        destructor: false,
    }],
    reqs: &[
        WaylandMethod {
            name: "destroy",
            sig: &[],
            destructor: true,
        },
        WaylandMethod {
            name: "create_positioner",
            sig: &[NewId(XdgPositioner)],
            destructor: false,
        },
        WaylandMethod {
            name: "get_xdg_surface",
            sig: &[NewId(XdgSurface), Object(WlSurface)],
            destructor: false,
        },
        WaylandMethod {
            name: "pong",
            sig: &[Uint],
            destructor: false,
        },
    ],
    version: 7,
};
pub const XDG_WM_BASE: &[u8] = DATA_XDG_WM_BASE.name.as_bytes();
const DATA_XDG_POSITIONER: WaylandData = WaylandData {
    name: "xdg_positioner",
    evts: &[],
    reqs: &[
        WaylandMethod {
            name: "destroy",
            sig: &[],
            destructor: true,
        },
        WaylandMethod {
            name: "set_size",
            sig: &[Int, Int],
            destructor: false,
        },
        WaylandMethod {
            name: "set_anchor_rect",
            sig: &[Int, Int, Int, Int],
            destructor: false,
        },
        WaylandMethod {
            name: "set_anchor",
            sig: &[Uint],
            destructor: false,
        },
        WaylandMethod {
            name: "set_gravity",
            sig: &[Uint],
            destructor: false,
        },
        WaylandMethod {
            name: "set_constraint_adjustment",
            sig: &[Uint],
            destructor: false,
        },
        WaylandMethod {
            name: "set_offset",
            sig: &[Int, Int],
            destructor: false,
        },
        WaylandMethod {
            name: "set_reactive",
            sig: &[],
            destructor: false,
        },
        WaylandMethod {
            name: "set_parent_size",
            sig: &[Int, Int],
            destructor: false,
        },
        WaylandMethod {
            name: "set_parent_configure",
            sig: &[Uint],
            destructor: false,
        },
    ],
    version: 7,
};
pub const XDG_POSITIONER: &[u8] = DATA_XDG_POSITIONER.name.as_bytes();
pub fn write_req_xdg_surface_get_toplevel(dst: &mut &mut [u8], for_id: ObjId, id: ObjId) {
    let l = length_req_xdg_surface_get_toplevel();
    write_header(dst, for_id, l, 1, 0);
    write_obj(dst, id);
}
pub fn length_req_xdg_surface_get_toplevel() -> usize {
    12
}
pub fn parse_req_xdg_surface_get_toplevel<'a>(mut msg: &'a [u8]) -> Result<ObjId, &'static str> {
    msg = msg.get(8..).ok_or(PARSE_ERROR)?;
    let arg1 = parse_obj(&mut msg)?;
    if !msg.is_empty() {
        return Err(PARSE_ERROR);
    }
    Ok(arg1)
}
pub const OPCODE_XDG_SURFACE_GET_TOPLEVEL: MethodId = MethodId::Request(1);
const DATA_XDG_SURFACE: WaylandData = WaylandData {
    name: "xdg_surface",
    evts: &[WaylandMethod {
        name: "configure",
        sig: &[Uint],
        destructor: false,
    }],
    reqs: &[
        WaylandMethod {
            name: "destroy",
            sig: &[],
            destructor: true,
        },
        WaylandMethod {
            name: "get_toplevel",
            sig: &[NewId(XdgToplevel)],
            destructor: false,
        },
        WaylandMethod {
            name: "get_popup",
            sig: &[NewId(XdgPopup), Object(XdgSurface), Object(XdgPositioner)],
            destructor: false,
        },
        WaylandMethod {
            name: "set_window_geometry",
            sig: &[Int, Int, Int, Int],
            destructor: false,
        },
        WaylandMethod {
            name: "ack_configure",
            sig: &[Uint],
            destructor: false,
        },
    ],
    version: 7,
};
pub const XDG_SURFACE: &[u8] = DATA_XDG_SURFACE.name.as_bytes();
pub fn write_req_xdg_toplevel_set_title(dst: &mut &mut [u8], for_id: ObjId, title: &[u8]) {
    let l = length_req_xdg_toplevel_set_title(title.len());
    write_header(dst, for_id, l, 2, 0);
    write_string(dst, Some(title));
}
pub fn length_req_xdg_toplevel_set_title(title_len: usize) -> usize {
    let mut v = 8;
    v += length_string(title_len);
    v
}
pub fn parse_req_xdg_toplevel_set_title<'a>(mut msg: &'a [u8]) -> Result<&'a [u8], &'static str> {
    msg = msg.get(8..).ok_or(PARSE_ERROR)?;
    let arg1 = parse_string(&mut msg)?.ok_or(PARSE_ERROR)?;
    if !msg.is_empty() {
        return Err(PARSE_ERROR);
    }
    Ok(arg1)
}
pub const OPCODE_XDG_TOPLEVEL_SET_TITLE: MethodId = MethodId::Request(2);
const DATA_XDG_TOPLEVEL: WaylandData = WaylandData {
    name: "xdg_toplevel",
    evts: &[
        WaylandMethod {
            name: "configure",
            sig: &[Int, Int, Array],
            destructor: false,
        },
        WaylandMethod {
            name: "close",
            sig: &[],
            destructor: false,
        },
        WaylandMethod {
            name: "configure_bounds",
            sig: &[Int, Int],
            destructor: false,
        },
        WaylandMethod {
            name: "wm_capabilities",
            sig: &[Array],
            destructor: false,
        },
    ],
    reqs: &[
        WaylandMethod {
            name: "destroy",
            sig: &[],
            destructor: true,
        },
        WaylandMethod {
            name: "set_parent",
            sig: &[Object(XdgToplevel)],
            destructor: false,
        },
        WaylandMethod {
            name: "set_title",
            sig: &[String],
            destructor: false,
        },
        WaylandMethod {
            name: "set_app_id",
            sig: &[String],
            destructor: false,
        },
        WaylandMethod {
            name: "show_window_menu",
            sig: &[Object(WlSeat), Uint, Int, Int],
            destructor: false,
        },
        WaylandMethod {
            name: "move",
            sig: &[Object(WlSeat), Uint],
            destructor: false,
        },
        WaylandMethod {
            name: "resize",
            sig: &[Object(WlSeat), Uint, Uint],
            destructor: false,
        },
        WaylandMethod {
            name: "set_max_size",
            sig: &[Int, Int],
            destructor: false,
        },
        WaylandMethod {
            name: "set_min_size",
            sig: &[Int, Int],
            destructor: false,
        },
        WaylandMethod {
            name: "set_maximized",
            sig: &[],
            destructor: false,
        },
        WaylandMethod {
            name: "unset_maximized",
            sig: &[],
            destructor: false,
        },
        WaylandMethod {
            name: "set_fullscreen",
            sig: &[Object(WlOutput)],
            destructor: false,
        },
        WaylandMethod {
            name: "unset_fullscreen",
            sig: &[],
            destructor: false,
        },
        WaylandMethod {
            name: "set_minimized",
            sig: &[],
            destructor: false,
        },
    ],
    version: 7,
};
pub const XDG_TOPLEVEL: &[u8] = DATA_XDG_TOPLEVEL.name.as_bytes();
const DATA_XDG_POPUP: WaylandData = WaylandData {
    name: "xdg_popup",
    evts: &[
        WaylandMethod {
            name: "configure",
            sig: &[Int, Int, Int, Int],
            destructor: false,
        },
        WaylandMethod {
            name: "popup_done",
            sig: &[],
            destructor: false,
        },
        WaylandMethod {
            name: "repositioned",
            sig: &[Uint],
            destructor: false,
        },
    ],
    reqs: &[
        WaylandMethod {
            name: "destroy",
            sig: &[],
            destructor: true,
        },
        WaylandMethod {
            name: "grab",
            sig: &[Object(WlSeat), Uint],
            destructor: false,
        },
        WaylandMethod {
            name: "reposition",
            sig: &[Object(XdgPositioner), Uint],
            destructor: false,
        },
    ],
    version: 7,
};
pub const XDG_POPUP: &[u8] = DATA_XDG_POPUP.name.as_bytes();
pub fn write_req_xdg_toplevel_icon_manager_v1_create_icon(
    dst: &mut &mut [u8],
    for_id: ObjId,
    id: ObjId,
) {
    let l = length_req_xdg_toplevel_icon_manager_v1_create_icon();
    write_header(dst, for_id, l, 1, 0);
    write_obj(dst, id);
}
pub fn length_req_xdg_toplevel_icon_manager_v1_create_icon() -> usize {
    12
}
pub fn parse_req_xdg_toplevel_icon_manager_v1_create_icon<'a>(
    mut msg: &'a [u8],
) -> Result<ObjId, &'static str> {
    msg = msg.get(8..).ok_or(PARSE_ERROR)?;
    let arg1 = parse_obj(&mut msg)?;
    if !msg.is_empty() {
        return Err(PARSE_ERROR);
    }
    Ok(arg1)
}
pub const OPCODE_XDG_TOPLEVEL_ICON_MANAGER_V1_CREATE_ICON: MethodId = MethodId::Request(1);
pub fn write_evt_xdg_toplevel_icon_manager_v1_icon_size(
    dst: &mut &mut [u8],
    for_id: ObjId,
    size: i32,
) {
    let l = length_evt_xdg_toplevel_icon_manager_v1_icon_size();
    write_header(dst, for_id, l, 0, 0);
    write_i32(dst, size);
}
pub fn length_evt_xdg_toplevel_icon_manager_v1_icon_size() -> usize {
    12
}
pub fn parse_evt_xdg_toplevel_icon_manager_v1_icon_size<'a>(
    mut msg: &'a [u8],
) -> Result<i32, &'static str> {
    msg = msg.get(8..).ok_or(PARSE_ERROR)?;
    let arg1 = parse_i32(&mut msg)?;
    if !msg.is_empty() {
        return Err(PARSE_ERROR);
    }
    Ok(arg1)
}
pub const OPCODE_XDG_TOPLEVEL_ICON_MANAGER_V1_ICON_SIZE: MethodId = MethodId::Event(0);
pub fn write_evt_xdg_toplevel_icon_manager_v1_done(dst: &mut &mut [u8], for_id: ObjId) {
    let l = length_evt_xdg_toplevel_icon_manager_v1_done();
    write_header(dst, for_id, l, 1, 0);
}
pub fn length_evt_xdg_toplevel_icon_manager_v1_done() -> usize {
    8
}
pub fn parse_evt_xdg_toplevel_icon_manager_v1_done<'a>(msg: &'a [u8]) -> Result<(), &'static str> {
    if msg.len() != 8 {
        return Err(PARSE_ERROR);
    }
    Ok(())
}
pub const OPCODE_XDG_TOPLEVEL_ICON_MANAGER_V1_DONE: MethodId = MethodId::Event(1);
const DATA_XDG_TOPLEVEL_ICON_MANAGER_V1: WaylandData = WaylandData {
    name: "xdg_toplevel_icon_manager_v1",
    evts: &[
        WaylandMethod {
            name: "icon_size",
            sig: &[Int],
            destructor: false,
        },
        WaylandMethod {
            name: "done",
            sig: &[],
            destructor: false,
        },
    ],
    reqs: &[
        WaylandMethod {
            name: "destroy",
            sig: &[],
            destructor: true,
        },
        WaylandMethod {
            name: "create_icon",
            sig: &[NewId(XdgToplevelIconV1)],
            destructor: false,
        },
        WaylandMethod {
            name: "set_icon",
            sig: &[Object(XdgToplevel), Object(XdgToplevelIconV1)],
            destructor: false,
        },
    ],
    version: 1,
};
pub const XDG_TOPLEVEL_ICON_MANAGER_V1: &[u8] = DATA_XDG_TOPLEVEL_ICON_MANAGER_V1.name.as_bytes();
pub fn write_req_xdg_toplevel_icon_v1_add_buffer(
    dst: &mut &mut [u8],
    for_id: ObjId,
    buffer: ObjId,
    scale: i32,
) {
    let l = length_req_xdg_toplevel_icon_v1_add_buffer();
    write_header(dst, for_id, l, 2, 0);
    write_obj(dst, buffer);
    write_i32(dst, scale);
}
pub fn length_req_xdg_toplevel_icon_v1_add_buffer() -> usize {
    16
}
pub fn parse_req_xdg_toplevel_icon_v1_add_buffer<'a>(
    mut msg: &'a [u8],
) -> Result<(ObjId, i32), &'static str> {
    msg = msg.get(8..).ok_or(PARSE_ERROR)?;
    let arg1 = parse_obj(&mut msg)?;
    let arg2 = parse_i32(&mut msg)?;
    if !msg.is_empty() {
        return Err(PARSE_ERROR);
    }
    Ok((arg1, arg2))
}
pub const OPCODE_XDG_TOPLEVEL_ICON_V1_ADD_BUFFER: MethodId = MethodId::Request(2);
const DATA_XDG_TOPLEVEL_ICON_V1: WaylandData = WaylandData {
    name: "xdg_toplevel_icon_v1",
    evts: &[],
    reqs: &[
        WaylandMethod {
            name: "destroy",
            sig: &[],
            destructor: true,
        },
        WaylandMethod {
            name: "set_name",
            sig: &[String],
            destructor: false,
        },
        WaylandMethod {
            name: "add_buffer",
            sig: &[Object(WlBuffer), Int],
            destructor: false,
        },
    ],
    version: 1,
};
pub const XDG_TOPLEVEL_ICON_V1: &[u8] = DATA_XDG_TOPLEVEL_ICON_V1.name.as_bytes();
#[repr(u8)]
#[derive(Debug, Clone, Copy)]
pub enum WaylandInterface {
    ExtDataControlDeviceV1,
    ExtDataControlManagerV1,
    ExtDataControlOfferV1,
    ExtDataControlSourceV1,
    ExtForeignToplevelHandleV1,
    ExtForeignToplevelImageCaptureSourceManagerV1,
    ExtForeignToplevelListV1,
    ExtImageCaptureSourceV1,
    ExtImageCopyCaptureCursorSessionV1,
    ExtImageCopyCaptureFrameV1,
    ExtImageCopyCaptureManagerV1,
    ExtImageCopyCaptureSessionV1,
    ExtOutputImageCaptureSourceManagerV1,
    GtkPrimarySelectionDevice,
    GtkPrimarySelectionDeviceManager,
    GtkPrimarySelectionOffer,
    GtkPrimarySelectionSource,
    WlBuffer,
    WlCallback,
    WlCompositor,
    WlDataDevice,
    WlDataDeviceManager,
    WlDataOffer,
    WlDataSource,
    WlDisplay,
    WlDrm,
    WlFixes,
    WlKeyboard,
    WlOutput,
    WlPointer,
    WlRegion,
    WlRegistry,
    WlSeat,
    WlShell,
    WlShellSurface,
    WlShm,
    WlShmPool,
    WlSubcompositor,
    WlSubsurface,
    WlSurface,
    WlTouch,
    WpColorManagementOutputV1,
    WpColorManagementSurfaceFeedbackV1,
    WpColorManagementSurfaceV1,
    WpColorManagerV1,
    WpCommitTimerV1,
    WpCommitTimingManagerV1,
    WpImageDescriptionCreatorIccV1,
    WpImageDescriptionCreatorParamsV1,
    WpImageDescriptionInfoV1,
    WpImageDescriptionReferenceV1,
    WpImageDescriptionV1,
    WpLinuxDrmSyncobjManagerV1,
    WpLinuxDrmSyncobjSurfaceV1,
    WpLinuxDrmSyncobjTimelineV1,
    WpPresentation,
    WpPresentationFeedback,
    WpSecurityContextManagerV1,
    WpSecurityContextV1,
    WpViewport,
    WpViewporter,
    XdgPopup,
    XdgPositioner,
    XdgSurface,
    XdgToplevel,
    XdgToplevelIconManagerV1,
    XdgToplevelIconV1,
    XdgWmBase,
    ZwlrDataControlDeviceV1,
    ZwlrDataControlManagerV1,
    ZwlrDataControlOfferV1,
    ZwlrDataControlSourceV1,
    ZwlrExportDmabufFrameV1,
    ZwlrExportDmabufManagerV1,
    ZwlrGammaControlManagerV1,
    ZwlrGammaControlV1,
    ZwlrScreencopyFrameV1,
    ZwlrScreencopyManagerV1,
    ZwpInputMethodKeyboardGrabV2,
    ZwpInputMethodManagerV2,
    ZwpInputMethodV2,
    ZwpInputPopupSurfaceV2,
    ZwpLinuxBufferParamsV1,
    ZwpLinuxDmabufFeedbackV1,
    ZwpLinuxDmabufV1,
    ZwpPrimarySelectionDeviceManagerV1,
    ZwpPrimarySelectionDeviceV1,
    ZwpPrimarySelectionOfferV1,
    ZwpPrimarySelectionSourceV1,
    ZwpVirtualKeyboardManagerV1,
    ZwpVirtualKeyboardV1,
}
const WAYLANDINTERFACE_LIST: &[WaylandInterface; 91] = &[
    ExtDataControlDeviceV1,
    ExtDataControlManagerV1,
    ExtDataControlOfferV1,
    ExtDataControlSourceV1,
    ExtForeignToplevelHandleV1,
    ExtForeignToplevelImageCaptureSourceManagerV1,
    ExtForeignToplevelListV1,
    ExtImageCaptureSourceV1,
    ExtImageCopyCaptureCursorSessionV1,
    ExtImageCopyCaptureFrameV1,
    ExtImageCopyCaptureManagerV1,
    ExtImageCopyCaptureSessionV1,
    ExtOutputImageCaptureSourceManagerV1,
    GtkPrimarySelectionDevice,
    GtkPrimarySelectionDeviceManager,
    GtkPrimarySelectionOffer,
    GtkPrimarySelectionSource,
    WlBuffer,
    WlCallback,
    WlCompositor,
    WlDataDevice,
    WlDataDeviceManager,
    WlDataOffer,
    WlDataSource,
    WlDisplay,
    WlDrm,
    WlFixes,
    WlKeyboard,
    WlOutput,
    WlPointer,
    WlRegion,
    WlRegistry,
    WlSeat,
    WlShell,
    WlShellSurface,
    WlShm,
    WlShmPool,
    WlSubcompositor,
    WlSubsurface,
    WlSurface,
    WlTouch,
    WpColorManagementOutputV1,
    WpColorManagementSurfaceFeedbackV1,
    WpColorManagementSurfaceV1,
    WpColorManagerV1,
    WpCommitTimerV1,
    WpCommitTimingManagerV1,
    WpImageDescriptionCreatorIccV1,
    WpImageDescriptionCreatorParamsV1,
    WpImageDescriptionInfoV1,
    WpImageDescriptionReferenceV1,
    WpImageDescriptionV1,
    WpLinuxDrmSyncobjManagerV1,
    WpLinuxDrmSyncobjSurfaceV1,
    WpLinuxDrmSyncobjTimelineV1,
    WpPresentation,
    WpPresentationFeedback,
    WpSecurityContextManagerV1,
    WpSecurityContextV1,
    WpViewport,
    WpViewporter,
    XdgPopup,
    XdgPositioner,
    XdgSurface,
    XdgToplevel,
    XdgToplevelIconManagerV1,
    XdgToplevelIconV1,
    XdgWmBase,
    ZwlrDataControlDeviceV1,
    ZwlrDataControlManagerV1,
    ZwlrDataControlOfferV1,
    ZwlrDataControlSourceV1,
    ZwlrExportDmabufFrameV1,
    ZwlrExportDmabufManagerV1,
    ZwlrGammaControlManagerV1,
    ZwlrGammaControlV1,
    ZwlrScreencopyFrameV1,
    ZwlrScreencopyManagerV1,
    ZwpInputMethodKeyboardGrabV2,
    ZwpInputMethodManagerV2,
    ZwpInputMethodV2,
    ZwpInputPopupSurfaceV2,
    ZwpLinuxBufferParamsV1,
    ZwpLinuxDmabufFeedbackV1,
    ZwpLinuxDmabufV1,
    ZwpPrimarySelectionDeviceManagerV1,
    ZwpPrimarySelectionDeviceV1,
    ZwpPrimarySelectionOfferV1,
    ZwpPrimarySelectionSourceV1,
    ZwpVirtualKeyboardManagerV1,
    ZwpVirtualKeyboardV1,
];
impl TryFrom<usize> for WaylandInterface {
    type Error = ();
    fn try_from(v: usize) -> Result<Self, Self::Error> {
        WAYLANDINTERFACE_LIST.get(v).copied().ok_or(())
    }
}
pub const INTERFACE_TABLE: &[WaylandData; 91] = &[
    DATA_EXT_DATA_CONTROL_DEVICE_V1,
    DATA_EXT_DATA_CONTROL_MANAGER_V1,
    DATA_EXT_DATA_CONTROL_OFFER_V1,
    DATA_EXT_DATA_CONTROL_SOURCE_V1,
    DATA_EXT_FOREIGN_TOPLEVEL_HANDLE_V1,
    DATA_EXT_FOREIGN_TOPLEVEL_IMAGE_CAPTURE_SOURCE_MANAGER_V1,
    DATA_EXT_FOREIGN_TOPLEVEL_LIST_V1,
    DATA_EXT_IMAGE_CAPTURE_SOURCE_V1,
    DATA_EXT_IMAGE_COPY_CAPTURE_CURSOR_SESSION_V1,
    DATA_EXT_IMAGE_COPY_CAPTURE_FRAME_V1,
    DATA_EXT_IMAGE_COPY_CAPTURE_MANAGER_V1,
    DATA_EXT_IMAGE_COPY_CAPTURE_SESSION_V1,
    DATA_EXT_OUTPUT_IMAGE_CAPTURE_SOURCE_MANAGER_V1,
    DATA_GTK_PRIMARY_SELECTION_DEVICE,
    DATA_GTK_PRIMARY_SELECTION_DEVICE_MANAGER,
    DATA_GTK_PRIMARY_SELECTION_OFFER,
    DATA_GTK_PRIMARY_SELECTION_SOURCE,
    DATA_WL_BUFFER,
    DATA_WL_CALLBACK,
    DATA_WL_COMPOSITOR,
    DATA_WL_DATA_DEVICE,
    DATA_WL_DATA_DEVICE_MANAGER,
    DATA_WL_DATA_OFFER,
    DATA_WL_DATA_SOURCE,
    DATA_WL_DISPLAY,
    DATA_WL_DRM,
    DATA_WL_FIXES,
    DATA_WL_KEYBOARD,
    DATA_WL_OUTPUT,
    DATA_WL_POINTER,
    DATA_WL_REGION,
    DATA_WL_REGISTRY,
    DATA_WL_SEAT,
    DATA_WL_SHELL,
    DATA_WL_SHELL_SURFACE,
    DATA_WL_SHM,
    DATA_WL_SHM_POOL,
    DATA_WL_SUBCOMPOSITOR,
    DATA_WL_SUBSURFACE,
    DATA_WL_SURFACE,
    DATA_WL_TOUCH,
    DATA_WP_COLOR_MANAGEMENT_OUTPUT_V1,
    DATA_WP_COLOR_MANAGEMENT_SURFACE_FEEDBACK_V1,
    DATA_WP_COLOR_MANAGEMENT_SURFACE_V1,
    DATA_WP_COLOR_MANAGER_V1,
    DATA_WP_COMMIT_TIMER_V1,
    DATA_WP_COMMIT_TIMING_MANAGER_V1,
    DATA_WP_IMAGE_DESCRIPTION_CREATOR_ICC_V1,
    DATA_WP_IMAGE_DESCRIPTION_CREATOR_PARAMS_V1,
    DATA_WP_IMAGE_DESCRIPTION_INFO_V1,
    DATA_WP_IMAGE_DESCRIPTION_REFERENCE_V1,
    DATA_WP_IMAGE_DESCRIPTION_V1,
    DATA_WP_LINUX_DRM_SYNCOBJ_MANAGER_V1,
    DATA_WP_LINUX_DRM_SYNCOBJ_SURFACE_V1,
    DATA_WP_LINUX_DRM_SYNCOBJ_TIMELINE_V1,
    DATA_WP_PRESENTATION,
    DATA_WP_PRESENTATION_FEEDBACK,
    DATA_WP_SECURITY_CONTEXT_MANAGER_V1,
    DATA_WP_SECURITY_CONTEXT_V1,
    DATA_WP_VIEWPORT,
    DATA_WP_VIEWPORTER,
    DATA_XDG_POPUP,
    DATA_XDG_POSITIONER,
    DATA_XDG_SURFACE,
    DATA_XDG_TOPLEVEL,
    DATA_XDG_TOPLEVEL_ICON_MANAGER_V1,
    DATA_XDG_TOPLEVEL_ICON_V1,
    DATA_XDG_WM_BASE,
    DATA_ZWLR_DATA_CONTROL_DEVICE_V1,
    DATA_ZWLR_DATA_CONTROL_MANAGER_V1,
    DATA_ZWLR_DATA_CONTROL_OFFER_V1,
    DATA_ZWLR_DATA_CONTROL_SOURCE_V1,
    DATA_ZWLR_EXPORT_DMABUF_FRAME_V1,
    DATA_ZWLR_EXPORT_DMABUF_MANAGER_V1,
    DATA_ZWLR_GAMMA_CONTROL_MANAGER_V1,
    DATA_ZWLR_GAMMA_CONTROL_V1,
    DATA_ZWLR_SCREENCOPY_FRAME_V1,
    DATA_ZWLR_SCREENCOPY_MANAGER_V1,
    DATA_ZWP_INPUT_METHOD_KEYBOARD_GRAB_V2,
    DATA_ZWP_INPUT_METHOD_MANAGER_V2,
    DATA_ZWP_INPUT_METHOD_V2,
    DATA_ZWP_INPUT_POPUP_SURFACE_V2,
    DATA_ZWP_LINUX_BUFFER_PARAMS_V1,
    DATA_ZWP_LINUX_DMABUF_FEEDBACK_V1,
    DATA_ZWP_LINUX_DMABUF_V1,
    DATA_ZWP_PRIMARY_SELECTION_DEVICE_MANAGER_V1,
    DATA_ZWP_PRIMARY_SELECTION_DEVICE_V1,
    DATA_ZWP_PRIMARY_SELECTION_OFFER_V1,
    DATA_ZWP_PRIMARY_SELECTION_SOURCE_V1,
    DATA_ZWP_VIRTUAL_KEYBOARD_MANAGER_V1,
    DATA_ZWP_VIRTUAL_KEYBOARD_V1,
];
