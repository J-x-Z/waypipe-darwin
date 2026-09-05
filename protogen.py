#!/usr/bin/env python3
# SPDX-License-Identifier: GPL-3.0-or-later

"""
Read protocol files, and convert to a rust-readable format
"""

import os
import xml.etree.ElementTree
import subprocess

# Table of functions for which to generate helper code
handled_funcs = [
    ("ext_data_control_device_v1", "data_offer"),
    ("ext_data_control_device_v1", "selection"),
    ("ext_data_control_device_v1", "set_selection"),
    ("ext_data_control_manager_v1", "create_data_source"),
    ("ext_data_control_manager_v1", "get_data_device"),
    ("ext_data_control_offer_v1", "offer"),
    ("ext_data_control_offer_v1", "receive"),
    ("ext_data_control_source_v1", "offer"),
    ("ext_data_control_source_v1", "send"),
    ("ext_image_copy_capture_cursor_session_v1", "get_capture_session"),
    ("ext_image_copy_capture_frame_v1", "attach_buffer"),
    ("ext_image_copy_capture_frame_v1", "capture"),
    ("ext_image_copy_capture_frame_v1", "damage_buffer"),
    ("ext_image_copy_capture_frame_v1", "destroy"),
    ("ext_image_copy_capture_frame_v1", "failed"),
    ("ext_image_copy_capture_frame_v1", "presentation_time"),
    ("ext_image_copy_capture_frame_v1", "ready"),
    ("ext_image_copy_capture_manager_v1", "create_session"),
    ("ext_image_copy_capture_session_v1", "buffer_size"),
    ("ext_image_copy_capture_session_v1", "create_frame"),
    ("ext_image_copy_capture_session_v1", "destroy"),
    ("ext_image_copy_capture_session_v1", "dmabuf_device"),
    ("ext_image_copy_capture_session_v1", "dmabuf_format"),
    ("ext_image_copy_capture_session_v1", "done"),
    ("ext_image_copy_capture_session_v1", "shm_format"),
    ("ext_output_image_capture_source_manager_v1", "create_source"),
    ("gtk_primary_selection_device", "data_offer"),
    ("gtk_primary_selection_device", "selection"),
    ("gtk_primary_selection_device", "set_selection"),
    ("gtk_primary_selection_device_manager", "create_source"),
    ("gtk_primary_selection_device_manager", "get_device"),
    ("gtk_primary_selection_offer", "offer"),
    ("gtk_primary_selection_offer", "receive"),
    ("gtk_primary_selection_source", "offer"),
    ("gtk_primary_selection_source", "send"),
    ("wl_buffer", "destroy"),
    ("wl_buffer", "release"),
    ("wl_callback", "done"),
    ("wl_compositor", "create_surface"),
    ("wl_data_device", "data_offer"),
    ("wl_data_device", "selection"),
    ("wl_data_device", "set_selection"),
    ("wl_data_device_manager", "create_data_source"),
    ("wl_data_device_manager", "get_data_device"),
    ("wl_data_offer", "offer"),
    ("wl_data_offer", "receive"),
    ("wl_data_source", "offer"),
    ("wl_data_source", "send"),
    ("wl_display", "delete_id"),
    ("wl_display", "error"),
    ("wl_display", "get_registry"),
    ("wl_display", "sync"),
    ("wl_fixes", "destroy"),
    ("wl_fixes", "destroy_registry"),
    ("wl_keyboard", "keymap"),
    ("wl_registry", "bind"),
    ("wl_registry", "global"),
    ("wl_registry", "global_remove"),
    ("wl_seat", "capabilities"),
    ("wl_seat", "get_keyboard"),
    ("wl_shm", "create_pool"),
    ("wl_shm_pool", "create_buffer"),
    ("wl_shm_pool", "destroy"),
    ("wl_shm_pool", "resize"),
    ("wl_surface", "attach"),
    ("wl_surface", "commit"),
    ("wl_surface", "damage"),
    ("wl_surface", "damage_buffer"),
    ("wl_surface", "destroy"),
    ("wl_surface", "set_buffer_scale"),
    ("wl_surface", "set_buffer_transform"),
    ("wp_color_management_output_v1", "get_image_description"),
    ("wp_color_manager_v1", "create_icc_creator"),
    ("wp_color_manager_v1", "get_output"),
    ("wp_commit_timer_v1", "set_timestamp"),
    ("wp_commit_timing_manager_v1", "get_timer"),
    ("wp_image_description_creator_icc_v1", "set_icc_file"),
    ("wp_image_description_info_v1", "icc_file"),
    ("wp_image_description_v1", "get_information"),
    ("wp_linux_drm_syncobj_manager_v1", "get_surface"),
    ("wp_linux_drm_syncobj_manager_v1", "import_timeline"),
    ("wp_linux_drm_syncobj_surface_v1", "set_acquire_point"),
    ("wp_linux_drm_syncobj_surface_v1", "set_release_point"),
    ("wp_presentation", "clock_id"),
    ("wp_presentation", "destroy"),
    ("wp_presentation", "feedback"),
    ("wp_presentation_feedback", "presented"),
    ("wp_security_context_manager_v1", "create_listener"),
    ("wp_security_context_v1", "commit"),
    ("wp_security_context_v1", "set_app_id"),
    ("wp_security_context_v1", "set_instance_id"),
    ("wp_security_context_v1", "set_sandbox_engine"),
    ("wp_viewport", "destroy"),
    ("wp_viewport", "set_destination"),
    ("wp_viewport", "set_source"),
    ("wp_viewporter", "get_viewport"),
    ("xdg_surface", "get_toplevel"),
    ("xdg_toplevel", "set_title"),
    ("xdg_toplevel_icon_manager_v1", "create_icon"),
    ("xdg_toplevel_icon_manager_v1", "done"),
    ("xdg_toplevel_icon_manager_v1", "icon_size"),
    ("xdg_toplevel_icon_v1", "add_buffer"),
    ("xdg_wm_base", "get_xdg_surface"),
    ("zwlr_data_control_device_v1", "data_offer"),
    ("zwlr_data_control_device_v1", "selection"),
    ("zwlr_data_control_device_v1", "set_selection"),
    ("zwlr_data_control_manager_v1", "create_data_source"),
    ("zwlr_data_control_manager_v1", "get_data_device"),
    ("zwlr_data_control_offer_v1", "offer"),
    ("zwlr_data_control_offer_v1", "receive"),
    ("zwlr_data_control_source_v1", "offer"),
    ("zwlr_data_control_source_v1", "send"),
    ("zwlr_gamma_control_manager_v1", "get_gamma_control"),
    ("zwlr_gamma_control_v1", "gamma_size"),
    ("zwlr_gamma_control_v1", "set_gamma"),
    ("zwlr_screencopy_frame_v1", "buffer"),
    ("zwlr_screencopy_frame_v1", "buffer_done"),
    ("zwlr_screencopy_frame_v1", "copy"),
    ("zwlr_screencopy_frame_v1", "copy_with_damage"),
    ("zwlr_screencopy_frame_v1", "destroy"),
    ("zwlr_screencopy_frame_v1", "failed"),
    ("zwlr_screencopy_frame_v1", "linux_dmabuf"),
    ("zwlr_screencopy_frame_v1", "ready"),
    ("zwlr_screencopy_manager_v1", "capture_output"),
    ("zwlr_screencopy_manager_v1", "capture_output_region"),
    ("zwp_linux_buffer_params_v1", "add"),
    ("zwp_linux_buffer_params_v1", "create"),
    ("zwp_linux_buffer_params_v1", "create_immed"),
    ("zwp_linux_buffer_params_v1", "created"),
    ("zwp_linux_buffer_params_v1", "destroy"),
    ("zwp_linux_buffer_params_v1", "failed"),
    ("zwp_linux_dmabuf_feedback_v1", "done"),
    ("zwp_linux_dmabuf_feedback_v1", "format_table"),
    ("zwp_linux_dmabuf_feedback_v1", "main_device"),
    ("zwp_linux_dmabuf_feedback_v1", "tranche_done"),
    ("zwp_linux_dmabuf_feedback_v1", "tranche_flags"),
    ("zwp_linux_dmabuf_feedback_v1", "tranche_formats"),
    ("zwp_linux_dmabuf_feedback_v1", "tranche_target_device"),
    ("zwp_linux_dmabuf_v1", "create_params"),
    ("zwp_linux_dmabuf_v1", "format"),
    ("zwp_linux_dmabuf_v1", "get_default_feedback"),
    ("zwp_linux_dmabuf_v1", "get_surface_feedback"),
    ("zwp_linux_dmabuf_v1", "modifier"),
    ("zwp_primary_selection_device_manager_v1", "create_source"),
    ("zwp_primary_selection_device_manager_v1", "get_device"),
    ("zwp_primary_selection_device_v1", "data_offer"),
    ("zwp_primary_selection_device_v1", "selection"),
    ("zwp_primary_selection_device_v1", "set_selection"),
    ("zwp_primary_selection_offer_v1", "offer"),
    ("zwp_primary_selection_offer_v1", "receive"),
    ("zwp_primary_selection_source_v1", "offer"),
    ("zwp_primary_selection_source_v1", "send"),
]
assert handled_funcs == sorted(handled_funcs), "\n".join(
    [str(x) for x in sorted(handled_funcs)]
)
handled_funcs = set(handled_funcs)

