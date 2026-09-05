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

    let lib = pkg_config::probe_library("libzstd").unwrap();

    println!("cargo:rustc-link-lib=zstd");

    let mut includes = Vec::new();
    includes.extend_from_slice(&lib.include_paths[..]);

    let functions: &[&str] = &[
        "ZSTD_createCCtx",
        "ZSTD_createDCtx",
        "ZSTD_freeCCtx",
        "ZSTD_freeDCtx",
        "ZSTD_CCtx_setParameter",
        "ZSTD_compressBound",
        "ZSTD_compress2",
        "ZSTD_isError",
        "ZSTD_decompressDCtx",
    ];

    let types: &[&str] = &["ZSTD_CCtx", "ZSTD_DCtx", "ZSTD_cParameter"];

    let vars: &[&str] = &[];

    let bindgen = "bindgen";

    let out_path = std::path::Path::new(&std::env::var_os("OUT_DIR").unwrap()).join("bindings.rs");
    let dep_path = std::path::Path::new(&std::env::var_os("OUT_DIR").unwrap()).join("depfile");

    let mut args: Vec<&OsStr> = vec![
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
