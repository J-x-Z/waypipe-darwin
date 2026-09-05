#![allow(missing_docs)]

fn depfile_to_cargo(path: &std::path::Path) {
    use std::io::Read;

    let mut depfile = std::fs::File::open(path).unwrap();
    let mut data = Vec::<u8>::new();
    depfile.read_to_end(&mut data).unwrap();
    // depfile contains the dependencies, in Make-style. Spaces in paths are escaped with '\ ',
    // and backslashes with '\\'. Other escaped chars (newlines, control characters) may break.
    let mut unescaped = Vec::new();
    let mut chunks: Vec<String> = Vec::new();
    assert!(!data.contains(&0));
    let mut scan = data.into_iter();
    while let Some(c) = scan.next() {
        if c == b'\\' {
            // TODO: how does Cargo handle escapes in path names? Or invalid utf8?
            let d = scan.next().unwrap();
            match d {
                b' ' => unescaped.push(b' '),
                b'\\' => unescaped.push(b'\\'),
                _ => panic!(),
            }
        } else if c == b' ' {
            chunks.push(std::str::from_utf8(&unescaped[..]).unwrap().into());
            unescaped.clear();
        } else {
            unescaped.push(c);
        }
    }
    chunks.push(std::str::from_utf8(&unescaped[..]).unwrap().into());

    for file in chunks.iter().skip(1) {
        println!("cargo:rerun-if-changed={}", file);
    }
}

fn main() {
    use std::ffi::{OsStr, OsString};

    let libavutil = pkg_config::probe_library("libavutil").unwrap();
    let libavcodec = pkg_config::probe_library("libavcodec").unwrap();

    let mut includes = Vec::new();
    includes.extend_from_slice(&libavutil.include_paths[..]);
    includes.extend_from_slice(&libavcodec.include_paths[..]);

    let functions = &[
        "av_buffer_ref",
        "av_buffer_unref",
        "av_dict_free",
        "av_dict_set",
        "av_frame_alloc",
        "av_frame_free",
        "av_free",
        "av_get_pix_fmt_name",
        "av_hwdevice_ctx_alloc",
        "av_hwdevice_ctx_init",
        "av_hwdevice_get_hwframe_constraints",
        "av_hwframe_ctx_alloc",
        "av_hwframe_ctx_init",
        "av_hwframe_get_buffer",
        "av_log_default_callback",
        "av_log_set_callback",
        "av_log_set_level",
        "av_malloc",
        "av_new_packet",
        "av_packet_alloc",
        "av_packet_free",
        "av_strerror",
        "avcodec_alloc_context3",
        "avcodec_align_dimensions2",
        "avcodec_find_decoder_by_name",
        "avcodec_find_encoder_by_name",
        "avcodec_free_context",
        "avcodec_get_hw_frames_parameters",
        "avcodec_open2",
        "avcodec_receive_frame",
        "avcodec_receive_packet",
        "avcodec_send_frame",
        "avcodec_send_packet",
    ];

    let types = &[
        "AVFrame",
        "AVHWDeviceContext",
        "AVHWFramesContext",
        "AVPacket",
        "AVRational",
        "AVVkFrame",
        "AVVulkanDeviceContext",
        "AVVulkanFramesContext",
    ];

    let vars = &[
        "AV_LOG_TRACE",
        "AV_LOG_VERBOSE",
        "AV_LOG_INFO",
        "AV_LOG_WARNING",
        "AV_NUM_DATA_POINTERS",
        "LIBAVUTIL_VERSION_MAJOR",
        "LIBAVCODEC_VERSION_MAJOR",
    ];

    let bindgen = "bindgen";

    let out_path = std::path::Path::new(&std::env::var_os("OUT_DIR").unwrap()).join("bindings.rs");
    let dep_path = std::path::Path::new(&std::env::var_os("OUT_DIR").unwrap()).join("depfile");

    let mut args: Vec<&OsStr> = vec![
        OsStr::new("--dynamic-loading"),
        OsStr::new("ffmpeg"),
        OsStr::new("--dynamic-link-require-all"),
        OsStr::new("--rust-target"),
        OsStr::new("1.59"),
        OsStr::new("--no-doc-comments"),
        OsStr::new("--depfile"),
        dep_path.as_os_str(),
        OsStr::new("--output"),
        out_path.as_os_str(),
    ];

    for f in functions.iter() {
        args.push(OsStr::new("--allowlist-function"));
        args.push(OsStr::new(*f));
    }
    for f in vars.iter() {
        args.push(OsStr::new("--allowlist-var"));
        args.push(OsStr::new(*f));
    }
    for f in types.iter() {
        args.push(OsStr::new("--allowlist-type"));
        args.push(OsStr::new(*f));
    }
    args.push(OsStr::new("wrapper.h"));
    args.push(OsStr::new("--"));
    let inc_vec: Vec<OsString> = includes
        .iter()
        .map(|x| {
            let mut s = OsString::from("-I");
            s.push(x);
            s
        })
        .collect();
    for x in inc_vec.iter() {
        args.push(OsStr::new(x));
    }
    let mut child = std::process::Command::new(bindgen)
        .args(args)
        .spawn()
        .unwrap();
    let exit_status = child.wait().unwrap();
    assert!(exit_status.success());

    depfile_to_cargo(&dep_path);
}