handled_enums = set([("wl_output", "transform"), ("wl_shm", "format")])

header = """/*! Wayland protocol interface and method data and functions. Code automatically generated from protocols/ folder. */
#![allow(clippy::all, dead_code, missing_docs)]
use crate::wayland::*;
use crate::wayland::WaylandArgument::*;
use WaylandInterface::*;
"""


def unsnake(s):
    return "".join(map(lambda x: x.capitalize(), s.split("_")))


def get_signature(method):
    """
    returns a string like "[Int, Uint, NewId(WlOutput)]"
    """
    vals = []
    for arg in method:
        if arg.tag != "arg":
            continue
        allow_null = "allow-null" in arg.attrib and arg.attrib["allow-null"] == "true"

        if arg.attrib["type"] == "object":
            if "interface" in arg.attrib:
                vals.append("Object(" + unsnake(arg.attrib["interface"]) + ")")
            else:
                vals.append("GenericObject")
        elif arg.attrib["type"] == "new_id":
            if "interface" in arg.attrib:
                vals.append("NewId(" + unsnake(arg.attrib["interface"]) + ")")
            else:
                vals.append("GenericNewId")
        elif arg.attrib["type"] == "int":
            vals.append("Int")
        elif arg.attrib["type"] == "uint":
            vals.append("Uint")
        elif arg.attrib["type"] == "fixed":
            vals.append("Fixed")
        elif arg.attrib["type"] == "string":
            if allow_null:
                vals.append("OptionalString")
            else:
                vals.append("String")
        elif arg.attrib["type"] == "array":
            vals.append("Array")
        elif arg.attrib["type"] == "fd":
            vals.append("Fd")
        else:
            raise NotImplementedError(arg.attrib)
    return "&[" + ", ".join(vals) + "]"


def write_method_length(meth_name, method, write):
    """
    Create a function to report how long the method would be
    """
    lines = []
    args = []
    base_len = 8
    for arg in method:
        if arg.tag != "arg":
            continue
        if arg.attrib["type"] == "new_id":
            if "interface" in arg.attrib:
                base_len += 4
            else:
                arg_name = arg.attrib["name"] + "_iface_name_len"
                args.append(arg_name + ": usize")
                lines.append("    v += length_string({});".format(arg_name))
                base_len += 8
        elif arg.attrib["type"] in ("int", "uint", "object", "fixed"):
            base_len += 4
        elif arg.attrib["type"] == "string":
            args.append(arg.attrib["name"] + "_len : usize")
            lines.append(
                "    v += length_string({});".format(arg.attrib["name"] + "_len")
            )
        elif arg.attrib["type"] == "array":
            args.append(arg.attrib["name"] + "_len : usize")
            lines.append(
                "    v += length_array({});".format(arg.attrib["name"] + "_len")
            )
        elif arg.attrib["type"] == "fd":
            pass
        else:
            raise NotImplementedError(arg.attrib)

    write("pub fn length_{}({}) -> usize {{".format(meth_name, ", ".join(args)))
    if lines:
        write("    let mut v = {};".format(base_len))
        for l in lines:
            write(l)
        write("    v")
    else:
        write("    {}".format(base_len))
    write("}")


def write_method_write(meth_name, meth_num, method, write):
    """
    Create a function to write the method to a buffer
    """

    num_fds = 0
    length_args = []
    for arg in method:
        if arg.tag != "arg":
            continue
        if arg.attrib["type"] == "new_id":
            if "interface" not in arg.attrib:
                length_args.append(arg.attrib["name"] + "_iface_name")
        elif arg.attrib["type"] == "string":
            length_args.append(arg.attrib["name"])
        elif arg.attrib["type"] == "array":
            length_args.append(arg.attrib["name"])
        elif arg.attrib["type"] == "fd":
            num_fds += 1
        elif arg.attrib["type"] in ("uint", "int", "object", "fixed"):
            pass
        else:
            raise NotImplementedError(arg.attrib)
    length_args = [x + ".len()" for x in length_args]

    args = ["dst: &mut &mut [u8]", "for_id: ObjId"]
    lines = [
        "    let l = length_{}({});".format(meth_name, ", ".join(length_args)),
    ]
    if num_fds > 0:
        args.append("tag_fds: bool")
        lines.append(
            "    write_header(dst, for_id, l, {}, if tag_fds {{ {} }} else {{ 0 }});".format(
                meth_num, num_fds
            )
        )
    else:
        lines.append("    write_header(dst, for_id, l, {}, 0);".format(meth_num))

    base_len = 8
    for arg in method:
        if arg.tag != "arg":
            continue
        allow_null = "allow-null" in arg.attrib and arg.attrib["allow-null"] == "true"

        if arg.attrib["type"] == "object":
            args.append(arg.attrib["name"] + ": ObjId")
            lines.append("    write_obj(dst, {});".format(arg.attrib["name"]))
        elif arg.attrib["type"] == "new_id":
            assert not allow_null
            if "interface" in arg.attrib:
                args.append(arg.attrib["name"] + ": ObjId")
                lines.append("    write_obj(dst, {});".format(arg.attrib["name"]))
            else:
                args.append(arg.attrib["name"] + "_iface_name" + ": &[u8]")
                args.append(arg.attrib["name"] + "_version" + ": u32")
                args.append(arg.attrib["name"] + ": ObjId")
                lines.append(
                    "    write_string(dst, Some({}));".format(
                        arg.attrib["name"] + "_iface_name"
                    )
                )
                lines.append(
                    "    write_u32(dst, {});".format(arg.attrib["name"] + "_version")
                )
                lines.append("    write_obj(dst, {});".format(arg.attrib["name"]))
        elif arg.attrib["type"] == "int":
            args.append(arg.attrib["name"] + ": i32")
            lines.append("    write_i32(dst, {});".format(arg.attrib["name"]))
        elif arg.attrib["type"] == "uint":
            args.append(arg.attrib["name"] + ": u32")
            lines.append("    write_u32(dst, {});".format(arg.attrib["name"]))
        elif arg.attrib["type"] == "fixed":
            args.append(arg.attrib["name"] + ": i32")
            lines.append("    write_i32(dst, {});".format(arg.attrib["name"]))
        elif arg.attrib["type"] == "string":
            if allow_null:
                args.append(arg.attrib["name"] + ": Option<&[u8]>")
                lines.append("    write_string(dst, {});".format(arg.attrib["name"]))
            else:
                args.append(arg.attrib["name"] + ": &[u8]")
                lines.append(
                    "    write_string(dst, Some({}));".format(arg.attrib["name"])
                )
        elif arg.attrib["type"] == "array":
            args.append(arg.attrib["name"] + ": &[u8]")
            lines.append("    write_array(dst, {});".format(arg.attrib["name"]))
        elif arg.attrib["type"] == "fd":
            pass
        else:
            raise NotImplementedError(arg.attrib)

    write("pub fn write_{}({}) {{".format(meth_name, ", ".join(args)))
    for l in lines:
        write(l)
    write("}")


def write_method_parse(meth_name, method, write):
    """
    Create a function to parse the method tail
    """
    length_args = []
    for arg in method:
        if arg.tag != "arg":
            continue
        if arg.attrib["type"] == "new_id":
            if "interface" not in arg.attrib:
                length_args.append(arg.attrib["name"] + "_iface_name")
        elif arg.attrib["type"] == "string":
            length_args.append(arg.attrib["name"])
        elif arg.attrib["type"] == "array":
            length_args.append(arg.attrib["name"])
        elif arg.attrib["type"] in ("fd", "uint", "int", "object", "fixed"):
            pass
        else:
            raise NotImplementedError(arg.attrib)

    sig = []
    lines = []
    ret = []

    for i, arg in enumerate(method):
        if arg.tag != "arg":
            continue
        allow_null = "allow-null" in arg.attrib and arg.attrib["allow-null"] == "true"

        if arg.attrib["type"] == "object":
            lines.append("    let arg{} = parse_obj(&mut msg)?;".format(i))
            sig.append("ObjId")
            ret.append("arg{}".format(i))
        elif arg.attrib["type"] == "new_id":
            if "interface" in arg.attrib:
                lines.append("    let arg{} = parse_obj(&mut msg)?;".format(i))
                ret.append("arg{}".format(i))
                sig.append("ObjId")
            else:
                lines.append(
                    "    let arg{}_iface_name = parse_string(&mut msg)?.ok_or(PARSE_ERROR)?;".format(
                        i
                    )
                )
                lines.append("    let arg{}_version = parse_u32(&mut msg)?;".format(i))
                lines.append("    let arg{} = parse_obj(&mut msg)?;".format(i))
                sig.append("&'a [u8]")
                sig.append("u32")
                sig.append("ObjId")
                ret.append("arg{}_iface_name".format(i))
                ret.append("arg{}_version".format(i))
                ret.append("arg{}".format(i))
        elif arg.attrib["type"] in ("int", "fixed"):
            lines.append("    let arg{} = parse_i32(&mut msg)?;".format(i))
            sig.append("i32")
            ret.append("arg{}".format(i))
        elif arg.attrib["type"] == "uint":
            lines.append("    let arg{} = parse_u32(&mut msg)?;".format(i))
            sig.append("u32")
            ret.append("arg{}".format(i))
        elif arg.attrib["type"] == "string":
            if allow_null:
                lines.append("    let arg{} = parse_string(&mut msg)?;".format(i))
                sig.append("Option<&'a [u8]>")
            else:
                lines.append(
                    "    let arg{} = parse_string(&mut msg)?.ok_or(PARSE_ERROR)?;".format(
                        i
                    )
                )
                sig.append("&'a [u8]")
            ret.append("arg{}".format(i))
        elif arg.attrib["type"] == "array":
            lines.append("    let arg{} = parse_array(&mut msg)?;".format(i))
            sig.append("&'a [u8]")
            ret.append("arg{}".format(i))
        elif arg.attrib["type"] == "fd":
            pass
        else:
            raise NotImplementedError(arg.attrib)

    tail_prefix = "" if not sig else "mut "
    paren = (lambda x: "(" + x + ")") if len(sig) != 1 else (lambda x: x)
    write(
        "pub fn parse_{}<'a>({}msg: &'a [u8]) -> Result<{}, &'static str> {{".format(
            meth_name, tail_prefix, paren(", ".join(sig))
        )
    )
    if sig:
        write("    msg = msg.get(8..).ok_or(PARSE_ERROR)?;")
        for l in lines:
            write(l)
        write("    if !msg.is_empty() { return Err(PARSE_ERROR); }")
    else:
        write("    if msg.len() != 8 { return Err(PARSE_ERROR); }")
    write("    Ok(" + paren(", ".join(ret)) + ")")
    write("}")


def write_dense_enum(enum_name, enum_entries, with_try, write):
    write("pub enum " + enum_name + " {")
    for i, name in enumerate(enum_entries):
        write("    " + name + ",")
    write("}")

    write(
        "const {}_LIST : &[{}; {}] = &[".format(
            enum_name.upper(), enum_name, len(enum_entries)
        )
    )
    for i, name in enumerate(enum_entries):
        write("    " + name + ",")
    write("];")

    if with_try:
        write("impl TryFrom<usize> for " + enum_name + " {")
        write("    type Error = ();")
        write("    fn try_from(v: usize) -> Result<Self, Self::Error> {")
        write("        {}_LIST.get(v).copied().ok_or(())".format(enum_name.upper()))
        write("    }")
        write("}")


def write_enum(enum_name, enum_entries, enum_values, with_try, write):
    write("pub enum " + enum_name + " {")
    for i, (name, value) in enumerate(zip(enum_entries, enum_values)):
        write("    " + name + " = " + str(value) + ",")
    write("}")

    if with_try:
        write("impl TryFrom<u32> for " + enum_name + " {")
        write("    type Error = ();")
        write("    fn try_from(v: u32) -> Result<Self, Self::Error> {")
        write("         Ok(match v {")
        for i, (name, value) in enumerate(zip(enum_entries, enum_values)):
            write("            {} => {}::{},".format(value, enum_name, name))
        write("            _ => return Err(()),")
        write("        })")
        write("    }")
        write("}")


def process_interface(interface, uid, write):
    iface_name = interface.attrib["name"]
    iface_version = interface.attrib["version"]
    evts = []
    reqs = []
    for thing in interface:
        if thing.tag == "event" or thing.tag == "request":
            signature = get_signature(thing)
            destructor = "type" in thing.attrib and thing.attrib["type"] == "destructor"
            dst = evts if thing.tag == "event" else reqs
            meth_num = len(dst)
            name = thing.attrib["name"]
            dst.append((name, signature, destructor))

            meth_name = (
                ("evt" if thing.tag == "event" else "req")
                + "_"
                + iface_name
                + "_"
                + name
            )
            if (iface_name, name) in handled_funcs:
                write_method_write(meth_name, meth_num, thing, write)
                write_method_length(meth_name, thing, write)
                write_method_parse(meth_name, thing, write)

                write(
                    "pub const {} : MethodId = {}({});".format(
                        "OPCODE_" + iface_name.upper() + "_" + name.upper(),
                        "MethodId::Event"
                        if thing.tag == "event"
                        else "MethodId::Request",
                        len(dst) - 1,
                    )
                )
                handled_funcs.remove((iface_name, name))

        elif thing.tag == "enum":
            enum_name = iface_name + "_" + thing.attrib["name"]
            names = []
            values = []
            for elt in thing:
                if elt.tag == "entry":
                    name = unsnake(elt.attrib["name"])
                    if name.isnumeric():
                        name = "Item" + name
                    names.append(name)
                    values.append(elt.attrib["value"])

            do_try = (iface_name, thing.attrib["name"]) in handled_enums
            if do_try:
                handled_enums.remove((iface_name, thing.attrib["name"]))
                write("#[derive(Debug,Clone,Copy,PartialEq,Eq)]")
                write_enum(unsnake(enum_name), names, values, do_try, write)

    write("const " + "DATA_" + iface_name.upper() + ": WaylandData = WaylandData {")

    write('    name: "' + iface_name + '",')
    if evts:
        write("    evts: &[")
        for name, sig, destructor in evts:
            write("        WaylandMethod {")
            write('            name: "' + name + '",')
            write("            sig: " + sig + ",")
            write("            destructor: " + str(destructor).lower() + ",")
            write("        },")
        write("    ],")
    else:
        write("    evts: &[],")
    if reqs:
        write("    reqs: &[")
        for name, sig, destructor in reqs:
            write("        WaylandMethod {")
            write('            name: "' + name + '",')
            write("            sig: " + sig + ",")
            write("            destructor: " + str(destructor).lower() + ",")
            write("        },")
        write("    ],")
    else:
        write("    reqs: &[],")
    write("    version: " + iface_version + ",")
    write("};")
    write(
        "pub const "
        + iface_name.upper()
        + ": &[u8] = DATA_"
        + iface_name.upper()
        + ".name.as_bytes();"
    )

    if False:
        if evts:
            write_enum(
                unsnake(iface_name) + "EvtIDs",
                [unsnake(name) for name, _, _ in evts],
                list(range(len(evts))),
                False,
                write,
            )
        if reqs:
            write_enum(
                unsnake(iface_name) + "ReqIDs",
                [unsnake(name) for name, _, _ in reqs],
                list(range(len(reqs))),
                False,
                write,
            )

    return iface_name


if __name__ == "__main__":
    protocols = sorted(os.listdir(path="protocols"))
    if not all(map(lambda x: x.endswith(".xml"), protocols)):
        print("Not all files in protocols/ are XML files:", protocols)
        quit()

    with open("src/wayland_gen.rs", "w") as output:

        def write(*x):
            print(*x, file=output)

        write(header)

        interfaces = []
        uid = 0
        for protocol_file in protocols:
            root = xml.etree.ElementTree.parse("protocols/" + protocol_file).getroot()
            for interface in root:
                if interface.tag == "interface":
                    interfaces.append(process_interface(interface, uid, write))
                    uid += 1

        write("#[repr(u8)]")
        write("#[derive(Debug,Clone,Copy)]")
        write_dense_enum(
            "WaylandInterface",
            [unsnake(x) for x in sorted(interfaces)],
            True,
            write,
        )

        write(
            "pub const INTERFACE_TABLE : &[WaylandData; {}] = &[".format(
                len(interfaces)
            )
        )
        for i, intf in enumerate(sorted(interfaces)):
            write("    {},".format("DATA_" + intf.upper()))
        write("];")

    if handled_enums or handled_funcs:
        raise Exception("Unhandled: {} {}".format(handled_funcs, handled_enums))

    subprocess.call(["rustfmt", "src/wayland_gen.rs"])
