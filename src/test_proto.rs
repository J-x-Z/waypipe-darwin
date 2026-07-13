/* SPDX-License-Identifier: GPL-3.0-or-later */
/*! Protocol test runner and framework.
 *
 * Emulate a Wayland client/compositor pair and verify that Waypipe properly
 * forwards messages and file descriptors.
 *
 * This is needed to:
 * - Properly capture debug output from processes under test
 * - Run Waypipe instances as individual processes instead of threads (to avoid
 *   possible bugs if Vulkan validation layers are used for independent instances)
 * - Test different versions of Waypipe against each other.
 * - Break out test variants by the Vulkan physical device being used
 */

use clap::{value_parser, Arg, ArgAction, Command as ClapCommand};
use nix::libc;
use nix::sys::wait::WaitStatus;
use nix::sys::{signal, socket, time, wait};
use nix::{errno::Errno, fcntl, poll, unistd};
use std::collections::{BTreeMap, BTreeSet};
use std::ffi::{OsStr, OsString};
use std::io::{IoSlice, IoSliceMut, Write};
use std::os::fd::{AsFd, AsRawFd, BorrowedFd, FromRawFd, OwnedFd, RawFd};
use std::process::{Child, Command, ExitCode, Stdio};
use std::sync::atomic::AtomicBool;
use std::sync::Arc;
use std::time::{Duration, Instant};

#[cfg(feature = "dmabuf")]
use mainloop::dmabuf::*;
use mainloop::kernel::*;
use mainloop::platform::*;
use mainloop::tag;
use mainloop::util::*;
use mainloop::wayland::*;
use mainloop::wayland_gen::*;

/** Test result codes for which error-type control flow is not needed. */
#[derive(Debug)]
enum StatusOk {
    /** Corresponds to [TestCategory::Pass] */
    Pass,
    /** Corresponds to [TestCategory::Skipped] */
    Skipped,
}
/** Test result codes which, like errors, typically are worth stopping a test over. */
#[derive(Debug)]
enum StatusBad {
    /** Corresponds to [TestCategory::Unclear] */
    Unclear(String),
    /** Corresponds to [TestCategory::Fail] */
    Fail(String),
}
/** Result of a test, organized so that StatusBad can be progated with ? */
type TestResult = Result<StatusOk, StatusBad>;

/** The result of a test. */
#[derive(Debug)]
enum TestCategory {
    /** Test passed */
    Pass,
    /** Test was skipped. */
    Skipped,
    /** Test failed */
    Fail,
    /** Something was broken, but it may be an issue in a library used by Waypipe */
    Unclear,
}
/** 77 is the automake return code for a skipped test */
const EXITCODE_SKIPPED: u8 = 77;
/** 99 is the automake return code for a hard error (failure of test set up,
 * segfault, failure of an external library, or something else very unexpected) */
const EXITCODE_UNCLEAR: u8 = 99;

/** Specifications for a filter of test names. */
struct Filter<'a> {
    substrings: &'a [&'a str],
    exact: bool,
}

/** Basic parameters needed to run a test. */
struct TestInfo<'a> {
    /** The name of the test. */
    test_name: &'a str,
    /** File to execute (posix_spawnp) for the `waypipe client` connection handling instance. */
    waypipe_client: &'a OsStr,
    /** File to execute (posix_spawnp) for the `waypipe server` connection handling instance. */
    waypipe_server: &'a OsStr,
}

/** A constant that saves some code when allocating sequentual object ids */
const ID_SEQUENCE: [ObjId; 12] = [
    ObjId(1),
    ObjId(2),
    ObjId(3),
    ObjId(4),
    ObjId(5),
    ObjId(6),
    ObjId(7),
    ObjId(8),
    ObjId(9),
    ObjId(10),
    ObjId(11),
    ObjId(12),
];

/** Write messages to the Wayland connection, followed by a test message that should directly pass through;
 * possibly stopping early if the connection is closed.
 *
 * Returns: the messages received on the other end, minus the test message; if there was an error details
 * will be returned */
fn test_interact(
    prog: &OwnedFd,
    comp: &OwnedFd,
    write_to_prog: bool,
    data: &[u8],
    fds: &[&OwnedFd],
) -> (Vec<Vec<u8>>, Vec<OwnedFd>, Option<(ObjId, u32, String)>) {
    let mut end_msg = [0_u8; 8];
    let mut end_msg_view: &mut [u8] = &mut end_msg;
    write_header(&mut end_msg_view, ObjId(u32::MAX), 8, 0, 0);

    let mut nbytes_sent: usize = 0;
    let mut nfds_sent: usize = 0;
    let net_len = data.len() + end_msg.len();

    let raw_fds: Vec<i32> = fds.iter().map(|x| x.as_raw_fd()).collect();

    /* Not: libwayland sends up to 28 fds per byte */
    const MAX_FDS_PER_BYTE: usize = 32;
    assert!(data.len() >= fds.len().div_ceil(MAX_FDS_PER_BYTE));

    struct ReadState {
        data: Vec<u8>,
        rmsgs: Vec<Vec<u8>>,
        fds: Vec<OwnedFd>,
        eof: bool,
    }

    let mut recv_prog = ReadState {
        data: Vec::new(),
        fds: Vec::new(),
        rmsgs: Vec::new(),
        eof: false,
    };
    let mut recv_comp = ReadState {
        data: Vec::new(),
        fds: Vec::new(),
        rmsgs: Vec::new(),
        eof: false,
    };

    let start = Instant::now();
    let timeout = Duration::from_secs(1);
    let mut err: Option<(ObjId, u32, String)> = None;
    'outer: loop {
        let current = Instant::now();
        let elapsed = current.duration_since(start);
        if elapsed >= timeout {
            panic!("timeout: {:?}", elapsed);
        }
        let remaining = time::TimeSpec::from_duration(timeout.saturating_sub(elapsed));

        let mut pfds = Vec::new();
        let mut recvs: Vec<&mut ReadState> = Vec::new();
        let writing = nbytes_sent < net_len;
        if !recv_prog.eof {
            pfds.push(poll::PollFd::new(
                prog.as_fd(),
                if writing && write_to_prog {
                    poll::PollFlags::POLLIN | poll::PollFlags::POLLOUT
                } else {
                    poll::PollFlags::POLLIN
                },
            ));
            recvs.push(&mut recv_prog);
        }
        if !recv_comp.eof {
            pfds.push(poll::PollFd::new(
                comp.as_fd(),
                if writing && !write_to_prog {
                    poll::PollFlags::POLLIN | poll::PollFlags::POLLOUT
                } else {
                    poll::PollFlags::POLLIN
                },
            ));
            recvs.push(&mut recv_comp);
        }

        /* Connections should not close before either error or end message is received */
        assert!(
            !pfds.is_empty(),
            "connections should not close before error or end message received"
        );

        let res = system_poll(&mut pfds, Some(remaining), None);
        if let Err(e) = res {
            assert!(e == Errno::EINTR || e == Errno::EAGAIN);
        }

        for (pfd, recv) in pfds.into_iter().zip(recvs.into_iter()) {
            let evt = pfd.revents().unwrap();

            if evt.contains(poll::PollFlags::POLLIN) {
                assert!(!recv.eof);
                let mut tmp = vec![0u8; 16384];
                let mut iovs = [IoSliceMut::new(&mut tmp)];
                let mut cmsg_fds = nix::cmsg_space!([RawFd; 32]);

                let r = socket::recvmsg::<socket::UnixAddr>(
                    pfd.as_fd().as_raw_fd(),
                    &mut iovs,
                    Some(&mut cmsg_fds),
                    socket::MsgFlags::empty(),
                );
                match r {
                    Ok(resp) => {
                        for msg in resp.cmsgs().unwrap() {
                            match msg {
                                socket::ControlMessageOwned::ScmRights(tfds) => {
                                    for f in &tfds {
                                        assert!(*f != -1);
                                        recv.fds.push(unsafe {
                                            // SAFETY: fd was just created, checked valid,
                                            // and is recorded nowhere else
                                            OwnedFd::from_raw_fd(*f)
                                        });
                                    }
                                }
                                _ => {
                                    panic!("Unexpected control message");
                                }
                            }
                        }

                        let nbytes = resp.bytes;
                        recv.data.extend_from_slice(&tmp[..nbytes]);

                        recv.eof = nbytes == 0;
                    }
                    Err(nix::errno::Errno::ECONNRESET) => {
                        recv.eof = true;
                    }
                    Err(nix::errno::Errno::EINTR) | Err(nix::errno::Errno::EAGAIN) => (),
                    Err(x) => {
                        panic!("Error reading from socket: {:?}", x)
                    }
                }

                /* Extract any complete messages from stream */
                let mut tail: &[u8] = &recv.data;
                while tail.len() >= 8 {
                    let (_obj_id, length, _opcode) = parse_wl_header(tail);
                    assert!(length >= 8 && length % 4 == 0);
                    if tail.len() >= length {
                        let (msg, nxt) = tail.split_at(length);
                        recv.rmsgs.push(msg.into());
                        tail = nxt;
                    }
                }
                recv.data.drain(..(recv.data.len() - tail.len()));
            } else if evt.contains(poll::PollFlags::POLLHUP) {
                recv.eof = true;
            } else if evt.contains(poll::PollFlags::POLLERR) {
                panic!("unexpected pollerr");
            }

            if evt.contains(poll::PollFlags::POLLOUT) {
                /* Only the writing side checks for POLLOUT, so write messages here */
                let iovs_long = [
                    IoSlice::new(&data[std::cmp::min(nbytes_sent, data.len())..]),
                    IoSlice::new(&end_msg[(std::cmp::max(nbytes_sent, data.len()) - data.len())..]),
                ];
                let iovs_short = [IoSlice::new(if nbytes_sent >= data.len() {
                    &end_msg[nbytes_sent - data.len()..(nbytes_sent - data.len() + 1)]
                } else {
                    &data[nbytes_sent..(nbytes_sent + 1)]
                })];
                let sfds =
                    &raw_fds[nfds_sent..std::cmp::min(fds.len(), nfds_sent + MAX_FDS_PER_BYTE)];
                let short_transfer = fds.len() > nfds_sent + MAX_FDS_PER_BYTE;
                let iovs: &[IoSlice] = if short_transfer {
                    &iovs_short
                } else {
                    &iovs_long
                };
                let cmsgs = [socket::ControlMessage::ScmRights(sfds)];

                let r = nix::sys::socket::sendmsg::<()>(
                    pfd.as_fd().as_raw_fd(),
                    iovs,
                    if sfds.is_empty() { &[] } else { &cmsgs },
                    nix::sys::socket::MsgFlags::empty(),
                    None,
                );
                match r {
                    Ok(s) => {
                        nbytes_sent += s;
                        nfds_sent += sfds.len();
                    }
                    Err(Errno::EINTR) | Err(Errno::EAGAIN) => {
                        println!("eintr");
                    }
                    Err(Errno::EPIPE) | Err(Errno::ECONNRESET) => {
                        recv.eof = true;
                    }
                    Err(e) => {
                        panic!("{:?}", e);
                    }
                }
            }
        }

        let opp_recv = if write_to_prog {
            &mut recv_comp
        } else {
            &mut recv_prog
        };
        for (i, msg) in opp_recv.rmsgs.iter().enumerate() {
            let (obj_id, _length, opcode) = parse_wl_header(msg);
            if obj_id == ObjId(u32::MAX) && opcode == 0 {
                /* encountered pass-through message; stop, are done */
                opp_recv.rmsgs.remove(i);
                break 'outer;
            }
        }

        for msg in &recv_prog.rmsgs {
            let (obj_id, _length, opcode) = parse_wl_header(msg);
            if obj_id == ObjId(1) && MethodId::Event(opcode) == OPCODE_WL_DISPLAY_ERROR {
                /* encountered error message; stop and return error */
                let (obj, code, msg) = parse_evt_wl_display_error(msg).unwrap();
                err = Some((obj, code, String::from_utf8(Vec::from(msg)).unwrap()));
                break 'outer;
            }
        }
    }

    if write_to_prog {
        assert!(recv_prog.fds.is_empty());
        assert!((recv_prog.rmsgs.len() == (err.is_some() as usize)) && recv_prog.data.is_empty());
        (recv_comp.rmsgs, recv_comp.fds, err)
    } else {
        assert!(recv_comp.fds.is_empty());
        assert!(recv_comp.rmsgs.is_empty() && recv_comp.data.is_empty());
        (recv_prog.rmsgs, recv_prog.fds, err)
    }
}

/** Write messages to the Wayland connection, followed by a test message that should directly pass through;
 * possibly stopping early if the connection is closed. (Note: in that case, test_read_msgs should
 * capture an error message). */
fn test_write_msgs(socket: &OwnedFd, data: &[u8], fds: &[&OwnedFd]) {
    let mut end_msg = [0_u8; 8];
    let mut end_msg_view: &mut [u8] = &mut end_msg;
    write_header(&mut end_msg_view, ObjId(u32::MAX), 8, 0, 0);

    let mut nbytes_sent: usize = 0;
    let mut nfds_sent: usize = 0;
    let net_len = data.len() + end_msg.len();

    let raw_fds: Vec<i32> = fds.iter().map(|x| x.as_raw_fd()).collect();
    assert!(data.len() >= fds.len().div_ceil(32));

    let start = Instant::now();
    let timeout = Duration::from_secs(1);

    while nbytes_sent < net_len {
        let iovs_long = [
            IoSlice::new(&data[std::cmp::min(nbytes_sent, data.len())..]),
            IoSlice::new(&end_msg[(std::cmp::max(nbytes_sent, data.len()) - data.len())..]),
        ];
        let iovs_short = [IoSlice::new(if nbytes_sent >= data.len() {
            &end_msg[nbytes_sent - data.len()..(nbytes_sent - data.len() + 1)]
        } else {
            &data[nbytes_sent..(nbytes_sent + 1)]
        })];
        let fds = &raw_fds[nfds_sent..std::cmp::min(fds.len(), nfds_sent + 32)];

        let short_transfer = fds.len() - nfds_sent >= 32;
        let iovs: &[IoSlice] = if short_transfer {
            &iovs_short
        } else {
            &iovs_long
        };
        let cmsgs = [socket::ControlMessage::ScmRights(fds)];

        let mut pfd = [poll::PollFd::new(socket.as_fd(), poll::PollFlags::POLLOUT)];
        let current = Instant::now();
        let elapsed = current.duration_since(start);
        if elapsed >= timeout {
            panic!("timeout: {:?}", elapsed);
        }
        let remaining = time::TimeSpec::from_duration(timeout.saturating_sub(elapsed));
        let res = system_poll(&mut pfd, Some(remaining), None);
        if let Err(e) = res {
            assert!(e == Errno::EINTR || e == Errno::EAGAIN);
        }
        let evts = pfd[0].revents().unwrap();
        if evts.contains(poll::PollFlags::POLLHUP) {
            println!("Pollhup on write");
            return;
        }
        if evts.contains(poll::PollFlags::POLLERR) {
            panic!("Unexpected pollerr");
        }
        if !evts.contains(poll::PollFlags::POLLOUT) {
            continue;
        }

        let r = nix::sys::socket::sendmsg::<()>(
            socket.as_raw_fd(),
            iovs,
            if fds.is_empty() { &[] } else { &cmsgs },
            nix::sys::socket::MsgFlags::empty(),
            None,
        );
        match r {
            Ok(s) => {
                nbytes_sent += s;
                if short_transfer {
                    nfds_sent += 32;
                } else {
                    nfds_sent = fds.len();
                }
            }
            Err(Errno::EINTR) | Err(Errno::EAGAIN) => {
                println!("eintr");
            }
            Err(e) => {
                panic!("{:?}", e);
            }
        }
    }
}

/** Buffer holding data read from a socket and whether the connection has closed. */
struct ReadRecv {
    data: Vec<u8>,
    fds: Vec<OwnedFd>,
    eof: bool,
}

/** Read data/fds from a socket, and return true on EOF. */
fn test_read_from_socket(socket: BorrowedFd, recv: &mut ReadRecv) -> bool {
    let mut tmp = vec![0u8; 16384];
    let mut iovs = [IoSliceMut::new(&mut tmp)];
    let mut cmsg_fds = nix::cmsg_space!([RawFd; 32]);

    let r = socket::recvmsg::<socket::UnixAddr>(
        socket.as_raw_fd(),
        &mut iovs,
        Some(&mut cmsg_fds),
        socket::MsgFlags::empty(),
    );
    match r {
        Ok(resp) => {
            for msg in resp.cmsgs().unwrap() {
                match msg {
                    socket::ControlMessageOwned::ScmRights(tfds) => {
                        for f in &tfds {
                            assert!(*f != -1);
                            recv.fds.push(unsafe {
                                // SAFETY: fd was just created, checked valid,
                                // and is recorded nowhere else
                                OwnedFd::from_raw_fd(*f)
                            });
                        }
                    }
                    _ => {
                        panic!("Unexpected control message");
                    }
                }
            }

            let nbytes = resp.bytes;
            recv.data.extend_from_slice(&tmp[..nbytes]);
            nbytes == 0
        }
        Err(nix::errno::Errno::ECONNRESET) => true,
        Err(nix::errno::Errno::EINTR) | Err(nix::errno::Errno::EAGAIN) => false,
        Err(x) => {
            panic!("Error reading from socket: {:?}", x)
        }
    }
}

/** Read messages from the Wayland connection, until an error is returned or the test message is found.
 * If opposite_fd is not None, then it is the program-side socket and will receive wl_display_error. */
fn test_read_msgs(
    socket: &OwnedFd,
    opposite_socket: Option<&OwnedFd>,
) -> (Vec<Vec<u8>>, Vec<OwnedFd>, Option<(ObjId, u32, String)>) {
    let mut msgs = Vec::new();
    let mut fds = Vec::new();

    let mut pfds = Vec::new();
    let mut recv = Vec::new();
    pfds.push(poll::PollFd::new(socket.as_fd(), poll::PollFlags::POLLIN));
    recv.push(ReadRecv {
        data: Vec::new(),
        fds: Vec::new(),
        eof: false,
    });
    if let Some(f) = opposite_socket {
        pfds.push(poll::PollFd::new(f.as_fd(), poll::PollFlags::POLLIN));
        recv.push(ReadRecv {
            data: Vec::new(),
            fds: Vec::new(),
            eof: false,
        });
    }

    let start = Instant::now();
    let timeout = Duration::from_secs(1);
    let mut err: Option<(ObjId, u32, String)> = None;
    'outer: loop {
        let current = Instant::now();
        let elapsed = current.duration_since(start);
        if elapsed >= timeout {
            panic!("timeout: {:?}", elapsed);
        }
        let remaining = time::TimeSpec::from_duration(timeout.saturating_sub(elapsed));
        let res = system_poll(&mut pfds, Some(remaining), None);
        if let Err(e) = res {
            assert!(e == Errno::EINTR || e == Errno::EAGAIN);
        }

        for (p, r) in pfds.iter().zip(recv.iter_mut()) {
            let evt = p.revents().unwrap();

            if evt.contains(poll::PollFlags::POLLIN) {
                let eof = test_read_from_socket(p.as_fd(), r);
                if eof {
                    r.eof = true;
                }
            }
            if evt.contains(poll::PollFlags::POLLERR) {
                panic!("unexpected pollerr");
            }
        }

        if !recv[0].fds.is_empty() {
            fds.append(&mut recv[0].fds);
        }

        while recv[0].data.len() >= 8 {
            let (obj_id, length, opcode) = parse_wl_header(&recv[0].data);
            assert!(length >= 8 && length % 4 == 0);

            if obj_id == ObjId(u32::MAX) && opcode == 0 {
                /* encountered pass-through message; we are done */
                break 'outer;
            }

            if recv[0].data.len() >= length {
                let data: Vec<u8> = recv[0].data.drain(..length).collect();

                if opposite_socket.is_none() {
                    if obj_id == ObjId(1) && MethodId::Event(opcode) == OPCODE_WL_DISPLAY_ERROR {
                        let (obj, code, msg) = parse_evt_wl_display_error(&data).unwrap();
                        err = Some((obj, code, String::from_utf8(Vec::from(msg)).unwrap()));
                        break 'outer;
                    }
                }

                msgs.push(data);
            }
        }

        if opposite_socket.is_some() {
            assert!(recv[1].fds.is_empty());
            if recv[1].data.len() >= 8 {
                let (obj_id, length, opcode) = parse_wl_header(&recv[1].data);
                if recv[1].data.len() >= length {
                    let data: Vec<u8> = recv[1].data.drain(..length).collect();
                    if obj_id == ObjId(1) && MethodId::Event(opcode) == OPCODE_WL_DISPLAY_ERROR {
                        let (obj, code, msg) = parse_evt_wl_display_error(&data).unwrap();
                        err = Some((obj, code, String::from_utf8(Vec::from(msg)).unwrap()));
                        break 'outer;
                    } else {
                        panic!("unexpected message on program side");
                    }
                }
            }
        }
    }

    (msgs, fds, err)
}

/** Helper function to return a `Vec<u8>`, given a function that builds its
 * contents by writing to a `&mut &mut [u8]`. Used to build Wayland message sequences. */
fn build_msgs<F>(f: F) -> Vec<u8>
where
    F: FnOnce(&mut &mut [u8]),
{
    let len = 16384;
    let mut buf = vec![0u8; len];
    let mut rest = &mut buf[..];
    f(&mut rest);
    let nwritten = len - rest.len();
    Vec::from(&buf[..nwritten])
}

/** Return true iff the interaction result in `x` matches concatenated messages `concat`. */
fn is_plain_msgs(
    x: Result<(Vec<Vec<u8>>, Vec<OwnedFd>), (ObjId, u32, String)>,
    concat: Vec<u8>,
) -> bool {
    if let Ok((msg, fds)) = x {
        fds.is_empty() && msg.concat() == concat
    } else {
        false
    }
}

/** Things needed for the test program to interact with a linked pair of Waypipe instances. */
struct ProtocolTestContext {
    sock_prog: OwnedFd,
    sock_comp: OwnedFd,
}

impl ProtocolTestContext {
    /** Write data and fds to the program-side Waypipe instance, and return the resulting messages or error message. */
    fn prog_write(
        &mut self,
        data: &[u8],
        fds: &[&OwnedFd],
    ) -> Result<(Vec<Vec<u8>>, Vec<OwnedFd>), (ObjId, u32, String)> {
        let (msg, ofds, err) = test_interact(&self.sock_prog, &self.sock_comp, true, data, fds);
        if let Some(e) = err {
            Err(e)
        } else {
            Ok((msg, ofds))
        }
    }
    /** Write data and fds to the compositor-side Waypipe instance, and return the resulting messages or error message. */
    fn comp_write(
        &mut self,
        data: &[u8],
        fds: &[&OwnedFd],
    ) -> Result<(Vec<Vec<u8>>, Vec<OwnedFd>), (ObjId, u32, String)> {
        let (msg, ofds, err) = test_interact(&self.sock_prog, &self.sock_comp, false, data, fds);
        if let Some(e) = err {
            Err(e)
        } else {
            Ok((msg, ofds))
        }
    }
    /** Write messages to a program-side Waypipe instance, and panic if they are not passed through unchanged. */
    fn prog_write_passthrough(&mut self, data: Vec<u8>) {
        assert!(is_plain_msgs(self.prog_write(&data, &[]), data));
    }
    /** Write messages to a compositor-side Waypipe instance, and panic if they are not passed through unchanged. */
    fn comp_write_passthrough(&mut self, data: Vec<u8>) {
        assert!(is_plain_msgs(self.comp_write(&data, &[]), data));
    }
}

/** Options to be passed to an instance of Waypipe */
struct WaypipeOptions<'a> {
    wire_version: Option<u32>,
    drm_node: Option<u64>,
    device_type: RenderDeviceType,
    video: VideoSetting,
    title_prefix: &'a str,
    compression: Compression,
}

/** Construct the command to run a Waypipe connection handling process, with specified options. */
fn build_arguments(waypipe_bin: &OsStr, opts: &WaypipeOptions, is_client: bool) -> Vec<String> {
    let mut v = Vec::new();

    let cross_runner = std::env::var_os("CROSS_TARGET_RUNNER");
    let cx: &OsStr = cross_runner.as_deref().unwrap_or_default();
    /* note: CROSS_TARGET_RUNNER is typically something like 'CROSS_TARGET_RUNNER=/linux-runner aarch64' */
    for chunk in cx.as_encoded_bytes().split(|x| *x == b' ') {
        if !chunk.is_empty() {
            v.push(std::str::from_utf8(chunk).unwrap().into());
        }
    }

    v.push(waypipe_bin.to_str().unwrap().into());
    v.push("--debug".into());
    v.push("--threads=1".into());
    v.push(format!("--compress={}", opts.compression));
    if !opts.title_prefix.is_empty() {
        v.push(format!("--title-prefix={}", opts.title_prefix));
    }
    if let Some(device_id) = opts.drm_node {
        v.push(format!("--drm-node=/dev/dri/renderD{}", (device_id & 0xff)));
    } else {
        v.push("--no-gpu".into());
    }
    if matches!(opts.device_type, RenderDeviceType::Gbm) {
        v.push("--test-skip-vulkan".into());
    }
    if opts.video.format.is_some() {
        v.push(format!("--video={}", opts.video));
    }
    if let Some(ver) = opts.wire_version {
        v.push(format!("--test-wire-version={}", ver));
    }

    if is_client {
        v.push("client-conn".into());
    } else {
        v.push("server-conn".into());
    }
    v
}

/** Run a protocol test with the specified options. */
fn run_protocol_test_with_opts(
    info: &TestInfo,
    opts_client: &WaypipeOptions,
    opts_server: &WaypipeOptions,
    test_fn: &dyn Fn(ProtocolTestContext),
) -> Result<(), StatusBad> {
    let (channel1, channel2) = create_socketpair(fcntl::OFlag::O_CLOEXEC).unwrap();
    let (prog_appl1, prog_appl2) =
        create_socketpair(fcntl::OFlag::O_NONBLOCK | fcntl::OFlag::O_CLOEXEC).unwrap();
    let (prog_comp1, prog_comp2) =
        create_socketpair(fcntl::OFlag::O_NONBLOCK | fcntl::OFlag::O_CLOEXEC).unwrap();

    let client_args = build_arguments(info.waypipe_client, opts_client, true);
    let server_args = build_arguments(info.waypipe_server, opts_server, false);

    set_cloexec(&prog_comp2, false).unwrap();
    set_cloexec(&channel1, false).unwrap();
    let mut client: Child = Command::new(&client_args[0])
        .args(&client_args[1..])
        .env("WAYLAND_SOCKET", format!("{}", prog_comp2.as_raw_fd()))
        .env("WAYPIPE_CONNECTION_FD", format!("{}", channel1.as_raw_fd()))
        .stdin(Stdio::null())
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .spawn()
        .unwrap();
    drop(prog_comp2);
    drop(channel1);

    set_cloexec(&prog_appl2, false).unwrap();
    set_cloexec(&channel2, false).unwrap();
    let mut server: Child = Command::new(&server_args[0])
        .args(&server_args[1..])
        .env("WAYLAND_SOCKET", format!("{}", channel2.as_raw_fd()))
        .env(
            "WAYPIPE_CONNECTION_FD",
            format!("{}", prog_appl2.as_raw_fd()),
        )
        .stdin(Stdio::null())
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .spawn()
        .unwrap();
    drop(prog_appl2);
    drop(channel2);

    let ctx = ProtocolTestContext {
        sock_prog: prog_appl1,
        sock_comp: prog_comp1,
    };
    test_fn(ctx);

    /* Wait for processes to die */
    // todo: this needs a 1 second timeout to properly catch deadlocked processes
    let exit_c = client.wait().unwrap();
    let exit_s = server.wait().unwrap();
    if !exit_s.success() || !exit_c.success() {
        let msg = tag!(
            "Waypipe connection handlers for {} did not exit cleanly: client {} server {}",
            info.test_name,
            exit_c.success(),
            exit_s.success()
        );
        return Err(StatusBad::Fail(msg));
    }

    Ok(())
}

/** Run a protocol test with both sides of the connection using the DRM node for `device`. */
#[cfg(feature = "dmabuf")]
fn run_protocol_test_with_drm_node(
    info: &TestInfo,
    device: &RenderDevice,
    test_fn: &dyn Fn(ProtocolTestContext),
) -> Result<(), StatusBad> {
    let options = WaypipeOptions {
        wire_version: None,
        drm_node: Some(device.id),
        device_type: device.device_type,
        compression: Compression::None,
        title_prefix: "",
        video: VideoSetting::default(),
    };
    run_protocol_test_with_opts(info, &options, &options, test_fn)
}

/** Run a protocol test with default options. */
fn run_protocol_test(
    info: &TestInfo,
    test_fn: &dyn Fn(ProtocolTestContext),
) -> Result<(), StatusBad> {
    let options = WaypipeOptions {
        wire_version: None,
        drm_node: None,
        device_type: RenderDeviceType::Vulkan,
        compression: Compression::None,
        title_prefix: "",
        video: VideoSetting::default(),
    };
    run_protocol_test_with_opts(info, &options, &options, test_fn)
}

/** Try to setup a Vulkan instance and device for the given `device_id`. */
#[cfg(feature = "dmabuf")]
fn setup_vulkan(device_id: u64) -> Result<Arc<VulkanDevice>, String> {
    let instance = setup_vulkan_instance(true, &VideoSetting::default(), false, false)?
        .ok_or_else(|| tag!("Vulkan instance not available"))?;
    Ok(Arc::new(
        setup_vulkan_device_base(&instance, Some(device_id), false)?
            .ok_or_else(|| tag!("Vulkan device {} not available", device_id))?,
    ))
}

/** Return a memfd whose contents are precisely `data`. */
fn make_file_with_contents(data: &[u8]) -> Result<OwnedFd, String> {
    let local_fd =
        create_anon_file().map_err(|x| tag!("Failed to create shared-memory file: {:?}", x))?;
    unistd::ftruncate(&local_fd, data.len().try_into().unwrap())
        .map_err(|x| tag!("Failed to resize memfd: {:?}", x))?;

    let mapping = ExternalMapping::new(&local_fd, data.len(), false)?;
    copy_onto_mapping(data, &mapping, 0);

    Ok(local_fd)
}
/** Replace the contents of a memfd, whose length is already `data.len()`, with `data`. */
fn update_file_contents(fd: &OwnedFd, data: &[u8]) -> Result<(), String> {
    let mapping = ExternalMapping::new(fd, data.len(), false)?;
    copy_onto_mapping(data, &mapping, 0);
    drop(mapping);
    Ok(())
}
/** Replace the conents of a memfd with `data`, changing the file size as needed. */
fn resize_file_with_contents(fd: &OwnedFd, data: &[u8]) -> Result<(), String> {
    unistd::ftruncate(fd, data.len().try_into().unwrap())
        .map_err(|x| tag!("Failed to resize memfd: {:?}", x))?;
    let mapping = ExternalMapping::new(fd, data.len(), false)?;
    copy_onto_mapping(data, &mapping, 0);
    Ok(())
}
/** Get the contents of the file, assuming its length is `len`. May error or SIGBUS if the actual
 * length is shorter */
fn get_file_contents(fd: &OwnedFd, len: usize) -> Result<Vec<u8>, String> {
    let mapping = ExternalMapping::new(fd, len, true)?;
    let mut data = vec![0xff_u8; len];
    copy_from_mapping(&mut data, &mapping, 0);
    drop(mapping);
    Ok(data)
}

/** Return true iff the name matches the given filter.  */
fn test_is_included(name: &str, filter: &Filter) -> bool {
    if filter.substrings.is_empty() {
        return true;
    }
    for x in filter.substrings {
        if filter.exact {
            let extended_x = format!("::{}::", x);
            let extended_name = format!("::{}::", name);
            if extended_name.contains(&extended_x) {
                return true;
            }
        } else if name.contains(x) {
            return true;
        }
    }
    false
}

/** Register a single test, if the name passes the filter. */
fn register_single<'a>(
    tests: &mut Vec<(String, Box<dyn Fn(TestInfo) -> TestResult + 'a>)>,
    filter: &Filter,
    name: &str,
    func: fn(TestInfo) -> TestResult,
) {
    let ext_name = format!("proto::{}", name);
    if !test_is_included(&ext_name, filter) {
        return;
    }

    tests.push((ext_name, Box::new(func)));
}

/** Register a test instance for each device in the list, if the resulting test name passes the filter. */
fn register_per_device<'a>(
    tests: &mut Vec<(String, Box<dyn Fn(TestInfo) -> TestResult + 'a>)>,
    filter: &Filter,
    devices: &[(String, u64)],
    name: &str,
    func: fn(TestInfo, RenderDevice) -> TestResult,
) {
    for (dev_name, dev_id) in devices {
        for (tp, tpname) in [
            (RenderDeviceType::Vulkan, "vk"),
            (RenderDeviceType::Gbm, "gbm"),
        ] {
            if !cfg!(feature = "gbmfallback") && matches!(tp, RenderDeviceType::Gbm) {
                continue;
            }

            let ext_name = format!("proto::{}::{}::{}", name, dev_name, tpname);
            if !test_is_included(&ext_name, filter) {
                continue;
            }
            let m: (String, u64) = (dev_name.clone(), *dev_id);
            tests.push((
                ext_name,
                Box::new(move |info| {
                    let s = &m;
                    let dev = RenderDevice {
                        name: &s.0,
                        id: s.1,
                        device_type: tp,
                    };
                    func(info, dev)
                }),
            ));
        }
    }
}

#[derive(Clone, Copy)]
enum RenderDeviceType {
    Gbm,
    Vulkan,
}

/** Information about a render device. */
struct RenderDevice<'a> {
    #[allow(unused)]
    name: &'a str,
    id: u64,
    /** The device type for the _instances under test_ to use; test_proto itself,
     * for now, uses Vulkan to handle buffers. */
    device_type: RenderDeviceType,
}

/** List all render devices on this system. This just checks file properties
 * and does not test that they are actually usable. */
pub fn list_vulkan_device_ids() -> Vec<(String, u64)> {
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
        dev_ids.push((entry.file_name().into_string().unwrap(), rdev));
    }
    dev_ids
}

/** No-op signal handler (used to ensure SIGCHLD interrupts poll) */
extern "C" fn noop_signal_handler(_: i32) {}

/** Test to verify that a simple message exchange behaves as expected */
fn proto_basic(info: TestInfo) -> TestResult {
    run_protocol_test(&info, &|mut ctx: ProtocolTestContext| {
        let write_prog = build_msgs(|dst| {
            write_req_wl_display_get_registry(dst, ObjId(1), ObjId(2));
            write_req_wl_display_sync(dst, ObjId(1), ObjId(3));
        });
        let (resp, resp_fds) = ctx.prog_write(&write_prog, &[]).unwrap();
        assert!(resp_fds.is_empty());
        assert!(resp.concat() == write_prog);

        let write_comp = build_msgs(|dst| {
            write_evt_wl_registry_global(dst, ObjId(2), 1, WL_COMPOSITOR, 3);
            write_evt_wl_callback_done(dst, ObjId(3), 0);
        });
        assert!(is_plain_msgs(ctx.comp_write(&write_comp, &[]), write_comp));
    })?;
    Ok(StatusOk::Pass)
}

/** Test that using the base protocol version still works, for basic operations */
fn proto_base_wire(info: TestInfo) -> TestResult {
    let opts = WaypipeOptions {
        wire_version: Some(MIN_PROTOCOL_VERSION),
        drm_node: None,
        device_type: RenderDeviceType::Vulkan,
        title_prefix: "",
        compression: Compression::None,
        video: VideoSetting::default(),
    };
    run_protocol_test_with_opts(&info, &opts, &opts, &|mut ctx: ProtocolTestContext| {
        ctx.prog_write_passthrough(build_msgs(|dst| {
            write_req_wl_display_get_registry(dst, ObjId(1), ObjId(2));
            write_req_wl_display_sync(dst, ObjId(1), ObjId(3));
        }));
        ctx.comp_write_passthrough(build_msgs(|dst| {
            write_evt_wl_callback_done(dst, ObjId(3), 0);
        }));
        ctx.prog_write_passthrough(build_msgs(|dst| {
            write_req_wl_display_sync(dst, ObjId(1), ObjId(3));
        }));
        ctx.comp_write_passthrough(build_msgs(|dst| {
            write_evt_wl_callback_done(dst, ObjId(3), 0);
        }));
    })?;
    Ok(StatusOk::Pass)
}

/** Test to verify that keymap files can be transferred reliably */
fn proto_keymap(info: TestInfo) -> TestResult {
    for length in [0, 9, 4096, 300001] {
        run_protocol_test(&info, &|mut ctx: ProtocolTestContext| {
            let source_text = "test data ".as_bytes();
            let mut source_data =
                source_text.repeat(align(length, source_text.len()) / source_text.len());
            source_data.truncate(length);
            let source_fd = make_file_with_contents(&source_data).unwrap();

            let (display, registry, callback, seat, keyboard) =
                (ObjId(1), ObjId(2), ObjId(3), ObjId(4), ObjId(5));

            ctx.prog_write_passthrough(build_msgs(|dst| {
                write_req_wl_display_get_registry(dst, display, registry);
                write_req_wl_display_sync(dst, display, callback);
            }));

            ctx.comp_write_passthrough(build_msgs(|dst| {
                write_evt_wl_registry_global(dst, registry, 1, WL_SEAT, 7);
                write_evt_wl_callback_done(dst, callback, 0);
            }));

            ctx.prog_write_passthrough(build_msgs(|dst| {
                write_req_wl_registry_bind(dst, registry, 1, WL_SEAT, 7, seat);
            }));

            ctx.comp_write_passthrough(build_msgs(|dst| {
                write_evt_wl_seat_capabilities(dst, seat, 3);
            }));

            ctx.prog_write_passthrough(build_msgs(|dst| {
                write_req_wl_seat_get_keyboard(dst, seat, keyboard);
            }));

            let (msgs, mut fds) = ctx
                .comp_write(
                    &build_msgs(|dst| {
                        write_evt_wl_keyboard_keymap(
                            dst,
                            keyboard,
                            false,
                            1,
                            source_data.len() as _,
                        );
                    }),
                    &[&source_fd],
                )
                .unwrap();
            drop(source_fd);

            let (_format, keymap_length) = parse_evt_wl_keyboard_keymap(&msgs[0]).unwrap();

            let new_kb_fd = fds.remove(0);
            let new_data = get_file_contents(&new_kb_fd, keymap_length as usize).unwrap();
            assert!(
                new_data == source_data,
                "{} {:?} {:?}",
                keymap_length,
                &new_data[..new_data.len().min(1000)],
                &source_data[..source_data.len().min(1000)]
            );
        })?;
    }
    Ok(StatusOk::Pass)
}

/** Test to verify that data for wp_image_description_creator_icc_v1::set_icc_file and
 * wp_image_description_info_v1::icc_file is correctly transferred */
fn proto_icc(info: TestInfo) -> TestResult {
    run_protocol_test(&info, &|mut ctx: ProtocolTestContext| {
        let [display, registry, color_mgr, output, creator, color_output, desc, info, ..] =
            ID_SEQUENCE;

        ctx.prog_write_passthrough(build_msgs(|dst| {
            write_req_wl_display_get_registry(dst, display, registry);
        }));

        ctx.comp_write_passthrough(build_msgs(|dst| {
            write_evt_wl_registry_global(dst, registry, 1, WL_OUTPUT, 4);
            write_evt_wl_registry_global(dst, registry, 2, WP_COLOR_MANAGER_V1, 1);
        }));

        let offset: u32 = 2048;
        let length: u32 = 1024;
        let file_sz: u32 = 4096;
        assert!(file_sz >= offset + length);

        let data: Vec<u8> = (0..file_sz / 2)
            .flat_map(|x| (x as u16).to_le_bytes())
            .collect();
        let base_fd = make_file_with_contents(&data).unwrap();

        // Check that wp_image_description_creator_icc_v1::set_icc_file works
        let msg_start = build_msgs(|dst| {
            write_req_wl_registry_bind(dst, registry, 2, WP_COLOR_MANAGER_V1, 1, color_mgr);
            write_req_wl_registry_bind(dst, registry, 1, WL_OUTPUT, 4, output);
            write_req_wp_color_manager_v1_create_icc_creator(dst, color_mgr, creator);
            write_req_wp_color_manager_v1_get_output(dst, color_mgr, color_output, output);
            write_req_wp_color_management_output_v1_get_image_description(dst, color_output, desc);
            write_req_wp_image_description_v1_get_information(dst, desc, info);
        });
        let msg_fd = build_msgs(|dst| {
            write_req_wp_image_description_creator_icc_v1_set_icc_file(
                dst, creator, false, offset, length,
            );
        });
        let mut smsgs = msg_start.clone();
        smsgs.extend_from_slice(&msg_fd);

        let (mut rmsgs, mut rfds) = ctx.prog_write(&smsgs, &[&base_fd]).unwrap();
        let rmsg_fd = rmsgs.pop().unwrap();
        let fd_from_prog = rfds.pop().unwrap();
        assert!(rfds.is_empty());
        assert!(
            parse_wl_header(&rmsg_fd)
                == (
                    creator,
                    length_req_wp_image_description_creator_icc_v1_set_icc_file(),
                    OPCODE_WP_IMAGE_DESCRIPTION_CREATOR_ICC_V1_SET_ICC_FILE.code()
                )
        );
        let (roffset, rlength) =
            parse_req_wp_image_description_creator_icc_v1_set_icc_file(&rmsg_fd).unwrap();
        assert!(rlength == length);
        assert!(rmsgs.concat() == msg_start);

        let data_from_prog = get_file_contents(
            &fd_from_prog,
            (offset.checked_add(length).unwrap()) as usize,
        )
        .unwrap();
        assert!(
            data_from_prog[roffset as usize..(roffset + length) as usize]
                == data[offset as usize..(offset + length) as usize]
        );

        // Check that wp_image_description_info_v1::icc_file works
        let cmsgs = build_msgs(|dst| {
            write_evt_wp_image_description_info_v1_icc_file(dst, info, false, length);
        });
        let (rcmsgs, mut rcfds) = ctx.comp_write(&cmsgs, &[&base_fd]).unwrap();
        assert!(rcmsgs.concat() == cmsgs);
        let fd_from_comp = rcfds.pop().unwrap();
        assert!(rcfds.is_empty());
        let data_from_comp = get_file_contents(&fd_from_comp, length as usize).unwrap();
        assert!(data_from_comp == data[..length as usize]);
    })?;
    Ok(StatusOk::Pass)
}

/** Test to verify that proper replication for wlr_gamma_control_unstable_v1 */
fn proto_gamma_control(info: TestInfo) -> TestResult {
    let gamma_size: usize = 4096;
    const BYTES_PER_ENTRY: usize = 6;
    let mut source_pattern = Vec::new(); /* Pattern for a single gamma ramp channel */
    for i in 0..gamma_size {
        source_pattern.extend_from_slice(&u16::to_le_bytes(i as u16));
    }

    println!("Subtest: sending gamma ramp fd before size provided");
    run_protocol_test(&info, &|mut ctx: ProtocolTestContext| {
        let [display, registry, output, manager, gamma, ..] = ID_SEQUENCE;
        ctx.prog_write_passthrough(build_msgs(|dst| {
            write_req_wl_display_get_registry(dst, display, registry);
        }));
        ctx.comp_write_passthrough(build_msgs(|dst| {
            write_evt_wl_registry_global(dst, registry, 1, WL_OUTPUT, 4);
            write_evt_wl_registry_global(dst, registry, 1, ZWLR_GAMMA_CONTROL_MANAGER_V1, 1);
        }));
        ctx.prog_write_passthrough(build_msgs(|dst| {
            write_req_wl_registry_bind(dst, registry, 1, WL_OUTPUT, 4, output);
            write_req_wl_registry_bind(dst, registry, 1, ZWLR_GAMMA_CONTROL_MANAGER_V1, 1, manager);
            write_req_zwlr_gamma_control_manager_v1_get_gamma_control(dst, manager, gamma, output);
        }));

        let source_data = source_pattern.repeat(3);
        let source_fd = make_file_with_contents(&source_data).unwrap();

        let msg = build_msgs(|dst| {
            write_req_zwlr_gamma_control_v1_set_gamma(dst, gamma, false);
        });
        let err_msg = ctx
            .prog_write(&msg, &[&source_fd])
            .expect_err("sending ramp early should fail");
        println!(
            "Error message from sending gamma ramp fd too early: {} {} {}",
            err_msg.0, err_msg.1, err_msg.2
        );
    })?;

    let correct_length = gamma_size * BYTES_PER_ENTRY;
    let lengths = [
        /* correct size */
        gamma_size * BYTES_PER_ENTRY,
        /* oversize: OK */
        gamma_size * BYTES_PER_ENTRY * 2,
        /* slightly undersize (typically zero extended) */
        // gamma_size * BYTES_PER_ENTRY - 2,
        /* very undersize (SIGBUS) */
        // gamma_size,
    ];

    for length in lengths {
        println!(
            "Subtest: specified size {}={}*{}, provided file size {}",
            gamma_size * BYTES_PER_ENTRY,
            gamma_size,
            BYTES_PER_ENTRY,
            length
        );
        run_protocol_test(&info, &|mut ctx: ProtocolTestContext| {
            let mut source_data =
                source_pattern.repeat(align(length, source_pattern.len()) / source_pattern.len());
            source_data.truncate(length);
            let source_fd = make_file_with_contents(&source_data).unwrap();
            let [display, registry, output, manager, gamma, ..] = ID_SEQUENCE;

            ctx.prog_write_passthrough(build_msgs(|dst| {
                write_req_wl_display_get_registry(dst, display, registry);
            }));
            ctx.comp_write_passthrough(build_msgs(|dst| {
                write_evt_wl_registry_global(dst, registry, 1, WL_OUTPUT, 4);
                write_evt_wl_registry_global(dst, registry, 1, ZWLR_GAMMA_CONTROL_MANAGER_V1, 1);
            }));
            ctx.prog_write_passthrough(build_msgs(|dst| {
                write_req_wl_registry_bind(dst, registry, 1, WL_OUTPUT, 4, output);
                write_req_wl_registry_bind(
                    dst,
                    registry,
                    1,
                    ZWLR_GAMMA_CONTROL_MANAGER_V1,
                    1,
                    manager,
                );
                write_req_zwlr_gamma_control_manager_v1_get_gamma_control(
                    dst, manager, gamma, output,
                );
            }));
            ctx.comp_write_passthrough(build_msgs(|dst| {
                write_evt_zwlr_gamma_control_v1_gamma_size(
                    dst,
                    gamma,
                    gamma_size.try_into().unwrap(),
                );
            }));

            let msg = build_msgs(|dst| {
                write_req_zwlr_gamma_control_v1_set_gamma(dst, gamma, false);
            });

            let (msgs, mut fds) = ctx.prog_write(&msg, &[&source_fd]).unwrap();
            drop(source_fd);
            if length >= gamma_size * BYTES_PER_ENTRY {
                assert!(msgs.concat() == msg);
                assert!(fds.len() == 1);
                let new_gamma_fd = fds.remove(0);
                let new_data = get_file_contents(&new_gamma_fd, correct_length).unwrap();
                assert!(new_data == source_data[..correct_length]);
            } else {
                /* Corruption, error message, or SIGBUS */
            }
        })?;
    }
    Ok(StatusOk::Pass)
}

/** Generate a stream of data using seed `seed`, and send the first `max_write` bytes of it (or as much as possible),
 * while receiving the first `max_read` bytes of it (or as much as possible), and check that the data read matches
 * what was sent. */
fn check_pipe_transfer(
    pipe_w: OwnedFd,
    pipe_r: OwnedFd,
    seed: u64,
    max_write: Option<usize>,
    max_read: Option<usize>,
) {
    assert!(max_write.is_some() || max_read.is_some());
    let mut nwritten = 0;
    let start = Instant::now();
    let timeout = Duration::from_secs(1);

    let mut ord = Some(pipe_r);
    let mut owr = Some(pipe_w);
    if max_write == Some(0) {
        owr = None;
    }

    let mut recv = Vec::new();
    let mut to_send = Vec::new();
    let mut gen = BadRng { state: seed };

    let mut tmp = vec![0; 4096];
    while ord.is_some() || owr.is_some() {
        let mut pfds = Vec::new();
        let wr_idx = owr.as_ref().map(|x| {
            pfds.push(poll::PollFd::new(x.as_fd(), poll::PollFlags::POLLOUT));
            pfds.len() - 1
        });
        let rd_idx = ord.as_ref().map(|x| {
            pfds.push(poll::PollFd::new(x.as_fd(), poll::PollFlags::POLLIN));
            pfds.len() - 1
        });
        let current = Instant::now();
        let elapsed = current.duration_since(start);
        if elapsed >= timeout {
            panic!("timeout: {:?}", elapsed);
        }
        let remaining = time::TimeSpec::from_duration(timeout.saturating_sub(elapsed));
        let ret = system_poll(&mut pfds, Some(remaining), None);
        if let Err(e) = ret {
            assert!(e == Errno::EINTR);
        }
        let rev_wr = wr_idx.map(|i| pfds[i].revents().unwrap());
        let rev_rd = rd_idx.map(|i| pfds[i].revents().unwrap());

        if let Some(evts) = rev_rd {
            if evts.contains(poll::PollFlags::POLLIN) {
                let read_len = if let Some(limit) = max_read {
                    limit.checked_sub(recv.len()).unwrap().min(tmp.len())
                } else {
                    tmp.len()
                };

                match unistd::read(ord.as_ref().unwrap(), &mut tmp[..read_len]) {
                    Err(Errno::EINTR) | Err(Errno::EAGAIN) => { /* do nothing */ }
                    Err(Errno::ECONNRESET) | Err(Errno::ENOTCONN) => {
                        ord = None;
                    }
                    Err(x) => panic!("{:?}", x),
                    Ok(len) => {
                        if len > 0 {
                            recv.extend_from_slice(&tmp[..len]);
                        } else {
                            /* nothing more to read */
                            ord = None;
                        }
                        if let Some(limit) = max_read {
                            if recv.len() >= limit {
                                /* Stop reading, have read enough */
                                ord = None;
                            }
                        }
                    }
                }
            } else if evts.contains(poll::PollFlags::POLLHUP)
                || evts.contains(poll::PollFlags::POLLERR)
            {
                /* case: hangup, no pending data */
                ord = None;
            }
        }
        if let Some(evts) = rev_wr {
            if evts.contains(poll::PollFlags::POLLHUP) || evts.contains(poll::PollFlags::POLLERR) {
                owr = None;
            } else if evts.contains(poll::PollFlags::POLLOUT) {
                let extension = if let Some(len) = max_write {
                    len - to_send.len()
                } else {
                    std::cmp::max(nwritten + (1 << 20), to_send.len()) - to_send.len()
                };
                for _ in 0..extension {
                    to_send.push(gen.next() as u8);
                }
                assert!(to_send.len() > nwritten);

                match unistd::write(owr.as_ref().unwrap(), &to_send[nwritten..]) {
                    Err(Errno::EINTR) | Err(Errno::EAGAIN) => { /* do nothing */ }
                    Err(Errno::EPIPE) | Err(Errno::ECONNRESET) => {
                        owr = None;
                    }
                    Err(x) => panic!("{:?}", x),
                    Ok(len) => {
                        nwritten += len;
                        if max_write == Some(nwritten) {
                            owr = None;
                        }
                    }
                }
            }
        }
    }

    /* Data received must be a prefix of data sent */
    assert!(recv == to_send[..recv.len()]);
    if let Some(len) = max_read {
        /* Should have read exactly the requested amount */
        assert!(recv.len() == len);
    } else {
        /* Should read all of input */
        assert!(recv.len() == to_send.len());
    }
}

/** Test to check that basic pipe replication works for the various copy-paste type protocols, and
 * that the pipe replicated can be used to transfer various amounts of data. */
fn proto_pipe_write(info: TestInfo) -> TestResult {
    let (display, registry, manager, seat, dev, source) =
        (ObjId(1), ObjId(2), ObjId(3), ObjId(4), ObjId(5), ObjId(6));
    let seat_name = WL_SEAT;
    let ddev_name = WL_DATA_DEVICE_MANAGER;
    let prim_name = ZWP_PRIMARY_SELECTION_DEVICE_MANAGER_V1;
    let data_name = EXT_DATA_CONTROL_MANAGER_V1;
    let gtk_name = GTK_PRIMARY_SELECTION_DEVICE_MANAGER;
    let wlr_name = ZWLR_DATA_CONTROL_MANAGER_V1;
    let mime = "text/plain;charset=utf-8".as_bytes();

    /* Protocol sequences leading to a pipe receipt; in all cases the pipe is provided with the last message */
    let ex_wl: &[Vec<u8>] = &[
        build_msgs(|dst| {
            write_req_wl_display_get_registry(dst, display, registry);
        }),
        build_msgs(|dst| {
            write_evt_wl_registry_global(dst, registry, 1, seat_name, 7);
            write_evt_wl_registry_global(dst, registry, 2, ddev_name, 3);
        }),
        build_msgs(|dst| {
            write_req_wl_registry_bind(dst, registry, 1, seat_name, 7, seat);
            write_req_wl_registry_bind(dst, registry, 2, ddev_name, 3, manager);
            write_req_wl_data_device_manager_get_data_device(dst, manager, dev, seat);
            write_req_wl_data_device_manager_create_data_source(dst, manager, source);
            write_req_wl_data_source_offer(dst, source, mime);
            write_req_wl_data_device_set_selection(dst, dev, source, 99);
        }),
        build_msgs(|dst| {
            write_evt_wl_data_source_send(dst, source, false, mime);
        }),
    ];
    let ex_prim: &[Vec<u8>] = &[
        build_msgs(|dst| {
            write_req_wl_display_get_registry(dst, display, registry);
        }),
        build_msgs(|dst| {
            write_evt_wl_registry_global(dst, registry, 1, seat_name, 7);
            write_evt_wl_registry_global(dst, registry, 2, prim_name, 1);
        }),
        build_msgs(|dst| {
            write_req_wl_registry_bind(dst, registry, 1, seat_name, 7, seat);
            write_req_wl_registry_bind(dst, registry, 2, prim_name, 1, manager);
            write_req_zwp_primary_selection_device_manager_v1_get_device(dst, manager, dev, seat);
            write_req_zwp_primary_selection_device_manager_v1_create_source(dst, manager, source);
            write_req_zwp_primary_selection_source_v1_offer(dst, source, mime);
            write_req_zwp_primary_selection_device_v1_set_selection(dst, dev, source, 99);
        }),
        build_msgs(|dst| {
            write_evt_zwp_primary_selection_source_v1_send(dst, source, false, mime);
        }),
    ];
    let ex_data: &[Vec<u8>] = &[
        build_msgs(|dst| {
            write_req_wl_display_get_registry(dst, display, registry);
        }),
        build_msgs(|dst| {
            write_evt_wl_registry_global(dst, registry, 1, seat_name, 7);
            write_evt_wl_registry_global(dst, registry, 2, data_name, 1);
        }),
        build_msgs(|dst| {
            write_req_wl_registry_bind(dst, registry, 1, seat_name, 7, seat);
            write_req_wl_registry_bind(dst, registry, 2, data_name, 1, manager);
            write_req_ext_data_control_manager_v1_get_data_device(dst, manager, dev, seat);
            write_req_ext_data_control_manager_v1_create_data_source(dst, manager, source);
            write_req_ext_data_control_source_v1_offer(dst, source, mime);
            write_req_ext_data_control_device_v1_set_selection(dst, dev, source);
        }),
        build_msgs(|dst| {
            write_evt_ext_data_control_source_v1_send(dst, source, false, mime);
        }),
    ];
    let ex_gtk: &[Vec<u8>] = &[
        build_msgs(|dst| {
            write_req_wl_display_get_registry(dst, display, registry);
        }),
        build_msgs(|dst| {
            write_evt_wl_registry_global(dst, registry, 1, seat_name, 7);
            write_evt_wl_registry_global(dst, registry, 2, gtk_name, 1);
        }),
        build_msgs(|dst| {
            write_req_wl_registry_bind(dst, registry, 1, seat_name, 7, seat);
            write_req_wl_registry_bind(dst, registry, 2, gtk_name, 1, manager);
            write_req_gtk_primary_selection_device_manager_get_device(dst, manager, dev, seat);
            write_req_gtk_primary_selection_device_manager_create_source(dst, manager, source);
            write_req_gtk_primary_selection_source_offer(dst, source, mime);
            write_req_gtk_primary_selection_device_set_selection(dst, dev, source, 99);
        }),
        build_msgs(|dst| {
            write_evt_gtk_primary_selection_source_send(dst, source, false, mime);
        }),
    ];
    let ex_wlr: &[Vec<u8>] = &[
        build_msgs(|dst| {
            write_req_wl_display_get_registry(dst, display, registry);
        }),
        build_msgs(|dst| {
            write_evt_wl_registry_global(dst, registry, 1, seat_name, 7);
            write_evt_wl_registry_global(dst, registry, 2, wlr_name, 1);
        }),
        build_msgs(|dst| {
            write_req_wl_registry_bind(dst, registry, 1, seat_name, 7, seat);
            write_req_wl_registry_bind(dst, registry, 2, wlr_name, 1, manager);
            write_req_zwlr_data_control_manager_v1_get_data_device(dst, manager, dev, seat);
            write_req_zwlr_data_control_manager_v1_create_data_source(dst, manager, source);
            write_req_zwlr_data_control_source_v1_offer(dst, source, mime);
            write_req_zwlr_data_control_device_v1_set_selection(dst, dev, source);
        }),
        build_msgs(|dst| {
            write_evt_zwlr_data_control_source_v1_send(dst, source, false, mime);
        }),
    ];

    let (display, registry, manager, seat, dev, offer) =
        (ObjId(1), ObjId(2), ObjId(3), ObjId(4), ObjId(5), ObjId(6));
    /* Protocol sequences leading to a pipe receipt; in all cases the pipe is provided with the last message */
    let ex2_wl: &[Vec<u8>] = &[
        build_msgs(|dst| {
            write_req_wl_display_get_registry(dst, display, registry);
        }),
        build_msgs(|dst| {
            write_evt_wl_registry_global(dst, registry, 1, seat_name, 7);
            write_evt_wl_registry_global(dst, registry, 2, ddev_name, 3);
        }),
        build_msgs(|dst| {
            write_req_wl_registry_bind(dst, registry, 1, seat_name, 7, seat);
            write_req_wl_registry_bind(dst, registry, 2, ddev_name, 3, manager);
            write_req_wl_data_device_manager_get_data_device(dst, manager, dev, seat);
        }),
        build_msgs(|dst| {
            write_evt_wl_data_device_data_offer(dst, dev, offer);
            write_evt_wl_data_offer_offer(dst, offer, mime);
            write_evt_wl_data_device_selection(dst, dev, offer);
        }),
        build_msgs(|dst| {
            write_req_wl_data_offer_receive(dst, offer, false, mime);
        }),
    ];
    let ex2_prim: &[Vec<u8>] = &[
        build_msgs(|dst| {
            write_req_wl_display_get_registry(dst, display, registry);
        }),
        build_msgs(|dst| {
            write_evt_wl_registry_global(dst, registry, 1, seat_name, 7);
            write_evt_wl_registry_global(dst, registry, 2, prim_name, 1);
        }),
        build_msgs(|dst| {
            write_req_wl_registry_bind(dst, registry, 1, seat_name, 7, seat);
            write_req_wl_registry_bind(dst, registry, 2, prim_name, 1, manager);
            write_req_zwp_primary_selection_device_manager_v1_get_device(dst, manager, dev, seat);
        }),
        build_msgs(|dst| {
            write_evt_zwp_primary_selection_device_v1_data_offer(dst, dev, offer);
            write_evt_zwp_primary_selection_offer_v1_offer(dst, offer, mime);
            write_evt_zwp_primary_selection_device_v1_selection(dst, dev, offer);
        }),
        build_msgs(|dst| {
            write_req_zwp_primary_selection_offer_v1_receive(dst, offer, false, mime);
        }),
    ];
    let ex2_data: &[Vec<u8>] = &[
        build_msgs(|dst| {
            write_req_wl_display_get_registry(dst, display, registry);
        }),
        build_msgs(|dst| {
            write_evt_wl_registry_global(dst, registry, 1, seat_name, 7);
            write_evt_wl_registry_global(dst, registry, 2, data_name, 1);
        }),
        build_msgs(|dst| {
            write_req_wl_registry_bind(dst, registry, 1, seat_name, 7, seat);
            write_req_wl_registry_bind(dst, registry, 2, data_name, 1, manager);
            write_req_ext_data_control_manager_v1_get_data_device(dst, manager, dev, seat);
        }),
        build_msgs(|dst| {
            write_evt_ext_data_control_device_v1_data_offer(dst, dev, offer);
            write_evt_ext_data_control_offer_v1_offer(dst, offer, mime);
            write_evt_ext_data_control_device_v1_selection(dst, dev, offer);
        }),
        build_msgs(|dst| {
            write_req_ext_data_control_offer_v1_receive(dst, offer, false, mime);
        }),
    ];
    let ex2_gtk: &[Vec<u8>] = &[
        build_msgs(|dst| {
            write_req_wl_display_get_registry(dst, display, registry);
        }),
        build_msgs(|dst| {
            write_evt_wl_registry_global(dst, registry, 1, seat_name, 7);
            write_evt_wl_registry_global(dst, registry, 2, gtk_name, 1);
        }),
        build_msgs(|dst| {
            write_req_wl_registry_bind(dst, registry, 1, seat_name, 7, seat);
            write_req_wl_registry_bind(dst, registry, 2, gtk_name, 1, manager);
            write_req_gtk_primary_selection_device_manager_get_device(dst, manager, dev, seat);
        }),
        build_msgs(|dst| {
            write_evt_gtk_primary_selection_device_data_offer(dst, dev, offer);
            write_evt_gtk_primary_selection_offer_offer(dst, offer, mime);
            write_evt_gtk_primary_selection_device_selection(dst, dev, offer);
        }),
        build_msgs(|dst| {
            write_req_gtk_primary_selection_offer_receive(dst, offer, false, mime);
        }),
    ];
    let ex2_wlr: &[Vec<u8>] = &[
        build_msgs(|dst| {
            write_req_wl_display_get_registry(dst, display, registry);
        }),
        build_msgs(|dst| {
            write_evt_wl_registry_global(dst, registry, 1, seat_name, 7);
            write_evt_wl_registry_global(dst, registry, 2, wlr_name, 1);
        }),
        build_msgs(|dst| {
            write_req_wl_registry_bind(dst, registry, 1, seat_name, 7, seat);
            write_req_wl_registry_bind(dst, registry, 2, wlr_name, 1, manager);
            write_req_zwlr_data_control_manager_v1_get_data_device(dst, manager, dev, seat);
        }),
        build_msgs(|dst| {
            write_evt_zwlr_data_control_device_v1_data_offer(dst, dev, offer);
            write_evt_zwlr_data_control_offer_v1_offer(dst, offer, mime);
            write_evt_zwlr_data_control_device_v1_selection(dst, dev, offer);
        }),
        build_msgs(|dst| {
            write_req_zwlr_data_control_offer_v1_receive(dst, offer, false, mime);
        }),
    ];

    let test_cases: &[&[Vec<u8>]] = &[
        ex_wl, ex_prim, ex_data, ex_gtk, ex_wlr, ex2_wl, ex2_prim, ex2_data, ex2_gtk, ex2_wlr,
    ];

    let lengths = [usize::MAX, 100_usize, 0_usize, 131073_usize]
        .iter()
        .chain(std::iter::repeat(&256_usize));
    for (test_no, (test, length)) in test_cases.iter().zip(lengths).enumerate() {
        println!("Test {}.", test_no);
        run_protocol_test(&info, &|mut ctx: ProtocolTestContext| {
            let (pipe_r, pipe_w) =
                create_pipe(fcntl::OFlag::O_CLOEXEC | fcntl::OFlag::O_NONBLOCK).unwrap();
            for (i, line) in test.iter().enumerate().take(test.iter().len() - 1) {
                if i % 2 == 0 {
                    ctx.prog_write_passthrough(line.clone());
                } else {
                    ctx.comp_write_passthrough(line.clone());
                }
            }

            let end = test.last().unwrap();
            let ifds = [&pipe_w];
            let (msg, mut ofds) = if test.iter().len() % 2 == 0 {
                ctx.comp_write(end, &ifds)
            } else {
                ctx.prog_write(end, &ifds)
            }
            .unwrap();
            assert!(msg.concat() == *end);
            drop(pipe_w);
            let pipe_w = ofds.remove(0);

            if *length < usize::MAX {
                /* Send and receive a given length of message. */
                check_pipe_transfer(
                    pipe_w,
                    pipe_r,
                    test_no as u64,
                    Some(*length),
                    if test_no == 5 { Some(*length) } else { None },
                );
            } else {
                /* Send infinite message, and receive only the first part */
                check_pipe_transfer(pipe_w, pipe_r, test_no as u64, None, Some(50000));
            }

            /* Pass through empty message to determine if there was an error */
            ctx.comp_write_passthrough(Vec::new());
        })?;
    }
    Ok(StatusOk::Pass)
}

/** Test to verify that presentation time handling does not introduce major errors */
fn proto_presentation_time(info: TestInfo) -> TestResult {
    for (pres_clock, fast_start) in [
        (libc::CLOCK_MONOTONIC as u32, true),
        (libc::CLOCK_REALTIME as u32, false),
    ] {
        run_protocol_test(&info, &|mut ctx: ProtocolTestContext| {
            let (display, registry, pres, comp, surface, feedback) =
                (ObjId(1), ObjId(2), ObjId(3), ObjId(4), ObjId(5), ObjId(6));

            ctx.prog_write_passthrough(build_msgs(|dst| {
                write_req_wl_display_get_registry(dst, display, registry);
            }));

            ctx.comp_write_passthrough(build_msgs(|dst| {
                write_evt_wl_registry_global(dst, registry, 1, WP_PRESENTATION, 1);
                write_evt_wl_registry_global(dst, registry, 2, WL_COMPOSITOR, 1);
            }));

            let start = Instant::now();

            ctx.prog_write_passthrough(build_msgs(|dst| {
                write_req_wl_registry_bind(dst, registry, 1, WP_PRESENTATION, 1, pres);
                write_req_wl_registry_bind(dst, registry, 2, WL_COMPOSITOR, 1, comp);
                write_req_wl_compositor_create_surface(dst, comp, surface);
                write_req_wl_surface_damage(dst, surface, 0, 0, 64, 64);
                if fast_start {
                    write_req_wp_presentation_feedback(dst, pres, surface, feedback);
                    write_req_wp_presentation_destroy(dst, pres);
                    write_req_wl_surface_commit(dst, surface);
                }
            }));

            if !fast_start {
                ctx.comp_write_passthrough(build_msgs(|dst| {
                    write_evt_wp_presentation_clock_id(dst, pres, pres_clock);
                }));
                ctx.prog_write_passthrough(build_msgs(|dst| {
                    write_req_wp_presentation_feedback(dst, pres, surface, feedback);
                }));
            }

            let init_time_ns = 1111111111111u128;
            let data = build_msgs(|dst| {
                if fast_start {
                    write_evt_wp_presentation_clock_id(dst, pres, pres_clock);
                    write_evt_wl_display_delete_id(dst, display, pres.0);
                }
                write_evt_wp_presentation_feedback_presented(
                    dst,
                    feedback,
                    0,
                    (init_time_ns / 1000000000) as u32,
                    (init_time_ns % 1000000000) as u32,
                    500000000,
                    0,
                    10,
                    0x9,
                );
            });
            let (msgs, fds) = ctx.comp_write(&data, &[]).unwrap();
            let end = Instant::now();
            assert!(fds.is_empty());
            if fast_start {
                assert!(
                    msgs[..2].concat()
                        == data[..length_evt_wp_presentation_clock_id()
                            + length_evt_wl_display_delete_id()]
                );
            }
            assert!(
                parse_wl_header(msgs.last().unwrap())
                    == (
                        feedback,
                        length_evt_wp_presentation_feedback_presented(),
                        OPCODE_WP_PRESENTATION_FEEDBACK_PRESENTED.code()
                    )
            );
            let (tv_sec_hi, tv_sec_lo, tv_nsec, _refresh, _seq_hi, _seq_lo, _flags) =
                parse_evt_wp_presentation_feedback_presented(msgs.last().unwrap()).unwrap();
            let output_ns =
                1000000000 * (join_u64(tv_sec_hi, tv_sec_lo) as u128) + (tv_nsec as u128);

            /* The time adjustment uses two XYX measurements, whose absolute error
             * is ≤ half the elapsed time each, assuming the clocks run at the same
             * rate and do not change. */
            let max_time_error = end.duration_since(start).saturating_mul(2);

            let abs_diff = output_ns.abs_diff(init_time_ns);
            println!(
                "clock {}: roundtrip diff: {} ns, max permissible error: {} ns",
                pres_clock,
                (abs_diff as i128) * (if output_ns > init_time_ns { 1 } else { -1 }),
                max_time_error.as_nanos()
            );

            assert!(abs_diff < max_time_error.as_nanos());
        })?;
    }
    Ok(StatusOk::Pass)
}

/** Test to verify that presentation time handling does not introduce major errors */
fn proto_commit_timing(info: TestInfo) -> TestResult {
    for pres_clock in [libc::CLOCK_MONOTONIC as u32, libc::CLOCK_REALTIME as u32] {
        run_protocol_test(&info, &|mut ctx: ProtocolTestContext| {
            let [display, registry, pres, comp, manager, surface, timer, ..] = ID_SEQUENCE;

            ctx.prog_write_passthrough(build_msgs(|dst| {
                write_req_wl_display_get_registry(dst, display, registry);
            }));

            ctx.comp_write_passthrough(build_msgs(|dst| {
                write_evt_wl_registry_global(dst, registry, 1, WP_PRESENTATION, 1);
                write_evt_wl_registry_global(dst, registry, 2, WP_COMMIT_TIMING_MANAGER_V1, 1);
                write_evt_wl_registry_global(dst, registry, 3, WL_COMPOSITOR, 1);
            }));

            let start = Instant::now();

            ctx.prog_write_passthrough(build_msgs(|dst| {
                write_req_wl_registry_bind(dst, registry, 1, WP_PRESENTATION, 1, pres);
                write_req_wl_registry_bind(dst, registry, 3, WL_COMPOSITOR, 1, comp);
                write_req_wl_registry_bind(
                    dst,
                    registry,
                    2,
                    WP_COMMIT_TIMING_MANAGER_V1,
                    1,
                    manager,
                );
                write_req_wl_compositor_create_surface(dst, comp, surface);
                write_req_wl_surface_damage(dst, surface, 0, 0, 64, 64);
                write_req_wp_commit_timing_manager_v1_get_timer(dst, manager, timer, surface);
            }));

            ctx.comp_write_passthrough(build_msgs(|dst| {
                write_evt_wp_presentation_clock_id(dst, pres, pres_clock);
            }));

            let init_time_ns = 11111111111111111111111u128;
            let ((tv_sec_hi, tv_sec_lo), tv_nsec) = (
                split_u64((init_time_ns / 1000000000) as u64),
                (init_time_ns % 1000000000) as u32,
            );
            let data = build_msgs(|dst| {
                write_req_wp_commit_timer_v1_set_timestamp(
                    dst, timer, tv_sec_hi, tv_sec_lo, tv_nsec,
                );
            });
            let (msgs, fds) = ctx.prog_write(&data, &[]).unwrap();
            let end = Instant::now();
            assert!(fds.is_empty());
            assert!(msgs.len() == 1);
            let msg = &msgs[0];
            assert!(
                parse_wl_header(msg)
                    == (
                        timer,
                        length_req_wp_commit_timer_v1_set_timestamp(),
                        OPCODE_WP_COMMIT_TIMER_V1_SET_TIMESTAMP.code()
                    )
            );
            let (new_sec_hi, new_sec_lo, new_nsec) =
                parse_req_wp_commit_timer_v1_set_timestamp(msg).unwrap();
            let output_ns =
                1000000000 * (join_u64(new_sec_hi, new_sec_lo) as u128) + (new_nsec as u128);

            /* The time adjustment uses two XYX measurements, whose absolute error
             * is ≤ half the elapsed time each, assuming the clocks run at the same
             * rate and do not change. */
            let max_time_error = end.duration_since(start).saturating_mul(2);

            let abs_diff = output_ns.abs_diff(init_time_ns);
            println!(
                "clock {}: roundtrip diff: {} ns, max permissible error: {} ns",
                pres_clock,
                (abs_diff as i128) * (if output_ns > init_time_ns { 1 } else { -1 }),
                max_time_error.as_nanos()
            );
            assert!(abs_diff < max_time_error.as_nanos());
        })?;
    }
    Ok(StatusOk::Pass)
}

/** Test that an error is reported when a Wayland client attempts to make two objects
 * with the same ID at the same time. */
fn proto_object_collision(info: TestInfo) -> TestResult {
    run_protocol_test(&info, &|mut ctx: ProtocolTestContext| {
        let (display, registry) = (ObjId(1), ObjId(2));
        let msgs = build_msgs(|dst| {
            write_req_wl_display_get_registry(dst, display, registry);
            write_req_wl_display_get_registry(dst, display, registry);
        });
        let res = ctx.prog_write(&msgs, &[]);
        if let Err(ref e) = res {
            println!("error: {:?}", e);
        }
        assert!(res.is_err());
    })?;
    Ok(StatusOk::Pass)
}

/** Test to check that a wl_shm buffer can be created and its contents replicated. */
fn proto_shm_buffer(info: TestInfo) -> TestResult {
    run_protocol_test(&info, &|mut ctx: ProtocolTestContext| {
        let (display, registry, shm, comp, surface, pool, buffer) = (
            ObjId(1),
            ObjId(2),
            ObjId(3),
            ObjId(4),
            ObjId(5),
            ObjId(6),
            ObjId(7),
        );

        ctx.prog_write_passthrough(build_msgs(|dst| {
            write_req_wl_display_get_registry(dst, display, registry);
        }));

        ctx.comp_write_passthrough(build_msgs(|dst| {
            write_evt_wl_registry_global(dst, registry, 1, WL_SHM, 2);
            write_evt_wl_registry_global(dst, registry, 2, WL_COMPOSITOR, 6);
        }));

        ctx.prog_write_passthrough(build_msgs(|dst| {
            write_req_wl_registry_bind(dst, registry, 1, WL_SHM, 2, shm);
            write_req_wl_registry_bind(dst, registry, 2, WL_COMPOSITOR, 6, comp);
            write_req_wl_compositor_create_surface(dst, comp, surface);
        }));

        /* First, simple test case: an empty shm pool, never modified */
        let empty_fd = make_file_with_contents(&[]).unwrap();
        let msg = build_msgs(|dst| {
            write_req_wl_shm_create_pool(dst, shm, false, pool, 0);
        });
        let (rmsg, mut rfd) = ctx.prog_write(&msg[..], &[&empty_fd]).unwrap();
        assert!(rmsg.concat() == msg);
        drop(empty_fd);
        assert!(rfd.len() == 1);
        let output_fd = rfd.remove(0);
        assert!(get_file_contents(&output_fd, 0).unwrap().is_empty());

        ctx.prog_write_passthrough(build_msgs(|dst| {
            write_req_wl_shm_pool_destroy(dst, pool);
        }));
        ctx.comp_write_passthrough(build_msgs(|dst| {
            write_evt_wl_display_delete_id(dst, display, pool.0);
        }));

        /* pools and images in various sizes */
        for (w, h) in [(3, 3), (16, 16), (1023, 1025)] {
            let sz: usize = w * h;
            let mut data = vec![0; sz];
            let mut i: u8 = 0x80;
            /* Draw a square inside */
            for y in (h / 3)..(2 * h) / 3 {
                for x in (w / 3)..(2 * w) / 3 {
                    data[y * w + x] = i;
                    i = i.wrapping_add(1);
                }
            }
            let buf_fd = make_file_with_contents(&data[..]).unwrap();
            let msgs = build_msgs(|dst| {
                write_req_wl_shm_create_pool(dst, shm, false, pool, sz as i32);
                write_req_wl_shm_pool_create_buffer(
                    dst,
                    pool,
                    buffer,
                    0,
                    w as i32,
                    h as i32,
                    w as i32,
                    WlShmFormat::R8 as u32,
                );
                write_req_wl_surface_attach(dst, surface, buffer, 0, 0);
                write_req_wl_surface_damage_buffer(dst, surface, 0, 0, i32::MAX, i32::MAX);
                write_req_wl_surface_commit(dst, surface);
            });
            let (rmsg, mut rfd) = ctx.prog_write(&msgs[..], &[&buf_fd]).unwrap();
            assert!(rmsg.concat() == msgs);
            drop(buf_fd);
            assert!(rfd.len() == 1);
            let output_fd = rfd.remove(0);
            let output = get_file_contents(&output_fd, sz).unwrap();
            assert!(output == data);

            /* Cleanup */
            ctx.prog_write_passthrough(build_msgs(|dst| {
                write_req_wl_shm_pool_destroy(dst, pool);
                write_req_wl_buffer_destroy(dst, buffer);
            }));
            ctx.comp_write_passthrough(build_msgs(|dst| {
                write_evt_wl_display_delete_id(dst, display, pool.0);
                write_evt_wl_display_delete_id(dst, display, buffer.0);
            }));
        }
    })?;
    Ok(StatusOk::Pass)
}

/** Test that wl_shm_pool::resize is handled correctly. */
fn proto_shm_extend(info: TestInfo) -> TestResult {
    run_protocol_test(&info, &|mut ctx: ProtocolTestContext| {
        let (display, registry, shm, comp, surface, pool, buf_a, buf_b, buf_c) = (
            ObjId(1),
            ObjId(2),
            ObjId(3),
            ObjId(4),
            ObjId(5),
            ObjId(6),
            ObjId(7),
            ObjId(8),
            ObjId(9),
        );

        ctx.prog_write_passthrough(build_msgs(|dst| {
            write_req_wl_display_get_registry(dst, display, registry);
        }));

        ctx.comp_write_passthrough(build_msgs(|dst| {
            write_evt_wl_registry_global(dst, registry, 1, WL_SHM, 2);
            write_evt_wl_registry_global(dst, registry, 2, WL_COMPOSITOR, 6);
        }));

        let pool_fd = make_file_with_contents(&[]).unwrap();

        let msg = build_msgs(|dst| {
            write_req_wl_registry_bind(dst, registry, 1, WL_SHM, 2, shm);
            write_req_wl_registry_bind(dst, registry, 2, WL_COMPOSITOR, 6, comp);
            write_req_wl_compositor_create_surface(dst, comp, surface);
            write_req_wl_shm_create_pool(dst, shm, false, pool, 0);
        });
        let (rmsg, mut rfd) = ctx.prog_write(&msg[..], &[&pool_fd]).unwrap();
        assert!(rmsg.concat() == msg);
        assert!(rfd.len() == 1);
        let mirror_fd = rfd.remove(0);
        assert!(get_file_contents(&mirror_fd, 0).unwrap().is_empty());

        let (w, h): (usize, usize) = (15, 15);
        let fmt = WlShmFormat::Abgr16161616;

        let buf_ids = [buf_a, buf_b, buf_c];
        let mut i: u16 = 0x1000;
        for nblocks in 1..=3 {
            let new_sz = w * h * 8 * nblocks;
            let mut img_data = vec![0u8; new_sz];
            for k in 0..nblocks {
                /* Fill with gradient */
                let block_data = &mut img_data[(w * h * 8 * k)..(w * h * 8 * (k + 1))];
                for j in 0..(w * h) {
                    for h in 0..4 {
                        block_data[(8 * j + 2 * h)..(8 * j + 2 * h + 2)]
                            .copy_from_slice(&((h as u16 + 1) * i).to_le_bytes());
                    }
                    i = i.wrapping_add(1);
                }
            }

            resize_file_with_contents(&pool_fd, &img_data).unwrap();

            ctx.prog_write_passthrough(build_msgs(|dst| {
                for i in 0..nblocks {
                    if i == nblocks - 1 {
                        write_req_wl_shm_pool_resize(dst, pool, new_sz as i32);
                        write_req_wl_shm_pool_create_buffer(
                            dst,
                            pool,
                            buf_ids[nblocks - 1],
                            (w * h * 8 * (nblocks - 1)) as i32,
                            w as i32,
                            h as i32,
                            (w * 8) as i32,
                            fmt as u32,
                        );
                    }

                    write_req_wl_surface_attach(dst, surface, buf_ids[i], 0, 0);
                    write_req_wl_surface_damage_buffer(dst, surface, 0, 0, i32::MAX, i32::MAX);
                    write_req_wl_surface_commit(dst, surface);
                }
            }));

            let mir = get_file_contents(&mirror_fd, new_sz).unwrap();
            assert!(mir == img_data);
        }
    })?;
    Ok(StatusOk::Pass)
}

/** Helper function: _process_ dmabuf_feedback events for the given `feedback` object, and
 * return the map of format-modifier combinations supported by the program-side Waypipe
 * instance. `format_table` should be, and will be updated to, the most recently received
 * format table. */
fn process_linux_dmabuf_feedback(
    rmsgs: Vec<Vec<u8>>,
    mut rfds: Vec<OwnedFd>,
    format_table: &mut Vec<(u32, u64)>,
    feedback: ObjId,
) -> BTreeMap<u32, Vec<u64>> {
    rfds.reverse();

    let mut mod_table: BTreeMap<u32, Vec<u64>> = BTreeMap::new();
    let mut tranches: Vec<Vec<(u32, u64)>> = Vec::new();
    let mut current_tranche = Vec::new();
    for msg in rmsgs {
        let (obj, _len, opcode) = parse_wl_header(&msg);
        /* Have only send events to this dmabuf_feedback */
        assert!(obj == feedback);
        match MethodId::Event(opcode) {
            OPCODE_ZWP_LINUX_DMABUF_FEEDBACK_V1_MAIN_DEVICE => {
                /* ignore, Waypipe may choose a different one */
            }
            OPCODE_ZWP_LINUX_DMABUF_FEEDBACK_V1_TRANCHE_TARGET_DEVICE => {
                /* ignore, Waypipe currently doesn't do anything interesting here */
            }
            OPCODE_ZWP_LINUX_DMABUF_FEEDBACK_V1_TRANCHE_FLAGS => {
                /* ignore, Waypipe currently doesn't do anything interesting here */
            }
            OPCODE_ZWP_LINUX_DMABUF_FEEDBACK_V1_TRANCHE_FORMATS => {
                let format_list =
                    parse_evt_zwp_linux_dmabuf_feedback_v1_tranche_formats(&msg).unwrap();
                /* The indices correspond to the last received format table, and should must be interpreted immediately
                 * in case the format table is changed later on */
                for c in format_list.chunks_exact(2) {
                    let idx = u16::from_le_bytes(c.try_into().unwrap());
                    let entry: (u32, u64) = *format_table
                        .get(idx as usize)
                        .expect("format index out of range");
                    current_tranche.push(entry);
                }
            }
            OPCODE_ZWP_LINUX_DMABUF_FEEDBACK_V1_TRANCHE_DONE => {
                tranches.push(current_tranche.clone());
                current_tranche = Vec::new();
            }
            OPCODE_ZWP_LINUX_DMABUF_FEEDBACK_V1_FORMAT_TABLE => {
                format_table.clear();
                let fd = rfds.pop().unwrap();
                let len = parse_evt_zwp_linux_dmabuf_feedback_v1_format_table(&msg).unwrap();
                let table_contents = get_file_contents(&fd, len as usize).unwrap();
                for chunk in table_contents.chunks_exact(16) {
                    let format: u32 = u32::from_le_bytes(chunk[..4].try_into().unwrap());
                    let modifier: u64 = u64::from_le_bytes(chunk[8..].try_into().unwrap());
                    format_table.push((format, modifier));
                }
            }
            OPCODE_ZWP_LINUX_DMABUF_FEEDBACK_V1_DONE => {
                mod_table = BTreeMap::new();
                for t in &tranches {
                    for (fmt, modifier) in t {
                        mod_table
                            .entry(*fmt)
                            .and_modify(|x| x.push(*modifier))
                            .or_insert(vec![*modifier]);
                    }
                }
            }
            _ => {
                panic!("Unexpected opcode: {}", opcode);
            }
        }
    }
    assert!(rfds.is_empty());
    mod_table
}

/** Helper function: send dmabuf_feedback events for the given `feedback` object, and
 * return the map of format-modifier combinations supported by the program-side Waypipe
 * instance. */
#[cfg(feature = "dmabuf")]
fn send_linux_dmabuf_feedback(
    ctx: &mut ProtocolTestContext,
    vulk: &VulkanDevice,
    feedback: ObjId,
) -> BTreeMap<u32, Vec<u64>> {
    let main_device = vulk.get_device();
    let advertised_formats = [
        wayland_to_drm(WlShmFormat::R8),
        wayland_to_drm(WlShmFormat::Rgb565),
        wayland_to_drm(WlShmFormat::Argb8888),
        wayland_to_drm(WlShmFormat::Xrgb8888),
        wayland_to_drm(WlShmFormat::Xbgr16161616),
        wayland_to_drm(WlShmFormat::Abgr16161616),
    ];
    let mut table: Vec<u8> = Vec::new();
    let mut array: Vec<u8> = Vec::new();
    let mut i: u16 = 0;
    for fmt in advertised_formats {
        let modifier_list = vulk.get_supported_modifiers(fmt);
        for m in modifier_list {
            table.extend_from_slice(&fmt.to_le_bytes());
            table.extend_from_slice(&0_u32.to_le_bytes());
            table.extend_from_slice(&m.to_le_bytes());
            array.extend_from_slice(&i.to_le_bytes());
            i += 1;
        }
    }
    let table_fd = make_file_with_contents(&table).unwrap();
    let msgs = build_msgs(|dst| {
        write_evt_zwp_linux_dmabuf_feedback_v1_format_table(
            dst,
            feedback,
            false,
            table.len() as u32,
        );
        write_evt_zwp_linux_dmabuf_feedback_v1_main_device(
            dst,
            feedback,
            &main_device.to_le_bytes(),
        );
        write_evt_zwp_linux_dmabuf_feedback_v1_tranche_target_device(
            dst,
            feedback,
            &main_device.to_le_bytes(),
        );
        write_evt_zwp_linux_dmabuf_feedback_v1_tranche_flags(dst, feedback, 0);
        write_evt_zwp_linux_dmabuf_feedback_v1_tranche_formats(dst, feedback, &array[..]);
        write_evt_zwp_linux_dmabuf_feedback_v1_tranche_done(dst, feedback);
        write_evt_zwp_linux_dmabuf_feedback_v1_done(dst, feedback);
    });
    let (rmsgs, rfds) = ctx.comp_write(&msgs[..], &[&table_fd]).unwrap();
    let mut format_table = Vec::new();
    process_linux_dmabuf_feedback(rmsgs, rfds, &mut format_table, feedback)
}

/** Helper function: setup wl_compositor and zwp_linux_dmabuf_v1 globals and a surface,
 * and return the map of format-modifier combinations supported by the program-side Waypipe
 * instance. */
#[cfg(feature = "dmabuf")]
fn setup_linux_dmabuf(
    ctx: &mut ProtocolTestContext,
    vulk: &VulkanDevice,
    display: ObjId,
    registry: ObjId,
    dmabuf: ObjId,
    comp: ObjId,
    surface: ObjId,
    feedback: ObjId,
) -> BTreeMap<u32, Vec<u64>> {
    ctx.prog_write_passthrough(build_msgs(|dst| {
        write_req_wl_display_get_registry(dst, display, registry);
    }));

    ctx.comp_write_passthrough(build_msgs(|dst| {
        write_evt_wl_registry_global(dst, registry, 1, ZWP_LINUX_DMABUF_V1, 5);
        write_evt_wl_registry_global(dst, registry, 2, WL_COMPOSITOR, 6);
    }));

    ctx.prog_write_passthrough(build_msgs(|dst| {
        write_req_wl_registry_bind(dst, registry, 1, ZWP_LINUX_DMABUF_V1, 5, dmabuf);
        write_req_wl_registry_bind(dst, registry, 2, WL_COMPOSITOR, 6, comp);
        write_req_wl_compositor_create_surface(dst, comp, surface);
        write_req_zwp_linux_dmabuf_v1_get_default_feedback(dst, dmabuf, feedback);
    }));

    send_linux_dmabuf_feedback(ctx, vulk, feedback)
}

/** Test that `wl_buffer` objects backed by DMABUFs are properly replicated. */
#[cfg(feature = "dmabuf")]
fn proto_dmabuf(info: TestInfo, device: RenderDevice) -> TestResult {
    let Ok(vulk) = setup_vulkan(device.id) else {
        return Ok(StatusOk::Skipped);
    };

    run_protocol_test_with_drm_node(&info, &device, &|mut ctx: ProtocolTestContext| {
        let (display, registry, dmabuf, comp, surface, feedback, params, buffer, sbuffer) = (
            ObjId(1),
            ObjId(2),
            ObjId(3),
            ObjId(4),
            ObjId(5),
            ObjId(6),
            ObjId(7),
            ObjId(8),
            ObjId(0xff000000),
        );

        let supported_modifier_table = setup_linux_dmabuf(
            &mut ctx, &vulk, display, registry, dmabuf, comp, surface, feedback,
        );
        let fmt = wayland_to_drm(WlShmFormat::Rgb565);
        let bpp = 2;
        let Some(mod_list) = supported_modifier_table.get(&fmt) else {
            println!("Skipping test, format not supported");
            return;
        };

        for (w, h, immed) in [(3, 4, true), (64, 64, true), (513, 511, false)] {
            let mut img_data = vec![0u8; w * h * bpp];
            let mut i: u16 = 0x1234;
            /* Draw a square inside */
            for y in (h / 3)..(2 * h) / 3 {
                for x in (w / 3)..(2 * w) / 3 {
                    img_data[2 * (y * w + x)..2 * (y * w + x) + 2]
                        .copy_from_slice(&i.to_le_bytes());
                    i = i.wrapping_add(1);
                }
            }

            let (img, planes) =
                vulkan_create_dmabuf(&vulk, w as u32, h as u32, fmt, mod_list, false).unwrap();

            let msgs = build_msgs(|dst| {
                write_req_zwp_linux_dmabuf_v1_create_params(dst, dmabuf, params);
                for p in planes.iter() {
                    let (mod_hi, mod_lo) = split_u64(p.modifier);
                    write_req_zwp_linux_buffer_params_v1_add(
                        dst,
                        params,
                        false,
                        p.plane_idx,
                        p.offset,
                        p.stride,
                        mod_hi,
                        mod_lo,
                    );
                }

                if immed {
                    write_req_zwp_linux_buffer_params_v1_create_immed(
                        dst, params, buffer, w as i32, h as i32, fmt, 0,
                    );
                } else {
                    write_req_zwp_linux_buffer_params_v1_create(
                        dst, params, w as i32, h as i32, fmt, 0,
                    );
                }
            });
            let plane_fds: Vec<&OwnedFd> = planes.iter().map(|p| &p.fd).collect();
            let (rmsgs, mut rfds) = ctx.prog_write(&msgs[..], &plane_fds[..]).unwrap();
            drop(planes);
            let add_msgs = &rmsgs[1..rmsgs.len() - 1];
            assert!(rfds.len() == add_msgs.len());
            let create_msg = &rmsgs[rmsgs.len() - 1];
            let (rw, rh, rfmt) = if immed {
                let (_rbuf, rw, rh, rfmt, _rflags) =
                    parse_req_zwp_linux_buffer_params_v1_create_immed(&create_msg[..]).unwrap();
                (rw, rh, rfmt)
            } else {
                let (rw, rh, rfmt, _rflags) =
                    parse_req_zwp_linux_buffer_params_v1_create(&create_msg[..]).unwrap();
                (rw, rh, rfmt)
            };
            assert!((rw, rh, rfmt) == (w as i32, h as i32, fmt));

            let mut planes = Vec::new();
            for (fd, msg) in rfds.drain(..).zip(add_msgs.iter()) {
                let (plane_idx, offset, stride, mod_hi, mod_lo) =
                    parse_req_zwp_linux_buffer_params_v1_add(msg).unwrap();
                let modifier = join_u64(mod_hi, mod_lo);
                planes.push(AddDmabufPlane {
                    fd,
                    plane_idx,
                    offset,
                    stride,
                    modifier,
                });
            }
            let mirror =
                vulkan_import_dmabuf(&vulk, planes, w as u32, h as u32, fmt, false).unwrap();

            let tmp = Arc::new(vulkan_get_buffer(&vulk, img.nominal_size(None), true).unwrap());
            copy_onto_dmabuf(&img, &tmp, &img_data).unwrap();

            if immed {
                ctx.prog_write_passthrough(build_msgs(|dst| {
                    write_req_wl_surface_attach(dst, surface, buffer, 0, 0);
                    write_req_wl_surface_damage_buffer(dst, surface, 0, 0, i32::MAX, i32::MAX);
                    write_req_wl_surface_commit(dst, surface);
                }));
            } else {
                ctx.comp_write_passthrough(build_msgs(|dst| {
                    write_evt_zwp_linux_buffer_params_v1_created(dst, params, sbuffer);
                }));
                ctx.prog_write_passthrough(build_msgs(|dst| {
                    write_req_wl_surface_attach(dst, surface, sbuffer, 0, 0);
                    write_req_wl_surface_damage_buffer(dst, surface, 0, 0, i32::MAX, i32::MAX);
                    write_req_wl_surface_commit(dst, surface);
                }));
            }

            let tmp = Arc::new(vulkan_get_buffer(&vulk, mirror.nominal_size(None), true).unwrap());
            let mir_data = copy_from_dmabuf(&img, &tmp).unwrap();
            assert!(img_data == mir_data);

            /* Cleanup buffer and params objects, for reuse with next image */
            ctx.prog_write_passthrough(build_msgs(|dst| {
                write_req_zwp_linux_buffer_params_v1_destroy(dst, params);
                write_req_wl_buffer_destroy(dst, if immed { buffer } else { sbuffer });
            }));
            ctx.comp_write_passthrough(build_msgs(|dst| {
                write_evt_wl_display_delete_id(dst, display, params.0);
                if immed {
                    write_evt_wl_display_delete_id(dst, display, buffer.0);
                }
            }));
        }
    })?;
    Ok(StatusOk::Pass)
}

/** Test that Waypipe can process dmabuf feedback _changes_ (including table changes
 * during the feedback events) properly */
#[cfg(feature = "dmabuf")]
fn proto_dmabuf_feedback_table(info: TestInfo, device: RenderDevice) -> TestResult {
    if setup_vulkan(device.id).is_err() {
        return Ok(StatusOk::Skipped);
    };

    fn make_simple_format_table(formats: &[u32]) -> (OwnedFd, u32) {
        let mod_linear = 0u64;

        let mut table: Vec<u8> = Vec::new();
        for fmt in formats {
            table.extend_from_slice(&fmt.to_le_bytes());
            table.extend_from_slice(&0_u32.to_le_bytes());
            table.extend_from_slice(&mod_linear.to_le_bytes());
        }
        (make_file_with_contents(&table).unwrap(), table.len() as u32)
    }
    fn make_index_array(indices: &[usize]) -> Vec<u8> {
        let mut arr: Vec<u8> = Vec::new();
        for i in indices {
            arr.extend_from_slice(&(*i as u16).to_le_bytes());
        }
        arr
    }

    run_protocol_test_with_drm_node(&info, &device, &|mut ctx: ProtocolTestContext| {
        let [display, registry, dmabuf, feedback, ..] = ID_SEQUENCE;

        /* Note: Vulkan implementations are required to support TRANSFER_SRC_BIT |
         * TRANSFER_DST_BIT for the following formats, so Waypipe should be able to
         * process all their corresponding formats.
         *
         * R5G6B5_UNORM_PACK16, R8G8_UNORM, R8G8B8A8_UNORM, A2B10G10R10_UNORM
         * R16G16B16A16_SFLOAT.
         */

        let formats: [u32; 4] = [
            wayland_to_drm(WlShmFormat::R8),
            wayland_to_drm(WlShmFormat::Rgb565),
            wayland_to_drm(WlShmFormat::Argb8888),
            wayland_to_drm(WlShmFormat::Xrgb8888),
        ];
        println!("Formats: {:?}", formats);
        let unsupported_format: u32 = 0xFFFFFFFF;

        let main_device = &device.id.to_le_bytes();

        ctx.prog_write_passthrough(build_msgs(|dst| {
            write_req_wl_display_get_registry(dst, display, registry);
        }));

        ctx.comp_write_passthrough(build_msgs(|dst| {
            write_evt_wl_registry_global(dst, registry, 1, ZWP_LINUX_DMABUF_V1, 5);
        }));

        ctx.prog_write_passthrough(build_msgs(|dst| {
            write_req_wl_registry_bind(dst, registry, 1, ZWP_LINUX_DMABUF_V1, 5, dmabuf);
            write_req_zwp_linux_dmabuf_v1_get_default_feedback(dst, dmabuf, feedback);
        }));

        /* Round 1: Regular setup */
        let r1_formats = [formats[0], formats[1]];
        let (r1_fd, r1_size) = make_simple_format_table(&r1_formats);
        let mut format_table = Vec::new();

        let ret = ctx
            .comp_write(
                &build_msgs(|dst| {
                    write_evt_zwp_linux_dmabuf_feedback_v1_format_table(
                        dst, feedback, false, r1_size,
                    );
                    write_evt_zwp_linux_dmabuf_feedback_v1_main_device(dst, feedback, main_device);
                    write_evt_zwp_linux_dmabuf_feedback_v1_tranche_target_device(
                        dst,
                        feedback,
                        main_device,
                    );
                    write_evt_zwp_linux_dmabuf_feedback_v1_tranche_flags(dst, feedback, 0);
                    write_evt_zwp_linux_dmabuf_feedback_v1_tranche_formats(
                        dst,
                        feedback,
                        &make_index_array(&[1, 0]),
                    );
                    write_evt_zwp_linux_dmabuf_feedback_v1_tranche_done(dst, feedback);
                    write_evt_zwp_linux_dmabuf_feedback_v1_done(dst, feedback);
                }),
                &[&r1_fd],
            )
            .unwrap();
        let map = process_linux_dmabuf_feedback(ret.0, ret.1, &mut format_table, feedback);
        println!("Round 1: {:?}", map);
        for f in r1_formats {
            assert!(map.contains_key(&f), "missing {:x}", f);
        }

        /* Round 2: Repeat setup with slightly different tranches, keeping the same table */
        let ret = ctx
            .comp_write(
                &build_msgs(|dst| {
                    write_evt_zwp_linux_dmabuf_feedback_v1_main_device(dst, feedback, main_device);

                    write_evt_zwp_linux_dmabuf_feedback_v1_tranche_target_device(
                        dst,
                        feedback,
                        main_device,
                    );
                    write_evt_zwp_linux_dmabuf_feedback_v1_tranche_flags(dst, feedback, 0);
                    write_evt_zwp_linux_dmabuf_feedback_v1_tranche_formats(
                        dst,
                        feedback,
                        &make_index_array(&[0]),
                    );
                    write_evt_zwp_linux_dmabuf_feedback_v1_tranche_done(dst, feedback);

                    write_evt_zwp_linux_dmabuf_feedback_v1_tranche_target_device(
                        dst,
                        feedback,
                        main_device,
                    );
                    write_evt_zwp_linux_dmabuf_feedback_v1_tranche_flags(dst, feedback, 0);
                    write_evt_zwp_linux_dmabuf_feedback_v1_tranche_formats(
                        dst,
                        feedback,
                        &make_index_array(&[1]),
                    );
                    write_evt_zwp_linux_dmabuf_feedback_v1_tranche_done(dst, feedback);

                    write_evt_zwp_linux_dmabuf_feedback_v1_done(dst, feedback);
                }),
                &[],
            )
            .unwrap();
        let map = process_linux_dmabuf_feedback(ret.0, ret.1, &mut format_table, feedback);
        println!("Round 2: {:?}", map);
        for f in r1_formats {
            assert!(map.contains_key(&f), "missing {:x}", f);
        }

        /* Round 3: Changing format table for each tranche, or more frequently.
         * (Note: the protocol specification implies but does not explicitly state that
         * there is at most one format table per feedback::done; in practice most
         * implementations will work even if the table is changed more often.)
         *
         * Table 1; tranche 1
         * Table 2: tranche 2
         * Table 3: ignored
         * Table 4: empty, ignored
         * Table 5: tranches 3+4
         */
        let t1_formats = [formats[1]];
        let t2_formats = [unsupported_format, formats[0]];
        let t3_formats = [formats[0], formats[3], formats[3], formats[0]];
        let t4_formats: [u32; 0] = [];
        let t5_formats = [formats[3], formats[2], unsupported_format];
        let (t1, t2, t3, t4, t5) = (
            make_simple_format_table(&t1_formats),
            make_simple_format_table(&t2_formats),
            make_simple_format_table(&t3_formats),
            make_simple_format_table(&t4_formats),
            make_simple_format_table(&t5_formats),
        );
        let ret = ctx
            .comp_write(
                &build_msgs(|dst| {
                    write_evt_zwp_linux_dmabuf_feedback_v1_main_device(dst, feedback, main_device);

                    write_evt_zwp_linux_dmabuf_feedback_v1_format_table(dst, feedback, false, t1.1);
                    write_evt_zwp_linux_dmabuf_feedback_v1_tranche_target_device(
                        dst,
                        feedback,
                        main_device,
                    );
                    write_evt_zwp_linux_dmabuf_feedback_v1_tranche_flags(dst, feedback, 0);
                    write_evt_zwp_linux_dmabuf_feedback_v1_tranche_formats(
                        dst,
                        feedback,
                        &make_index_array(&[0]),
                    );
                    write_evt_zwp_linux_dmabuf_feedback_v1_tranche_done(dst, feedback);

                    write_evt_zwp_linux_dmabuf_feedback_v1_format_table(dst, feedback, false, t2.1);
                    write_evt_zwp_linux_dmabuf_feedback_v1_tranche_target_device(
                        dst,
                        feedback,
                        main_device,
                    );
                    write_evt_zwp_linux_dmabuf_feedback_v1_tranche_flags(dst, feedback, 0);
                    write_evt_zwp_linux_dmabuf_feedback_v1_tranche_formats(
                        dst,
                        feedback,
                        &make_index_array(&[1]),
                    );
                    write_evt_zwp_linux_dmabuf_feedback_v1_tranche_done(dst, feedback);

                    write_evt_zwp_linux_dmabuf_feedback_v1_format_table(dst, feedback, false, t3.1);
                    write_evt_zwp_linux_dmabuf_feedback_v1_format_table(dst, feedback, false, t4.1);

                    write_evt_zwp_linux_dmabuf_feedback_v1_format_table(dst, feedback, false, t5.1);
                    write_evt_zwp_linux_dmabuf_feedback_v1_tranche_target_device(
                        dst,
                        feedback,
                        main_device,
                    );
                    write_evt_zwp_linux_dmabuf_feedback_v1_tranche_flags(dst, feedback, 0);
                    write_evt_zwp_linux_dmabuf_feedback_v1_tranche_formats(
                        dst,
                        feedback,
                        &make_index_array(&[0, 1, 2]),
                    );
                    write_evt_zwp_linux_dmabuf_feedback_v1_tranche_done(dst, feedback);

                    write_evt_zwp_linux_dmabuf_feedback_v1_done(dst, feedback);
                }),
                &[&t1.0, &t2.0, &t3.0, &t4.0, &t5.0],
            )
            .unwrap();
        let map = process_linux_dmabuf_feedback(ret.0, ret.1, &mut format_table, feedback);
        println!("Round 3: {:?}", map);
        for f in formats {
            assert!(map.contains_key(&f), "missing {:x}", f);
        }
    })?;

    Ok(StatusOk::Pass)
}

/** Test that Waypipe correctly processes linux-dmabuf format and modifier
 * advertisements for linux-dmabuf version ≤ 3 */
#[cfg(feature = "dmabuf")]
fn proto_dmabuf_pre_v4(info: TestInfo, device: RenderDevice) -> TestResult {
    let Ok(vulk) = setup_vulkan(device.id) else {
        return Ok(StatusOk::Skipped);
    };

    let supported_modifier: u64 = 0x0;
    let unsupported_modifier: u64 = 0x1;
    let unsupported_format: u32 = 0x0;
    let formats: [u32; 5] = [
        wayland_to_drm(WlShmFormat::R8),
        wayland_to_drm(WlShmFormat::Rgb565),
        wayland_to_drm(WlShmFormat::Argb8888),
        wayland_to_drm(WlShmFormat::Xrgb8888),
        unsupported_format, /* unsupported */
    ];

    let mut ext_mods = Vec::from(vulk.get_supported_modifiers(formats[2]));
    assert!(ext_mods.contains(&supported_modifier));
    ext_mods.insert(0, unsupported_modifier);

    let modifiers: [&[u64]; 5] = [
        &[supported_modifier],
        &[supported_modifier, unsupported_modifier],
        &ext_mods,
        &[unsupported_modifier],
        &[supported_modifier],
    ];

    println!("Initial formats: {:?}", formats);
    println!("Initial modifiers: {:?}", modifiers);

    run_protocol_test_with_drm_node(&info, &device, &|mut ctx: ProtocolTestContext| {
        /* version 3 introduced zwp_linux_dmabuf_v1::modifier */
        let [display, registry, dmabuf_v1, dmabuf_v3, ..] = ID_SEQUENCE;

        ctx.prog_write_passthrough(build_msgs(|dst| {
            write_req_wl_display_get_registry(dst, display, registry);
        }));

        ctx.comp_write_passthrough(build_msgs(|dst| {
            write_evt_wl_registry_global(dst, registry, 1, ZWP_LINUX_DMABUF_V1, 1);
            write_evt_wl_registry_global(dst, registry, 2, ZWP_LINUX_DMABUF_V1, 3);
        }));

        ctx.prog_write_passthrough(build_msgs(|dst| {
            write_req_wl_registry_bind(dst, registry, 1, ZWP_LINUX_DMABUF_V1, 1, dmabuf_v1);
            write_req_wl_registry_bind(dst, registry, 2, ZWP_LINUX_DMABUF_V1, 3, dmabuf_v3);
        }));

        let msgs_v1 = build_msgs(|dst| {
            for f in formats.iter() {
                write_evt_zwp_linux_dmabuf_v1_format(dst, dmabuf_v1, *f);
            }
        });
        let (rmsgs_v1, rfds_v1) = ctx.comp_write(&msgs_v1, &[]).unwrap();
        assert!(rfds_v1.is_empty());
        let mut recvd_fmts_v1 = BTreeSet::<u32>::new();
        for msg in rmsgs_v1 {
            assert!(
                parse_wl_header(&msg)
                    == (
                        dmabuf_v1,
                        length_evt_zwp_linux_dmabuf_v1_format(),
                        OPCODE_ZWP_LINUX_DMABUF_V1_FORMAT.code()
                    )
            );
            let fmt = parse_evt_zwp_linux_dmabuf_v1_format(&msg).unwrap();
            recvd_fmts_v1.insert(fmt);
        }
        println!("Received formats for v1: {:?}", recvd_fmts_v1);
        for f in formats.iter() {
            assert!(recvd_fmts_v1.contains(f) == (*f != unsupported_format));
        }

        let msgs_v3 = build_msgs(|dst| {
            for (f, mods) in formats.iter().zip(modifiers.iter()) {
                write_evt_zwp_linux_dmabuf_v1_format(dst, dmabuf_v3, *f);
                for m in mods.iter() {
                    let (m_hi, m_lo) = split_u64(*m);
                    write_evt_zwp_linux_dmabuf_v1_modifier(dst, dmabuf_v3, *f, m_hi, m_lo);
                }
            }
        });
        let (rmsgs_v3, rfds_v3) = ctx.comp_write(&msgs_v3, &[]).unwrap();
        assert!(rfds_v3.is_empty());
        let mut recvd_fmts_v3 = BTreeSet::<u32>::new();
        let mut recvd_mods_v3 = BTreeMap::<u32, Vec<u64>>::new();
        for msg in rmsgs_v3 {
            if parse_wl_header(&msg).2 == OPCODE_ZWP_LINUX_DMABUF_V1_FORMAT.code() {
                assert!(
                    parse_wl_header(&msg)
                        == (
                            dmabuf_v3,
                            length_evt_zwp_linux_dmabuf_v1_format(),
                            OPCODE_ZWP_LINUX_DMABUF_V1_FORMAT.code()
                        )
                );
                let fmt = parse_evt_zwp_linux_dmabuf_v1_format(&msg).unwrap();
                recvd_fmts_v3.insert(fmt);
            } else {
                assert!(
                    parse_wl_header(&msg)
                        == (
                            dmabuf_v3,
                            length_evt_zwp_linux_dmabuf_v1_modifier(),
                            OPCODE_ZWP_LINUX_DMABUF_V1_MODIFIER.code()
                        )
                );
                let (fmt, mod_hi, mod_lo) = parse_evt_zwp_linux_dmabuf_v1_modifier(&msg).unwrap();
                recvd_mods_v3
                    .entry(fmt)
                    .or_default()
                    .push(join_u64(mod_hi, mod_lo));
            }
        }
        println!("Received formats for v3: {:?}", recvd_fmts_v1);
        println!("Received modifiers for v3: {:?}", recvd_mods_v3);
        for (i, f) in formats.iter().enumerate() {
            assert!(recvd_fmts_v3.contains(f) == (*f != unsupported_format));

            if i < 3 {
                let mod_list = recvd_mods_v3.get(f);
                /* Modifiers returned depend on what Waypipe supports, which should at least
                 * include the linear modifier */
                assert!(mod_list.is_some() && mod_list.unwrap().contains(&supported_modifier));
            } else {
                assert!(!recvd_mods_v3.contains_key(f));
            }
        }
    })?;
    Ok(StatusOk::Pass)
}

/** Helper function to return data for a test image split into four quadrants with
 * different colors */
#[cfg(feature = "video")]
fn fill_blocks_xrgb(w: usize, h: usize) -> Vec<u8> {
    let bpp = 4;
    let colors: [u32; 4] = [0xaf0080ff, 0xbf00ff00, 0xcf800080, 0xdf808080];
    let mut img_data = vec![0u8; w * h * bpp];
    for y in 0..h {
        for x in 0..w {
            let by = (2 * y) / h;
            let bx = (2 * x) / w;
            let c = colors[2 * by + bx] ^ ((((x * y) % 2) as u32) * 0x00010101);
            img_data[bpp * (y * w + x)..bpp * (y * w + x) + bpp].copy_from_slice(&c.to_le_bytes());
        }
    }
    img_data
}

/** Fill a buffer with a pattern that does not have any major obvious patterns and
 * is unlikely to video-encode well */
#[cfg(feature = "video")]
fn fill_pseudorand_xrgb(w: usize, h: usize) -> Vec<u8> {
    let bpp = 4;
    let mut img_data = vec![0u8; w * h * bpp];
    let mut a: u32 = 1;
    for y in 0..h {
        for x in 0..w {
            a = (2 * a) % 16777213;
            let c = a | 0xff000000;
            img_data[bpp * (y * w + x)..bpp * (y * w + x) + bpp].copy_from_slice(&c.to_le_bytes());
        }
    }
    img_data
}

/** Helper function to create and share a DMABUF with a wl_buffer. */
#[cfg(feature = "dmabuf")]
fn create_dmabuf_and_copy(
    vulk: &Arc<VulkanDevice>,
    ctx: &mut ProtocolTestContext,
    params: ObjId,
    dmabuf: ObjId,
    buffer: ObjId,
    w: usize,
    h: usize,
    fmt: u32,
    modifier_list: &[u64],
    initial_data: &[u8],
) -> Result<(Arc<VulkanDmabuf>, Arc<VulkanDmabuf>), String> {
    let (img, planes) =
        vulkan_create_dmabuf(vulk, w as u32, h as u32, fmt, modifier_list, false).unwrap();

    /* Initialize the image with garbage data */
    let tmp_img = Arc::new(vulkan_get_buffer(vulk, img.nominal_size(None), false).unwrap());
    copy_onto_dmabuf(&img, &tmp_img, initial_data).unwrap();

    let msgs = build_msgs(|dst| {
        write_req_zwp_linux_dmabuf_v1_create_params(dst, dmabuf, params);
        for p in planes.iter() {
            let (mod_hi, mod_lo) = split_u64(p.modifier);
            write_req_zwp_linux_buffer_params_v1_add(
                dst,
                params,
                false,
                p.plane_idx,
                p.offset,
                p.stride,
                mod_hi,
                mod_lo,
            );
        }

        write_req_zwp_linux_buffer_params_v1_create_immed(
            dst, params, buffer, w as i32, h as i32, fmt, 0,
        );
    });
    let plane_fds: Vec<&OwnedFd> = planes.iter().map(|p| &p.fd).collect();
    let (rmsgs, mut rfds) = ctx
        .prog_write(&msgs[..], &plane_fds[..])
        .map_err(|_| tag!("Failed to replicate dmabuf"))?;
    drop(planes);

    let add_msgs = &rmsgs[1..rmsgs.len() - 1];
    assert!(rfds.len() == add_msgs.len());
    let create_msg = &rmsgs[rmsgs.len() - 1];
    let (_rbuf, rw, rh, rfmt, _rflags) =
        parse_req_zwp_linux_buffer_params_v1_create_immed(&create_msg[..]).unwrap();
    assert!((rw, rh, rfmt) == (w as i32, h as i32, fmt));

    let mut planes = Vec::new();
    for (fd, msg) in rfds.drain(..).zip(add_msgs.iter()) {
        let (plane_idx, offset, stride, mod_hi, mod_lo) =
            parse_req_zwp_linux_buffer_params_v1_add(msg).unwrap();
        let modifier = join_u64(mod_hi, mod_lo);
        planes.push(AddDmabufPlane {
            fd,
            plane_idx,
            offset,
            stride,
            modifier,
        });
    }

    let mirror = vulkan_import_dmabuf(vulk, planes, w as u32, h as u32, fmt, false).unwrap();
    Ok((img, mirror))
}

/** Helper function to test that video encoding works and approximately replicates test patterns;
 * the format and other video properties are specified in `opts`. */
#[cfg(feature = "video")]
fn test_dmavid_inner(
    vulk: &Arc<VulkanDevice>,
    info: &TestInfo,
    opts: &WaypipeOptions,
    sizes: &[(usize, usize)],
) -> Vec<bool> {
    let mut accurate_replication = Vec::new();
    for _ in sizes {
        accurate_replication.push(AtomicBool::new(true));
    }
    run_protocol_test_with_opts(
        info, opts, opts,
        &|mut ctx: ProtocolTestContext| {
            let (display, registry, dmabuf, comp, surface, feedback, params, buffer) = (
                ObjId(1),
                ObjId(2),
                ObjId(3),
                ObjId(4),
                ObjId(5),
                ObjId(6),
                ObjId(7),
                ObjId(8),
            );

            let supported_modifier_table = setup_linux_dmabuf(
                &mut ctx, vulk, display, registry, dmabuf, comp, surface, feedback,
            );
            let fmt = wayland_to_drm(WlShmFormat::Xrgb8888);
            let Some(modifier_list) = supported_modifier_table.get(&fmt) else {
                println!("Skipping test, format {:#08x} not supported", fmt);
                return;
            };
            // todo: run these in parallel?

            // In older libavcodec versions, the small (width <= 32, height <= 16) test sizes
            // failed with uniform (88) green channel. More recent versions enforce minimum sizes.
            for (i, (w, h)) in sizes.iter().copied().enumerate() {
                println!("Testing image transfer for WxH: {}x{}", w, h);

                let seed = fill_pseudorand_xrgb(w, h);
                let (img, mirror) = create_dmabuf_and_copy(
                    vulk,
                    &mut ctx,
                    params,
                    dmabuf,
                    buffer,
                    w,
                    h,
                    fmt,
                    modifier_list,
                    &seed,
                )
                .unwrap();

                let img_data = fill_blocks_xrgb(w, h);
                let tmp_img =
                    Arc::new(vulkan_get_buffer(vulk, img.nominal_size(None), true).unwrap());
                copy_onto_dmabuf(&img, &tmp_img, &img_data).unwrap();

                ctx.prog_write_passthrough(build_msgs(|dst| {
                    write_req_wl_surface_attach(dst, surface, buffer, 0, 0);
                    write_req_wl_surface_damage_buffer(dst, surface, 0, 0, i32::MAX, i32::MAX);
                    write_req_wl_surface_commit(dst, surface);
                }));

                let tmp_mirror =
                    Arc::new(vulkan_get_buffer(vulk, mirror.nominal_size(None), true).unwrap());
                let mir_data = copy_from_dmabuf(&mirror, &tmp_mirror).unwrap();

                let mut net_diff: u64 = 0;
                for (px_i, px_m) in img_data.chunks_exact(4).zip(mir_data.chunks_exact(4)) {
                    let (ib, ig, ir, _ix) = (px_i[0], px_i[1], px_i[2], px_i[3]);
                    let (mb, mg, mr, _mx) = (px_m[0], px_m[1], px_m[2], px_m[3]);
                    net_diff +=
                        ib.abs_diff(mb) as u64 + ig.abs_diff(mg) as u64 + ir.abs_diff(mr) as u64;
                }
                let avg_diff = net_diff / ((w * h) as u64);
                /* Allow up to +/- 32 error on flat regions, and +/- 255 on edges between colored blocks */
                let mut threshold: u64 = 32 + (255 - 32) / (w as u64) + (255 - 32) / (h as u64);
                if w == 1 && h == 1 {
                    threshold = 32;
                }
                if net_diff == 0 {
                    /* Either lossless video, or no video encoding at all */
                    println!(
                        "Perfect replication of {}x{} image, likely not using video enc/decoding",
                        w, h
                    );
                } else {
                    println!(
                        "Average difference on slightly noisy {}x{} block pattern: {}, threshold {}",
                        w,
                        h,
                        (net_diff as f32) / ((w * h) as f32),
                             threshold,
                    );
                    if avg_diff >= threshold {
                        /* XRGB8888 bytes are ordered: B G R x */
                        for (i, channel) in ["Blue", "Green", "Red"].iter().enumerate() {
                            println!("{} channel, original / replicated", channel);
                            for y in 0..h {
                                for x in 0..w {
                                    let base: [u8; 4] = img_data
                                        [(4 * y * w + 4 * x)..(4 * y * w + 4 * x + 4)]
                                        .try_into()
                                        .unwrap();
                                    let rep: [u8; 4] = mir_data
                                        [(4 * y * w + 4 * x)..(4 * y * w + 4 * x + 4)]
                                        .try_into()
                                        .unwrap();
                                    print!("{:2x}/{:2x} ", base[i], rep[i]);
                                }
                                println!();
                            }
                        }
                    }
                }

                if avg_diff > threshold {
                    accurate_replication[i].store(false, std::sync::atomic::Ordering::SeqCst);
                }
                // TODO: once video library bugs are resolved, set this
                // assert!(avg_diff < threshold);

                /* Cleanup buffer and params objects, for reuse with next image */
                ctx.prog_write_passthrough(build_msgs(|dst| {
                    write_req_zwp_linux_buffer_params_v1_destroy(dst, params);
                    write_req_wl_buffer_destroy(dst, buffer);
                }));
                ctx.comp_write_passthrough(build_msgs(|dst| {
                    write_evt_wl_display_delete_id(dst, display, params.0);
                    write_evt_wl_display_delete_id(dst, display, buffer.0);
                }));
            }
        }
    ).unwrap();

    accurate_replication
        .iter()
        .map(|x| x.load(std::sync::atomic::Ordering::SeqCst))
        .collect()
}

/** Test that video encoding works and approximately replicates test patterns. */
#[cfg(feature = "video")]
fn proto_dmavid(
    info: TestInfo,
    device: RenderDevice,
    video_format: VideoFormat,
    try_hw_dec: bool,
    try_hw_enc: bool,
    accurate_on_small_images: bool,
) -> TestResult {
    let Ok(vulk) = setup_vulkan(device.id) else {
        return Ok(StatusOk::Skipped);
    };

    let opts = WaypipeOptions {
        compression: Compression::None,
        video: VideoSetting {
            format: Some(video_format),
            bits_per_frame: None, // note: very high/low values can cause codec failure
            enc_pref: Some(if try_hw_enc {
                CodecPreference::HW
            } else {
                CodecPreference::SW
            }),
            dec_pref: Some(if try_hw_dec {
                CodecPreference::HW
            } else {
                CodecPreference::SW
            }),
        },
        title_prefix: "",
        drm_node: Some(device.id),
        device_type: device.device_type,
        wire_version: None,
    };
    println!(
        "\nTrying combination: video={:?}, try_hw_dec={}, try_hw_enc={}",
        video_format, try_hw_dec, try_hw_enc
    );
    let sizes = [(257, 240), (222, 129), (128, 333), (64, 64), (1, 1)];
    let passes = test_dmavid_inner(&vulk, &info, &opts, &sizes);

    let mut overall_pass = true;
    let mut perfect = true;

    for (pass, (w, h)) in passes.iter().zip(sizes.iter().copied()) {
        let is_small = w < 128 || h < 128;
        println!(
            "Result for video={:?}, try_hw_dec={}, try_hw_enc={}, size={:?}: {}",
            video_format,
            try_hw_dec,
            try_hw_enc,
            (w, h),
            if *pass { "pass" } else { "fail" }
        );
        if accurate_on_small_images || !is_small {
            if !pass {
                overall_pass = false;
            }
        }
        if !pass {
            perfect = false;
        }
    }
    // NOTE: as of writing, AMD hardware video decoding, and Intel hardware video
    // encoding, of size w<=32, h<=16 (w=32,h=16) images does not accurately
    // reproduce colors. Nor do x264 or vp9 software encoding/decoding for 1x1 images.
    println!(
        "Overall result for video={:?}, try_hw_dec={}, try_hw_enc={}: expected accurate on small {}; was accurate {}; accurate on all {}",
        video_format,
        try_hw_dec,
        try_hw_enc,
        accurate_on_small_images,
        overall_pass,
        perfect
    );
    assert!(overall_pass);

    if perfect {
        Ok(StatusOk::Pass)
    } else {
        Err(StatusBad::Unclear("Video replication not exact".into()))
    }
}

/** Test that very long messages are either cleanly accepted or cleanly rejected. */
fn proto_oversized(info: TestInfo) -> TestResult {
    run_protocol_test(&info, &|mut ctx: ProtocolTestContext| {
        let (display, registry) = (ObjId(1), ObjId(2));
        ctx.prog_write_passthrough(build_msgs(|dst| {
            write_req_wl_display_get_registry(dst, display, registry);
        }));

        let msg_len = (1 << 16) - 4;
        let str_length = msg_len - length_evt_wl_registry_global(0);
        assert!(length_evt_wl_registry_global(str_length) == msg_len);
        let mut msg = vec![0; msg_len];
        let mut dst = &mut msg[..];
        let long_name = vec![b'a'; str_length];
        write_evt_wl_registry_global(&mut dst, registry, 1, &long_name, 1);
        println!(
            "header: {:?}, {}",
            parse_wl_header(&msg),
            length_evt_wl_registry_global(0)
        );

        /* Waypipe should either reject or pass the message, but not hang or crash */
        let res = ctx.comp_write(&msg, &[]);
        assert!(res.is_err() || res.is_ok_and(|x| x.0.concat() == msg));
    })?;
    Ok(StatusOk::Pass)
}

/** Damage a buffer and return rectangles containing the damage.
 *
 * Individual sub-patterns are labeled by the high 4 bits of each byte, with the low 4 bits used
 * to make data harder to replicate by accident. */
fn get_diff_damage(
    iter: usize,
    base: &mut [u8],
    w: usize,
    h: usize,
    stride: usize,
    bpp: usize,
) -> Vec<(i32, i32, i32, i32)> {
    let iw: i32 = w.try_into().unwrap();
    let ih: i32 = h.try_into().unwrap();
    let mut ctr = 0;
    const CYCLE: u8 = 11;
    match iter {
        0 => {
            /* Test: large disjoint blocks, left side */
            for y in 0..h / 4 {
                for x in 0..w / 4 {
                    base[(y * stride + bpp * x)..(y * stride + bpp * x + bpp)].fill(0x10 + ctr);
                    ctr = (ctr + 1) % CYCLE;
                }
            }
            for y in (3 * h) / 4..h {
                for x in 0..w / 4 {
                    base[(y * stride + bpp * x)..(y * stride + bpp * x + bpp)].fill(0x20 + ctr);
                    ctr = (ctr + 1) % CYCLE;
                }
            }
            vec![
                (0, 0, iw / 4, ih / 4),
                (0, (3 * ih) / 4, iw / 4, (ih - (3 * ih) / 4)),
            ]
        }
        1 => {
            /* Test: sparse differences in middle */
            for y in (3 * h) / 8..(5 * h) / 8 {
                let x = y.clamp(w / 8, (7 * w) / 8);
                base[y * stride + bpp * x + bpp / 2] = 0x30 + ctr;
                ctr = (ctr + 1) % CYCLE;
            }
            vec![(0, (3 * ih) / 8, iw, (5 * ih) / 8 - (3 * ih) / 8)]
        }
        2 => {
            /* Test: large disjoint blocks, right side */
            for y in 0..h / 4 {
                for x in (w / 8)..w {
                    base[(y * stride + bpp * x)..(y * stride + bpp * x + bpp)].fill(0x40 + ctr);
                    ctr = (ctr + 1) % CYCLE;
                }
            }
            for y in (3 * h) / 4..h {
                for x in (w / 8)..w {
                    base[(y * stride + bpp * x)..(y * stride + bpp * x + bpp)].fill(0x50 + ctr);
                    ctr = (ctr + 1) % CYCLE;
                }
            }
            vec![
                (0, 0, iw, ih / 4),
                (0, (3 * ih) / 4, iw, (ih - (3 * ih) / 4)),
            ]
        }
        _ => unreachable!(),
    }
}

/** Test to check that damaged regions of shm-type `wl_buffer`s are replicated. */
fn proto_shm_damage(info: TestInfo) -> TestResult {
    run_protocol_test(&info, &|mut ctx: ProtocolTestContext| {
        let (display, registry, shm, comp, surface, pool, buffer) = (
            ObjId(1),
            ObjId(2),
            ObjId(3),
            ObjId(4),
            ObjId(5),
            ObjId(6),
            ObjId(7),
        );

        ctx.prog_write_passthrough(build_msgs(|dst| {
            write_req_wl_display_get_registry(dst, display, registry);
        }));

        ctx.comp_write_passthrough(build_msgs(|dst| {
            write_evt_wl_registry_global(dst, registry, 1, WL_SHM, 2);
            write_evt_wl_registry_global(dst, registry, 2, WL_COMPOSITOR, 6);
        }));

        ctx.prog_write_passthrough(build_msgs(|dst| {
            write_req_wl_registry_bind(dst, registry, 1, WL_SHM, 2, shm);
            write_req_wl_registry_bind(dst, registry, 2, WL_COMPOSITOR, 6, comp);
            write_req_wl_compositor_create_surface(dst, comp, surface);
        }));

        /* For wl_shm buffer replication, the format is not very important as it only affects damage calculations */
        let (w, h) = (257, 257);
        for format in [WlShmFormat::Rgb565, WlShmFormat::Argb8888] {
            let bpp = match format {
                WlShmFormat::Argb8888 => 4,
                WlShmFormat::Rgb565 => 2,
                _ => unreachable!(),
            };
            let stride = align(bpp * w, 19);
            let file_sz = h * stride;
            let mut base = vec![0xf0; file_sz];
            for (i, x) in base.iter_mut().enumerate() {
                *x = 0xf0 + (i % 11) as u8;
            }

            let buffer_fd = make_file_with_contents(&base).unwrap();
            let msg = build_msgs(|dst| {
                write_req_wl_shm_create_pool(dst, shm, false, pool, file_sz as i32);
                write_req_wl_shm_pool_create_buffer(
                    dst,
                    pool,
                    buffer,
                    0,
                    w as i32,
                    h as i32,
                    stride as i32,
                    format as u32,
                );
                write_req_wl_shm_pool_destroy(dst, pool);
                write_req_wl_surface_attach(dst, surface, buffer, 0, 0);
                write_req_wl_surface_damage_buffer(dst, surface, 0, 0, i32::MAX, i32::MAX);
                write_req_wl_surface_commit(dst, surface);
            });
            let (rmsg, mut rfd) = ctx.prog_write(&msg[..], &[&buffer_fd]).unwrap();
            assert!(rmsg.concat() == msg);
            assert!(rfd.len() == 1);
            let output_fd = rfd.remove(0);
            assert!(get_file_contents(&output_fd, file_sz).unwrap() == base);

            ctx.comp_write_passthrough(build_msgs(|dst| {
                write_evt_wl_display_delete_id(dst, display, pool.0);
            }));
            for iter in 0..3 {
                let damage: Vec<(i32, i32, i32, i32)> =
                    get_diff_damage(iter, &mut base, w, h, stride, bpp);
                update_file_contents(&buffer_fd, &base).unwrap();

                ctx.prog_write_passthrough(build_msgs(|dst| {
                    for d in damage {
                        write_req_wl_surface_damage_buffer(dst, surface, d.0, d.1, d.2, d.3);
                    }
                    write_req_wl_surface_commit(dst, surface);
                }));

                assert!(get_file_contents(&output_fd, file_sz).unwrap() == base);

                ctx.comp_write_passthrough(build_msgs(|dst| {
                    write_evt_wl_buffer_release(dst, buffer);
                }));
            }

            ctx.prog_write_passthrough(build_msgs(|dst| {
                write_req_wl_buffer_destroy(dst, buffer);
            }));
            ctx.comp_write_passthrough(build_msgs(|dst| {
                write_evt_wl_display_delete_id(dst, display, buffer.0);
            }));
        }
    })?;
    Ok(StatusOk::Pass)
}

/** Test to check that damage calculations do not severely overestimate the damaged region;
 * in particular, that even the entire buffer contents are changed, but only a small region
 * is damaged, Waypipe will act as if the stated damage is correct and only update a small
 * region.
 */
fn proto_damage_efficiency(info: TestInfo) -> TestResult {
    run_protocol_test(&info, &|mut ctx: ProtocolTestContext| {
        let [display, registry, shm, comp, surface, pool, buffer, ..] = ID_SEQUENCE;

        ctx.prog_write_passthrough(build_msgs(|dst| {
            write_req_wl_display_get_registry(dst, display, registry);
        }));
        ctx.comp_write_passthrough(build_msgs(|dst| {
            write_evt_wl_registry_global(dst, registry, 1, WL_SHM, 2);
            write_evt_wl_registry_global(dst, registry, 2, WL_COMPOSITOR, 6);
        }));

        let format = WlShmFormat::Argb8888;
        let bpp = 4;
        let (w, h): (u32, u32) = (128, 32);
        let stride = w * bpp;

        let file_sz = h * stride;
        let mut base = vec![0; file_sz as usize];
        for (i, x) in base.iter_mut().enumerate() {
            *x = 0xf0 + ((i as u32).wrapping_mul(0x01234567) >> 28) as u8;
        }
        let buffer_fd = make_file_with_contents(&base).unwrap();

        let setup_msgs = build_msgs(|dst| {
            write_req_wl_registry_bind(dst, registry, 1, WL_SHM, 2, shm);
            write_req_wl_registry_bind(dst, registry, 2, WL_COMPOSITOR, 6, comp);
            write_req_wl_compositor_create_surface(dst, comp, surface);
            write_req_wl_shm_create_pool(dst, shm, false, pool, file_sz as i32);
            write_req_wl_shm_pool_create_buffer(
                dst,
                pool,
                buffer,
                0,
                w as i32,
                h as i32,
                stride as i32,
                format as u32,
            );
            write_req_wl_shm_pool_destroy(dst, pool);
            write_req_wl_surface_attach(dst, surface, buffer, 0, 0);
            /* The initial attachment effectively damages everything */
            write_req_wl_surface_damage_buffer(dst, surface, 0, 0, i32::MAX, i32::MAX);
            write_req_wl_surface_commit(dst, surface);
        });

        let (rmsgs, mut rfds) = ctx.prog_write(&setup_msgs, &[&buffer_fd]).unwrap();
        assert!(rmsgs.concat() == setup_msgs);
        assert!(rfds.len() == 1);
        let output_fd = rfds.pop().unwrap();
        assert!(get_file_contents(&output_fd, file_sz as usize).unwrap() == base);

        for x in base.iter_mut() {
            *x = 0xff - *x;
        }
        update_file_contents(&buffer_fd, &base).unwrap();
        ctx.prog_write_passthrough(build_msgs(|dst| {
            /* Update just the opposite corners. */
            write_req_wl_surface_damage_buffer(dst, surface, 0, 0, 1, 1);
            write_req_wl_surface_damage_buffer(dst, surface, w as i32 - 1, h as i32 - 1, 1, 1);
            write_req_wl_surface_commit(dst, surface);
        }));

        let mut n_updated = 0;
        let updated = get_file_contents(&output_fd, file_sz as usize).unwrap();
        for (u, b) in updated.iter().zip(base.iter()) {
            if u == b {
                n_updated += 1;
            }
        }
        println!(
            "Updated bytes: {} of ideal {}/{}",
            n_updated,
            2 * bpp,
            file_sz
        );
        assert!(n_updated <= file_sz / 8);
    })?;
    Ok(StatusOk::Pass)
}

/** Test to check that damaged regions of DMABUF-type `wl_buffer`s are replicated. */
#[cfg(feature = "dmabuf")]
fn proto_dmabuf_damage(info: TestInfo, device: RenderDevice) -> TestResult {
    let Ok(vulk) = setup_vulkan(device.id) else {
        return Ok(StatusOk::Skipped);
    };

    run_protocol_test_with_drm_node(&info, &device, &|mut ctx: ProtocolTestContext| {
        let (display, registry, dmabuf, comp, surface, feedback, params, buffer) = (
            ObjId(1),
            ObjId(2),
            ObjId(3),
            ObjId(4),
            ObjId(5),
            ObjId(6),
            ObjId(7),
            ObjId(8),
        );

        let supported_modifier_table = setup_linux_dmabuf(
            &mut ctx, &vulk, display, registry, dmabuf, comp, surface, feedback,
        );

        /* For dmabuf replication, the format (well, texel size) affects diff alignment */
        for (w, h) in [(64_usize, 64_usize), (257_usize, 257_usize)] {
            for wl_format in [
                WlShmFormat::R8,
                WlShmFormat::Rgb565,
                WlShmFormat::Argb8888,
                WlShmFormat::Abgr16161616,
            ] {
                let bpp = match wl_format {
                    WlShmFormat::Abgr16161616 => 8,
                    WlShmFormat::Argb8888 => 4,
                    WlShmFormat::Rgb565 => 2,
                    WlShmFormat::R8 => 1,
                    _ => unreachable!(),
                };
                let format = wayland_to_drm(wl_format);
                let stride = bpp * w;
                let file_sz = h * stride;

                let Some(modifier_list) = supported_modifier_table.get(&format) else {
                    println!("Skipping test, format {:#08x} not supported", format);
                    continue;
                };

                let mut base = vec![0xf0; file_sz];
                for (i, x) in base.iter_mut().enumerate() {
                    *x = 0xf0 + (i % 11) as u8;
                }

                let (img, mirror) = create_dmabuf_and_copy(
                    &vulk,
                    &mut ctx,
                    params,
                    dmabuf,
                    buffer,
                    w,
                    h,
                    format,
                    modifier_list,
                    &base,
                )
                .unwrap();

                ctx.prog_write_passthrough(build_msgs(|dst| {
                    write_req_wl_surface_attach(dst, surface, buffer, 0, 0);
                    write_req_wl_surface_damage_buffer(dst, surface, 0, 0, i32::MAX, i32::MAX);
                    write_req_wl_surface_commit(dst, surface);
                }));

                let tmp_wr =
                    Arc::new(vulkan_get_buffer(&vulk, img.nominal_size(None), false).unwrap());
                let tmp_rd =
                    Arc::new(vulkan_get_buffer(&vulk, img.nominal_size(None), true).unwrap());

                let dup = copy_from_dmabuf(&mirror, &tmp_rd).unwrap();

                fn freq_counts(s: &[u8]) -> Vec<usize> {
                    let mut c = vec![0; 256];
                    for x in s {
                        c[*x as usize] += 1;
                    }
                    c
                }
                assert!(
                    dup == base,
                    "initial mismatch {} {}\n{:?}\n{:?}\n{:?}\n{:?}",
                    dup.len(),
                    base.len(),
                    &dup,
                    &base,
                    freq_counts(&dup),
                    freq_counts(&base),
                );

                for iter in 0..3 {
                    let damage: Vec<(i32, i32, i32, i32)> =
                        get_diff_damage(iter, &mut base, w, h, stride, bpp);
                    copy_onto_dmabuf(&img, &tmp_wr, &base).unwrap();

                    ctx.prog_write_passthrough(build_msgs(|dst| {
                        for d in damage {
                            write_req_wl_surface_damage_buffer(dst, surface, d.0, d.1, d.2, d.3);
                        }
                        write_req_wl_surface_commit(dst, surface);
                    }));

                    let dup = copy_from_dmabuf(&mirror, &tmp_rd).unwrap();
                    assert!(
                        dup == base,
                        "mismatch iter {}, {} {}\n{:?}\n{:?}",
                        iter,
                        dup.len(),
                        base.len(),
                        dup,
                        base
                    );

                    ctx.comp_write_passthrough(build_msgs(|dst| {
                        write_evt_wl_buffer_release(dst, buffer);
                    }));
                }

                ctx.prog_write_passthrough(build_msgs(|dst| {
                    write_req_zwp_linux_buffer_params_v1_destroy(dst, params);
                    write_req_wl_buffer_destroy(dst, buffer);
                }));
                ctx.comp_write_passthrough(build_msgs(|dst| {
                    write_evt_wl_display_delete_id(dst, display, params.0);
                    write_evt_wl_display_delete_id(dst, display, buffer.0);
                }));
            }
        }
    })?;
    Ok(StatusOk::Pass)
}

/** Test to verify the damage is correctly calculated (or at least, overestimated)
 * when using wp_viewport together with wl_surface.damage */
fn proto_viewporter_damage(info: TestInfo) -> TestResult {
    run_protocol_test(&info, &|mut ctx: ProtocolTestContext| {
        let [display, registry, shm, comp, viewporter, surface, pool, buffer, viewport, ..] =
            ID_SEQUENCE;

        let scale: u32 = 2;
        let w: u32 = 50 * scale;
        let h: u32 = 7 * scale;
        let fmt = WlShmFormat::Argb8888;
        let bpp = 4;
        let stride = w * bpp;

        ctx.prog_write_passthrough(build_msgs(|dst| {
            write_req_wl_display_get_registry(dst, display, registry);
        }));

        ctx.comp_write_passthrough(build_msgs(|dst| {
            write_evt_wl_registry_global(dst, registry, 1, WL_SHM, 2);
            write_evt_wl_registry_global(dst, registry, 2, WL_COMPOSITOR, 6);
            write_evt_wl_registry_global(dst, registry, 3, WP_VIEWPORTER, 1);
        }));

        let file_sz: usize = (w * h * bpp) as usize;
        let mut base = vec![0x01; file_sz];

        let buffer_fd = make_file_with_contents(&base).unwrap();
        let msg = build_msgs(|dst| {
            write_req_wl_registry_bind(dst, registry, 1, WL_SHM, 2, shm);
            write_req_wl_registry_bind(dst, registry, 2, WL_COMPOSITOR, 6, comp);
            write_req_wl_registry_bind(dst, registry, 3, WP_VIEWPORTER, 1, viewporter);
            write_req_wl_compositor_create_surface(dst, comp, surface);
            write_req_wl_shm_create_pool(dst, shm, false, pool, file_sz as i32);
            write_req_wl_shm_pool_create_buffer(
                dst,
                pool,
                buffer,
                0,
                w as i32,
                h as i32,
                (w * bpp) as i32,
                fmt as u32,
            );
            write_req_wp_viewporter_get_viewport(dst, viewporter, viewport, surface);
            write_req_wl_surface_set_buffer_scale(dst, surface, scale as i32);
            write_req_wl_surface_attach(dst, surface, buffer, 0, 0);
        });
        let (rmsg, mut rfd) = ctx.prog_write(&msg[..], &[&buffer_fd]).unwrap();
        assert!(rmsg.concat() == msg);
        assert!(rfd.len() == 1);
        let output_fd = rfd.remove(0);

        /* First, ensure replicated buffer matches */
        ctx.prog_write_passthrough(build_msgs(|dst| {
            write_req_wl_surface_damage(dst, surface, 0, 0, i32::MAX, i32::MAX);
            write_req_wl_surface_commit(dst, surface);
        }));
        assert!(get_file_contents(&output_fd, file_sz).unwrap() == base);

        /* Scale-only test */
        ctx.prog_write_passthrough(build_msgs(|dst| {
            /* Set viewport parameters (which effectively invalidates buffer contents,
             * making Waypipe replicate the entire visible buffer contents */
            write_req_wp_viewport_set_destination(dst, viewport, 200, 200);
            write_req_wl_surface_damage(dst, surface, 0, 0, i32::MAX, i32::MAX);
            write_req_wl_surface_commit(dst, surface);
        }));
        for x in w - 2..w {
            for y in h - 1..h {
                let o = (y * stride + x * bpp) as usize;
                base[o..o + bpp as usize].copy_from_slice(&0x11223344_u32.to_le_bytes());
            }
        }
        update_file_contents(&buffer_fd, &base).unwrap();
        ctx.prog_write_passthrough(build_msgs(|dst| {
            /* Now that viewport parameters are set, _this_ damage should be efficiently
             * tracked */
            write_req_wl_surface_damage(dst, surface, 196, 196, 4, 4);
            write_req_wl_surface_commit(dst, surface);
        }));
        assert!(get_file_contents(&output_fd, file_sz).unwrap() == base);

        /* Crop-only test */
        ctx.prog_write_passthrough(build_msgs(|dst| {
            write_req_wp_viewport_set_destination(dst, viewport, -1, -1);
            write_req_wp_viewport_set_source(
                dst,
                viewport,
                24 * 256 + 128,
                4 * 256,
                7 * 256,
                3 * 256,
            );
            write_req_wl_surface_damage(dst, surface, 0, 0, i32::MAX, i32::MAX);
            write_req_wl_surface_commit(dst, surface);
        }));
        for x in 25 * scale..(24 + 8) * scale {
            for y in 5 * scale..7 * scale {
                let o = (y * stride + x * bpp) as usize;
                base[o..o + bpp as usize].copy_from_slice(&0xaabbccdd_u32.to_le_bytes());
            }
        }
        update_file_contents(&buffer_fd, &base).unwrap();
        ctx.prog_write_passthrough(build_msgs(|dst| {
            write_req_wl_surface_damage(dst, surface, 1, 1, i32::MAX, i32::MAX);
            write_req_wl_surface_commit(dst, surface);
        }));
        assert!(get_file_contents(&output_fd, file_sz).unwrap() == base);

        /* Combination scale and crop test */
        ctx.prog_write_passthrough(build_msgs(|dst| {
            write_req_wp_viewport_set_source(
                dst,
                viewport,
                7 * 256 + 128 + 1,
                1,
                7 * 256 + 254,
                6 * 256 + 254,
            );
            write_req_wp_viewport_set_destination(dst, viewport, 2, 2);
            write_req_wl_surface_damage(dst, surface, 0, 0, i32::MAX, i32::MAX);
            write_req_wl_surface_commit(dst, surface);
        }));
        for x in (7 + 3) * scale..(7 + 7) * scale {
            for y in 3 * scale..7 * scale {
                let o = (y * stride + x * bpp) as usize;
                base[o..o + bpp as usize].copy_from_slice(&0x12345678_u32.to_le_bytes());
            }
        }
        update_file_contents(&buffer_fd, &base).unwrap();
        ctx.prog_write_passthrough(build_msgs(|dst| {
            write_req_wl_surface_damage(dst, surface, 1, 1, 1, 1);
            write_req_wl_surface_commit(dst, surface);
        }));
        assert!(get_file_contents(&output_fd, file_sz).unwrap() == base);

        /* Test destroying the viewport */
        ctx.prog_write_passthrough(build_msgs(|dst| {
            write_req_wp_viewport_destroy(dst, viewport);
            write_req_wl_surface_damage(dst, surface, 0, 0, i32::MAX, i32::MAX);
            write_req_wl_surface_commit(dst, surface);
        }));
        for x in 20 * scale..21 * scale {
            for y in scale..2 * scale {
                let o = (y * stride + x * bpp) as usize;
                base[o..o + bpp as usize].copy_from_slice(&0x22222222_u32.to_le_bytes());
            }
        }
        update_file_contents(&buffer_fd, &base).unwrap();
        ctx.prog_write_passthrough(build_msgs(|dst| {
            write_req_wl_surface_damage(dst, surface, 20, 1, 1, 1);
            write_req_wl_surface_commit(dst, surface);
        }));
        assert!(get_file_contents(&output_fd, file_sz).unwrap() == base);

        ctx.prog_write_passthrough(build_msgs(|dst| {
            write_req_wl_surface_destroy(dst, surface);
        }));
    })?;
    Ok(StatusOk::Pass)
}

/** Test that wl_fixes::destroy_registry works without issues */
fn proto_fixes(info: TestInfo) -> TestResult {
    run_protocol_test(&info, &|mut ctx: ProtocolTestContext| {
        let [display, registry, fixes, compositor, viewporter, viewport, ..] = ID_SEQUENCE;

        ctx.prog_write_passthrough(build_msgs(|dst| {
            write_req_wl_display_get_registry(dst, display, registry);
        }));

        ctx.comp_write_passthrough(build_msgs(|dst| {
            write_evt_wl_registry_global(dst, registry, 1, WL_COMPOSITOR, 6);
            write_evt_wl_registry_global(dst, registry, 2, WL_FIXES, 1);
            write_evt_wl_registry_global(dst, registry, 3, WP_VIEWPORTER, 1);
        }));

        ctx.prog_write_passthrough(build_msgs(|dst| {
            write_req_wl_registry_bind(dst, registry, 2, WL_FIXES, 1, fixes);
            write_req_wl_registry_bind(dst, registry, 1, WL_COMPOSITOR, 6, compositor);
            write_req_wl_registry_bind(dst, registry, 3, WP_VIEWPORTER, 1, viewporter);
            write_req_wl_fixes_destroy_registry(dst, fixes, registry);
        }));

        ctx.comp_write_passthrough(build_msgs(|dst| {
            write_evt_wl_display_delete_id(dst, display, registry.0);
        }));

        // Create and use a new object which recycles the old registry ID
        let surface = registry;
        ctx.prog_write_passthrough(build_msgs(|dst| {
            write_req_wl_compositor_create_surface(dst, compositor, surface);
            write_req_wp_viewporter_get_viewport(dst, viewporter, viewport, surface);
            write_req_wl_fixes_destroy(dst, fixes);
        }));
    })?;
    Ok(StatusOk::Pass)
}

/** Test that the entire buffer is updated when a transform change implies buffer content
 * changes. Flipping the way the buffer is linked to the surface may require changing its
 * contents when there is no damage -- because damage is defined as the change to _surface_
 * contents, not that to buffer contents */
fn proto_flip_damage(info: TestInfo) -> TestResult {
    run_protocol_test(&info, &|mut ctx: ProtocolTestContext| {
        let [display, registry, shm, comp, surface, pool, buffer, ..] = ID_SEQUENCE;

        let w: u32 = 80;
        let h: u32 = 2;
        let fmt = WlShmFormat::R8;

        let mut local_data = vec![0x00; (w * h) as usize];
        for x in 0..w / 2 {
            for y in 0..h {
                local_data[(y * w + x) as usize] = 0xff;
            }
        }

        ctx.prog_write_passthrough(build_msgs(|dst| {
            write_req_wl_display_get_registry(dst, display, registry);
        }));
        ctx.comp_write_passthrough(build_msgs(|dst| {
            write_evt_wl_registry_global(dst, registry, 1, WL_SHM, 2);
            write_evt_wl_registry_global(dst, registry, 2, WL_COMPOSITOR, 6);
        }));
        let local_fd = make_file_with_contents(&local_data).unwrap();
        let msg = build_msgs(|dst| {
            write_req_wl_registry_bind(dst, registry, 1, WL_SHM, 2, shm);
            write_req_wl_registry_bind(dst, registry, 2, WL_COMPOSITOR, 6, comp);
            write_req_wl_compositor_create_surface(dst, comp, surface);
            write_req_wl_shm_create_pool(dst, shm, false, pool, (w * h) as i32);
            write_req_wl_shm_pool_create_buffer(
                dst, pool, buffer, 0, w as i32, h as i32, w as i32, fmt as u32,
            );
            write_req_wl_surface_attach(dst, surface, buffer, 0, 0);
            write_req_wl_surface_damage(dst, surface, 0, 0, i32::MAX, i32::MAX);
            write_req_wl_surface_commit(dst, surface);
        });
        let (rmsg, mut rfd) = ctx.prog_write(&msg[..], &[&local_fd]).unwrap();
        assert!(rmsg.concat() == msg);
        assert!(rfd.len() == 1);
        let remote_fd = rfd.remove(0);
        assert!(get_file_contents(&remote_fd, (w * h) as usize).unwrap() == local_data);

        /* Flip the buffer and its transform horizontally, but do not report any damage,
         * since the pending buffer still agrees with the surface contents. */
        local_data.fill(0xff);
        for x in 0..w / 2 {
            for y in 0..h {
                local_data[(y * w + x) as usize] = 0x00;
            }
        }
        update_file_contents(&local_fd, &local_data).unwrap();
        ctx.prog_write_passthrough(build_msgs(|dst| {
            write_req_wl_surface_set_buffer_transform(
                dst,
                surface,
                WlOutputTransform::Flipped as i32,
            );
            write_req_wl_surface_commit(dst, surface);
        }));
        let remote_data = get_file_contents(&remote_fd, (w * h) as usize).unwrap();
        assert!(
            remote_data == local_data,
            "{:?} {:?}",
            remote_data,
            local_data
        );
    })?;
    Ok(StatusOk::Pass)
}

/** Test that damage tracking is done _correctly_ when the surface transform and scale change.
 *
 * Design: with pseudorandomly transformed and scaled underlying buffers, render a scene in
 * which a sequence of pixels (in surface coordinates) are drawn, using precise damage
 * tracking to indicate in either surface or buffer coordinates what has changed. If Waypipe
 * correctly and efficiently tracks damage, it should always successfully replicate the
 * buffers.
 */
fn proto_rotating_damage(info: TestInfo) -> TestResult {
    let transforms = [
        WlOutputTransform::Normal,
        WlOutputTransform::Item90,
        WlOutputTransform::Item180,
        WlOutputTransform::Item270,
        WlOutputTransform::Flipped,
        WlOutputTransform::Flipped90,
        WlOutputTransform::Flipped180,
        WlOutputTransform::Flipped270,
    ];

    /** Transform pixel from buffer coordinates to surface coordinates,
     * assuming scale 1.
     *
     * `buf_sz` is the size of the buffer.
     *
     * `transform` is the transform passed to `wl_surface::set_buffer_transform`, which
     * is the "transformation the client has already applied to the content of the buffer".
     * As a result, when moving from buffer to surface, one needs to do the _inverse_ of the
     * operation that the transform says to do.
     */
    fn buf_to_surface_tx(
        transform: WlOutputTransform,
        mut px: (i32, i32),
        buf_sz: (u32, u32),
    ) -> (i32, i32) {
        let s: (i32, i32) = (buf_sz.0.try_into().unwrap(), buf_sz.1.try_into().unwrap());
        px = match transform {
            WlOutputTransform::Normal | WlOutputTransform::Flipped => (px.0, px.1),
            WlOutputTransform::Item90 | WlOutputTransform::Flipped90 => (s.1 - 1 - px.1, px.0),
            WlOutputTransform::Item180 | WlOutputTransform::Flipped180 => {
                (s.0 - 1 - px.0, s.1 - 1 - px.1)
            }
            WlOutputTransform::Item270 | WlOutputTransform::Flipped270 => (px.1, s.0 - 1 - px.0),
        };
        match transform {
            WlOutputTransform::Normal
            | WlOutputTransform::Item90
            | WlOutputTransform::Item180
            | WlOutputTransform::Item270 => px,
            WlOutputTransform::Flipped | WlOutputTransform::Flipped180 => (s.0 - 1 - px.0, px.1),
            WlOutputTransform::Flipped90 | WlOutputTransform::Flipped270 => (s.1 - 1 - px.0, px.1),
        }
    }
    /** Like buf_to_surface_tx; here the transform operation is applied to the input pixel
     * within a surface.
     *
     * `buf_sz` is the size of the _buffer_, not the surface. */
    fn surface_to_buf_tx(
        transform: WlOutputTransform,
        mut px: (i32, i32),
        buf_sz: (u32, u32),
    ) -> (i32, i32) {
        let transpose = match transform {
            WlOutputTransform::Normal
            | WlOutputTransform::Item180
            | WlOutputTransform::Flipped
            | WlOutputTransform::Flipped180 => false,
            WlOutputTransform::Item90
            | WlOutputTransform::Item270
            | WlOutputTransform::Flipped90
            | WlOutputTransform::Flipped270 => true,
        };
        let surf_sz: (i32, i32) = if transpose {
            (buf_sz.1.try_into().unwrap(), buf_sz.0.try_into().unwrap())
        } else {
            (buf_sz.0.try_into().unwrap(), buf_sz.1.try_into().unwrap())
        };
        /* flip over vertical axis happens before rotation */
        px = match transform {
            WlOutputTransform::Normal
            | WlOutputTransform::Item90
            | WlOutputTransform::Item180
            | WlOutputTransform::Item270 => px,
            /* Subtract 1 when flipping since pixels are actually [(x,x+1,y,y+1)]
             * rectangles whose bounds swap on subtraction. */
            WlOutputTransform::Flipped
            | WlOutputTransform::Flipped90
            | WlOutputTransform::Flipped180
            | WlOutputTransform::Flipped270 => (surf_sz.0 - 1 - px.0, px.1),
        };
        match transform {
            WlOutputTransform::Normal | WlOutputTransform::Flipped => (px.0, px.1),
            /* contents rotated 90 deg ccw around the top left corner (0,0)
             *
             *   surface     buffer
             * 0--------X    0---X
             * |        |    |  *|
             * |      * | -> |   |
             * X--------X    |   |
             *               |   |
             *               X---X
             */
            WlOutputTransform::Item90 | WlOutputTransform::Flipped90 => {
                (px.1, surf_sz.0 - 1 - px.0)
            }
            WlOutputTransform::Item180 | WlOutputTransform::Flipped180 => {
                (surf_sz.0 - 1 - px.0, surf_sz.1 - 1 - px.1)
            }
            WlOutputTransform::Item270 | WlOutputTransform::Flipped270 => {
                (surf_sz.1 - 1 - px.1, px.0)
            }
        }
    }

    /* Sanity check, that transforms work correctly */
    for t in transforms {
        let p = (1, 2);
        let q = buf_to_surface_tx(t, p, (100, 200));
        let r = surface_to_buf_tx(t, q, (100, 200));
        println!("{:?} {:?} {:?} {:?}", t, p, q, r);
        assert!(p == r);
    }

    fn draw_pixel(
        region: &mut [u8],
        buf_size: (u32, u32),
        bpp: u32,
        spx: (i32, i32),
        scale: u32,
        transform: WlOutputTransform,
        value: u16,
    ) -> Vec<(u32, u32)> {
        assert!(buf_size.0 % scale == 0 && buf_size.1 % scale == 0);
        let bpx = surface_to_buf_tx(transform, spx, (buf_size.0 / scale, buf_size.1 / scale));

        println!(
            "Draw pixel: {:?} {} {:?}, surface {:?} -> buffer {:?}",
            transform,
            scale,
            (buf_size.0 / scale, buf_size.1 / scale),
            spx,
            (bpx.0 * scale as i32, bpx.1 * scale as i32),
        );
        let mut mod_pixels = Vec::new();
        if bpx.0 >= 0 && bpx.1 >= 0 {
            for x in (bpx.0 as u32) * scale..(bpx.0 as u32 + 1) * scale {
                for y in (bpx.1 as u32) * scale..(bpx.1 as u32 + 1) * scale {
                    if x >= buf_size.0 || y >= buf_size.1 {
                        continue;
                    }
                    region[(y * buf_size.0 * bpp + x * bpp) as usize
                        ..(y * buf_size.0 * bpp + x * bpp + bpp) as usize]
                        .copy_from_slice(&value.to_le_bytes());
                    mod_pixels.push((x, y));
                }
            }
        }
        mod_pixels
    }

    run_protocol_test(&info, &|mut ctx: ProtocolTestContext| {
        let [display, registry, shm, comp, surface, pool, buffer_base, ..] = ID_SEQUENCE;

        let mut rng = BadRng { state: 1 };

        let fmt = WlShmFormat::Rgb565;
        let bpp = 2;
        let pixel_s = 20; /* surface square size for pixel selection */
        let s = 2 * pixel_s + 30; /* So that all pixels fall inside a scale 2 surface */
        assert!(s * bpp >= 64);
        /* A randomly shuffled sequence which uses every transform+scale combination at least once */
        let buffer_sequence: &[(usize, u32, u32, u32)] = &[
            /* transform idx, scale, width, height */
            (0, 1, s, s), /* seed, default setup */
            (5, 2, s, s),
            (5, 1, s + 2, s - 2),
            (3, 2, s + 4, s - 4),
            (0, 1, s + 6, s - 6),
            (6, 1, s + 8, s - 8),
            (0, 2, s + 10, s - 10),
            (7, 2, s + 12, s - 12),
            (2, 1, s + 14, s - 14),
            (2, 2, s + 16, s - 16),
            (1, 1, s + 18, s - 18),
            (7, 1, s + 20, s - 20),
            (6, 2, s + 22, s - 22),
            (3, 1, s + 24, s - 24),
            (4, 1, s + 26, s - 26),
            (4, 2, s + 28, s - 28),
            (1, 2, s + 30, s - 30),
        ];
        let mut offsets: Vec<u32> = Vec::new();
        offsets.push(0);
        let mut file_sz = 0;
        for b in buffer_sequence {
            file_sz += (bpp * b.2 * b.3) as usize;
            offsets.push(file_sz as u32);
        }

        /* Setup: zero initialize three wl_buffers */
        let mut local_data = vec![0x01; file_sz];
        ctx.prog_write_passthrough(build_msgs(|dst| {
            write_req_wl_display_get_registry(dst, display, registry);
        }));
        ctx.comp_write_passthrough(build_msgs(|dst| {
            write_evt_wl_registry_global(dst, registry, 1, WL_SHM, 2);
            write_evt_wl_registry_global(dst, registry, 2, WL_COMPOSITOR, 6);
        }));
        let local_fd = make_file_with_contents(&local_data).unwrap();
        let msg = build_msgs(|dst| {
            write_req_wl_registry_bind(dst, registry, 1, WL_SHM, 2, shm);
            write_req_wl_registry_bind(dst, registry, 2, WL_COMPOSITOR, 6, comp);
            write_req_wl_compositor_create_surface(dst, comp, surface);
            write_req_wl_shm_create_pool(dst, shm, false, pool, file_sz as i32);
            for i in 0..buffer_sequence.len() {
                write_req_wl_shm_pool_create_buffer(
                    dst,
                    pool,
                    ObjId(buffer_base.0 + i as u32),
                    offsets[i] as i32,
                    buffer_sequence[i].2 as i32,
                    buffer_sequence[i].3 as i32,
                    (buffer_sequence[i].2 * bpp) as i32,
                    fmt as u32,
                );
            }
            write_req_wl_surface_attach(dst, surface, buffer_base, 0, 0);
            write_req_wl_surface_damage(dst, surface, 0, 0, i32::MAX, i32::MAX);
            write_req_wl_surface_commit(dst, surface);
        });
        let (rmsg, mut rfd) = ctx.prog_write(&msg[..], &[&local_fd]).unwrap();
        assert!(rmsg.concat() == msg);
        assert!(rfd.len() == 1);
        let remote_fd = rfd.remove(0);
        let remote_data = get_file_contents(&remote_fd, file_sz).unwrap();
        /* Check that the seed buffer was correctly replicated; others have not yet been used and may have arbitrary contents */
        assert!(
            remote_data[offsets[0] as usize..offsets[1] as usize]
                == local_data[offsets[0] as usize..offsets[1] as usize]
        );

        let mut px_seq = Vec::new();

        for step in 0..(3 * buffer_sequence.len() - 4) {
            let px = (
                (rng.next() as u32 % pixel_s) as i32,
                (rng.next() as u32 % pixel_s) as i32,
            );
            px_seq.push(px);

            /* Sequence; 0 121 232 343 ... 15.16.15 + 16 */
            let buf_no = if step == 0 {
                0
            } else {
                let b = 1 + (step - 1) / 3;
                let o = ((step - 1) % 3 == 1) as usize;
                b + o
            };

            let on_surface = rng.next() % 2 == 0;
            let transform = transforms[buffer_sequence[buf_no].0];
            let scale = buffer_sequence[buf_no].1;
            let buf_size = (buffer_sequence[buf_no].2, buffer_sequence[buf_no].3);
            println!(
                "Step {}: buffer {}, scale {}, transform {:?}, size {:?}, px {:?}",
                step, buf_no, scale, transform, buf_size, px
            );

            /* extract sub-buffer contents ? but note the stride difference, and that the pixel may miss */
            let local_region =
                &mut local_data[offsets[buf_no] as usize..offsets[buf_no + 1] as usize];

            for (step, p) in px_seq[..px_seq.len() - 1].iter().enumerate() {
                /* draw entire past pixel sequence; since each buffer only ever uses the same
                 * scale/transform parameters the data will be consistent between commits. */
                draw_pixel(
                    local_region,
                    buf_size,
                    bpp,
                    *p,
                    scale,
                    transform,
                    (step + 100) as u16,
                );
            }
            let mod_pixels = draw_pixel(
                local_region,
                buf_size,
                bpp,
                px,
                scale,
                transform,
                (step + 100) as u16,
            );
            /* If pixel is not drawn this round, its damage can be ignored and Waypipe should
             * not be expected to synchronize the not-actually-"damaged" spot for later commits. */
            assert!(!mod_pixels.is_empty());

            let cfg = build_msgs(|dst| {
                write_req_wl_surface_attach(
                    dst,
                    surface,
                    ObjId(buffer_base.0 + buf_no as u32),
                    0,
                    0,
                );
                write_req_wl_surface_set_buffer_scale(dst, surface, scale as i32);
                write_req_wl_surface_set_buffer_transform(dst, surface, transform as i32);
            });

            update_file_contents(&local_fd, &local_data).unwrap();
            if on_surface {
                /* plain wl_surface::damage; just mark old and new pixel positions */
                ctx.prog_write_passthrough(
                    [
                        cfg,
                        build_msgs(|dst| {
                            write_req_wl_surface_damage(dst, surface, px.0, px.1, 1, 1);
                            write_req_wl_surface_commit(dst, surface);
                        }),
                    ]
                    .concat(),
                );
            } else {
                /* damage_buffer: compute difference between old and new images, and
                 * report changed pixels from those */
                ctx.prog_write_passthrough(
                    [
                        cfg,
                        build_msgs(|dst| {
                            for (x, y) in mod_pixels {
                                write_req_wl_surface_damage_buffer(
                                    dst, surface, x as i32, y as i32, 1, 1,
                                );
                            }
                            write_req_wl_surface_commit(dst, surface);
                        }),
                    ]
                    .concat(),
                );
            }

            let remote_data = get_file_contents(&remote_fd, file_sz).unwrap();
            let remote_region =
                &remote_data[offsets[buf_no] as usize..offsets[buf_no + 1] as usize];
            let local_region = &local_data[offsets[buf_no] as usize..offsets[buf_no + 1] as usize];

            assert!(remote_region == local_region);
        }
    })?;
    Ok(StatusOk::Pass)
}

/** Test that timeline semaphores for the linux-drm-syncobj-v1 protocol are correctly handled. */
#[cfg(feature = "dmabuf")]
fn proto_explicit_sync(info: TestInfo, device: RenderDevice) -> TestResult {
    if matches!(device.device_type, RenderDeviceType::Gbm) {
        return Ok(StatusOk::Skipped);
    }
    let Ok(vulk) = setup_vulkan(device.id) else {
        return Ok(StatusOk::Skipped);
    };

    run_protocol_test_with_drm_node(&info, &device, &|mut ctx: ProtocolTestContext| {
        let (
            display,
            registry,
            dmabuf,
            comp,
            surface,
            feedback,
            params,
            manager,
            sync_surf,
            timeline,
            buffer,
        ) = (
            ObjId(1),
            ObjId(2),
            ObjId(3),
            ObjId(4),
            ObjId(5),
            ObjId(6),
            ObjId(7),
            ObjId(8),
            ObjId(9),
            ObjId(10),
            ObjId(11),
        );

        let supported_modifier_table = setup_linux_dmabuf(
            &mut ctx, &vulk, display, registry, dmabuf, comp, surface, feedback,
        );
        ctx.comp_write_passthrough(build_msgs(|dst| {
            write_evt_wl_registry_global(dst, registry, 3, WP_LINUX_DRM_SYNCOBJ_MANAGER_V1, 1);
        }));
        let msg = build_msgs(|dst| {
            write_req_wl_registry_bind(
                dst,
                registry,
                3,
                WP_LINUX_DRM_SYNCOBJ_MANAGER_V1,
                1,
                manager,
            );
            write_req_wp_linux_drm_syncobj_manager_v1_get_surface(dst, manager, sync_surf, surface);
            write_req_wp_linux_drm_syncobj_manager_v1_import_timeline(
                dst, manager, false, timeline,
            );
        });
        let start_pt = 150;
        let (prog_timeline, timeline_fd) = vulkan_create_timeline(&vulk, start_pt).unwrap();

        let (rmsg, mut rfd) = ctx.prog_write(&msg[..], &[&timeline_fd]).unwrap();
        assert!(rmsg.concat() == msg);
        drop(timeline_fd);
        assert!(rfd.len() == 1);
        let output_fd = rfd.remove(0);

        let comp_timeline = vulkan_import_timeline(&vulk, output_fd).unwrap();

        let (w, h) = (512, 512);
        let format = wayland_to_drm(WlShmFormat::R8);
        let file_sz = h * w;
        let mut base = vec![0x80; file_sz];

        let Some(modifier_list) = supported_modifier_table.get(&format) else {
            println!("Skipping test, format {:#08x} not supported", format);
            return;
        };
        let (img, mirror) = create_dmabuf_and_copy(
            &vulk,
            &mut ctx,
            params,
            dmabuf,
            buffer,
            w,
            h,
            format,
            modifier_list,
            &base,
        )
        .unwrap();

        ctx.prog_write_passthrough(build_msgs(|dst| {
            write_req_wl_surface_attach(dst, surface, buffer, 0, 0);
            write_req_wl_surface_damage_buffer(dst, surface, 0, 0, i32::MAX, i32::MAX);
            write_req_wl_surface_commit(dst, surface);
        }));

        let tmp_wr = Arc::new(vulkan_get_buffer(&vulk, img.nominal_size(None), false).unwrap());
        let tmp_rd = Arc::new(vulkan_get_buffer(&vulk, img.nominal_size(None), true).unwrap());

        for iter in 0..4 {
            /* Change entire image, except on the second iteration */
            if iter != 1 {
                base.fill(iter);
            }
            copy_onto_dmabuf(&img, &tmp_wr, &base).unwrap();
            let acq_pt: u64 = start_pt + 11 + (iter as u64) * 7;
            let rel_pt = acq_pt + 2;

            if iter == 2 {
                /* Special case: signal before writing */
                println!("Signalling acquire {}", acq_pt);
                prog_timeline.signal_timeline_pt(acq_pt).unwrap();
            }

            let msg = build_msgs(|dst| {
                write_req_wl_surface_damage_buffer(dst, surface, 0, 0, w as i32, h as i32);
                let (acq_hi, acq_lo) = split_u64(acq_pt);
                let (rel_hi, rel_lo) = split_u64(rel_pt);
                write_req_wp_linux_drm_syncobj_surface_v1_set_acquire_point(
                    dst, sync_surf, timeline, acq_hi, acq_lo,
                );
                write_req_wp_linux_drm_syncobj_surface_v1_set_release_point(
                    dst, sync_surf, timeline, rel_hi, rel_lo,
                );
                write_req_wl_surface_commit(dst, surface);
            });

            test_write_msgs(&ctx.sock_prog, &msg, &[]);

            /* Signal after writing; this is safe because the messages sent will at minimum
             * fit in the pipe buffer */
            if iter != 2 {
                println!("Signalling acquire {}", acq_pt);
                prog_timeline.signal_timeline_pt(acq_pt).unwrap();
            }

            /* Only start reading messages after signalling; this prevents a possible deadlock,
             * because the code might wait for the signal before sending messages further */
            let (rmsg, rfds, err) = test_read_msgs(&ctx.sock_comp, Some(&ctx.sock_prog));
            assert!(err.is_none());
            assert!(rfds.is_empty());
            assert!(rmsg.concat() == msg);

            let max_wait = 1000000000;
            println!("Waiting for acquire {}", acq_pt);
            comp_timeline
                .wait_for_timeline_pt(acq_pt, max_wait)
                .unwrap();

            let dup = copy_from_dmabuf(&mirror, &tmp_rd).unwrap();
            assert!(dup == base);

            println!("Signalling release {}", rel_pt);
            comp_timeline.signal_timeline_pt(rel_pt).unwrap();
            println!("Waiting for release {}", rel_pt);
            prog_timeline
                .wait_for_timeline_pt(rel_pt, max_wait)
                .unwrap();

            ctx.comp_write_passthrough(build_msgs(|dst| {
                write_evt_wl_buffer_release(dst, buffer);
            }));
        }

        ctx.prog_write_passthrough(build_msgs(|dst| {
            write_req_zwp_linux_buffer_params_v1_destroy(dst, params);
            write_req_wl_buffer_destroy(dst, buffer);
        }));
        ctx.comp_write_passthrough(build_msgs(|dst| {
            write_evt_wl_display_delete_id(dst, display, params.0);
            write_evt_wl_display_delete_id(dst, display, buffer.0);
        }));
    })?;

    Ok(StatusOk::Pass)
}

/** Test that Waypipe can successfully process a large number of FDS */
fn proto_many_fds(info: TestInfo) -> TestResult {
    let mut files: Vec<(Vec<u8>, OwnedFd)> = Vec::new();
    /* 100 is the > the 28 max fds sent in a batch by libwayland, but also
     * not that large that having four copies of each would break a standard
     * 1024-fd ulimit. */
    for i in 0..100 {
        let x: Vec<u8> = format!("{}", i).into();
        let fd = make_file_with_contents(&x).unwrap();
        files.push((x, fd));
    }

    run_protocol_test(&info, &|mut ctx: ProtocolTestContext| {
        /* Setup a wl_keyboard */
        let (display, registry, seat, keyboard) = (ObjId(1), ObjId(2), ObjId(3), ObjId(4));
        ctx.prog_write_passthrough(build_msgs(|dst| {
            write_req_wl_display_get_registry(dst, display, registry);
        }));
        ctx.comp_write_passthrough(build_msgs(|dst| {
            write_evt_wl_registry_global(dst, registry, 1, WL_SEAT, 7);
        }));
        ctx.prog_write_passthrough(build_msgs(|dst| {
            write_req_wl_registry_bind(dst, registry, 1, WL_SEAT, 7, seat);
        }));
        ctx.comp_write_passthrough(build_msgs(|dst| {
            write_evt_wl_seat_capabilities(dst, seat, 3);
        }));
        ctx.prog_write_passthrough(build_msgs(|dst| {
            write_req_wl_seat_get_keyboard(dst, seat, keyboard);
        }));

        /* Send all the files, in order */
        let fds: Vec<&OwnedFd> = files.iter().map(|(_c, f)| f).collect();

        let m = &build_msgs(|dst| {
            for (contents, _fd) in &files {
                write_evt_wl_keyboard_keymap(dst, keyboard, false, 1, contents.len() as u32);
            }
        });

        let (msgs, ofds) = ctx.comp_write(m, &fds).unwrap();

        assert!(msgs.concat() == *m);
        assert!(msgs.len() == ofds.len() && msgs.len() == files.len());
        for ((_msg, rfd), (contents, _fd)) in
            msgs.into_iter().zip(ofds.into_iter()).zip(files.iter())
        {
            let v = get_file_contents(&rfd, contents.len()).unwrap();
            assert!(v == *contents);
        }
    })?;
    Ok(StatusOk::Pass)
}

/** Test that Waypipe either cleanly processes or errors when applying a title prefix. */
fn proto_title_prefix(info: TestInfo) -> TestResult {
    // /* test lengths are: empty, short, too long for small buffers, cannot fit in a Wayland message */
    for (prefix_len, fail) in [(0, false), (100, false), (10000, true), (100000, true)] {
        let prefix = String::from("a").repeat(prefix_len);
        let options = WaypipeOptions {
            wire_version: None,
            drm_node: None,
            device_type: RenderDeviceType::Vulkan,
            video: VideoSetting::default(),
            title_prefix: &prefix,
            compression: Compression::None,
        };
        run_protocol_test_with_opts(
            &info,
            &options,
            &options,
            &|mut ctx: ProtocolTestContext| {
                let [display, reg, compositor, xdg_wm_base, wl_surf, xdg_surf, toplevel, ..] =
                    ID_SEQUENCE;

                ctx.prog_write_passthrough(build_msgs(|dst| {
                    write_req_wl_display_get_registry(dst, display, reg);
                }));
                ctx.comp_write_passthrough(build_msgs(|dst| {
                    write_evt_wl_registry_global(dst, reg, 1, WL_COMPOSITOR, 6);
                    write_evt_wl_registry_global(dst, reg, 2, XDG_WM_BASE, 6);
                }));
                ctx.prog_write_passthrough(build_msgs(|dst| {
                    write_req_wl_registry_bind(dst, reg, 1, WL_COMPOSITOR, 6, compositor);
                    write_req_wl_registry_bind(dst, reg, 2, XDG_WM_BASE, 6, xdg_wm_base);
                    write_req_wl_compositor_create_surface(dst, compositor, wl_surf);
                    write_req_xdg_wm_base_get_xdg_surface(dst, xdg_wm_base, xdg_surf, wl_surf);
                    write_req_xdg_surface_get_toplevel(dst, xdg_surf, toplevel);
                }));
                let title_lengths = [0, 200];
                let test = build_msgs(|dst| {
                    for title_len in title_lengths {
                        let title = vec![b'b'; title_len];
                        write_req_xdg_toplevel_set_title(dst, toplevel, &title);
                    }
                });

                match ctx.prog_write(&test, &[]) {
                    Err(_) => {
                        assert!(fail);
                    }
                    Ok((rmsgs, rfds)) => {
                        assert!(!fail);
                        assert!(rfds.is_empty());
                        for (title_len, msg) in title_lengths.iter().zip(rmsgs.iter()) {
                            assert!(
                                parse_wl_header(msg)
                                    == (toplevel, msg.len(), OPCODE_XDG_TOPLEVEL_SET_TITLE.code())
                            );
                            let title = parse_req_xdg_toplevel_set_title(msg).unwrap();
                            let orig_title = vec![b'b'; *title_len];
                            /* The title prefix is added twice, once per main loop instance */
                            let mut ref_title = vec![b'a'; prefix_len * 2];
                            ref_title.extend_from_slice(&orig_title);
                            assert!(title == ref_title);
                        }
                    }
                }
            },
        )?;
    }
    Ok(StatusOk::Pass)
}

/** Protocol to use for a screencopy test */
#[derive(Clone, Copy)]
enum ScreencopyType {
    WlrScreencopy,
    ExtImageCopyCapture,
}

/** Send event(s) signaling completion of a screencopy frame, and check they are correctly
 * replicated */
fn send_screencopy_ready(ctx: &mut ProtocolTestContext, frame: ObjId, style: ScreencopyType) {
    let time_ns: u32 = 123456789;
    let time_s: u32 = 1;
    let init_time_ns: u128 = (time_s as u128) * 1000000000 + (time_ns as u128);
    let ready_msg = build_msgs(|dst| match style {
        ScreencopyType::WlrScreencopy => {
            write_evt_zwlr_screencopy_frame_v1_ready(dst, frame, 0, time_s, time_ns)
        }
        ScreencopyType::ExtImageCopyCapture => {
            write_evt_ext_image_copy_capture_frame_v1_presentation_time(
                dst, frame, 0, time_s, time_ns,
            );
            write_evt_ext_image_copy_capture_frame_v1_ready(dst, frame)
        }
    });

    let start = Instant::now();
    let (rmsgs, rfds) = ctx.comp_write(&ready_msg, &[]).unwrap();
    assert!(rfds.is_empty());
    let end = Instant::now();

    let (tv_sec_hi, tv_sec_lo, tv_nsec) = match style {
        ScreencopyType::WlrScreencopy => {
            assert!(rmsgs.len() == 1);
            assert!(
                parse_wl_header(&rmsgs[0])
                    == (
                        frame,
                        length_evt_zwlr_screencopy_frame_v1_ready(),
                        OPCODE_ZWLR_SCREENCOPY_FRAME_V1_READY.code()
                    )
            );
            parse_evt_zwlr_screencopy_frame_v1_ready(&rmsgs[0]).unwrap()
        }
        ScreencopyType::ExtImageCopyCapture => {
            assert!(rmsgs.len() == 2);
            assert!(
                parse_wl_header(&rmsgs[0])
                    == (
                        frame,
                        length_evt_ext_image_copy_capture_frame_v1_presentation_time(),
                        OPCODE_EXT_IMAGE_COPY_CAPTURE_FRAME_V1_PRESENTATION_TIME.code()
                    )
            );
            assert!(
                parse_wl_header(&rmsgs[1])
                    == (
                        frame,
                        length_evt_ext_image_copy_capture_frame_v1_ready(),
                        OPCODE_EXT_IMAGE_COPY_CAPTURE_FRAME_V1_READY.code()
                    )
            );
            parse_evt_ext_image_copy_capture_frame_v1_presentation_time(&rmsgs[0]).unwrap()
        }
    };
    let output_ns = 1000000000 * (join_u64(tv_sec_hi, tv_sec_lo) as u128) + (tv_nsec as u128);

    /* The time adjustment uses two XYX measurements, whose absolute error
     * is ≤ half the elapsed time each, assuming the clocks run at the same
     * rate and do not change. */
    let max_time_error = end.duration_since(start).saturating_mul(2);

    let abs_diff = output_ns.abs_diff(init_time_ns);
    assert!(abs_diff < max_time_error.as_nanos());
}

/** Test that basic screencopy operations work with wl_shm buffers */
fn proto_screencopy_shm(info: TestInfo, style: ScreencopyType) -> TestResult {
    run_protocol_test(&info, &|mut ctx: ProtocolTestContext| {
        let [display, reg, shm, screencopy, output, shm_pool, buffer1, buffer2, ..] = ID_SEQUENCE;
        let (capture_manager, capture_source, session, frame) = match style {
            ScreencopyType::WlrScreencopy => (ObjId(0), ObjId(0), ObjId(0), ObjId(9)),
            ScreencopyType::ExtImageCopyCapture => (ObjId(9), ObjId(10), ObjId(11), ObjId(12)),
        };
        ctx.prog_write_passthrough(build_msgs(|dst| {
            write_req_wl_display_get_registry(dst, display, reg);
        }));
        let scrcopy_name = ZWLR_SCREENCOPY_MANAGER_V1;
        ctx.comp_write_passthrough(build_msgs(|dst| {
            write_evt_wl_registry_global(dst, reg, 1, WL_SHM, 2);
            write_evt_wl_registry_global(dst, reg, 2, WL_OUTPUT, 4);
            match style {
                ScreencopyType::WlrScreencopy => {
                    write_evt_wl_registry_global(dst, reg, 3, scrcopy_name, 3);
                }
                ScreencopyType::ExtImageCopyCapture => {
                    write_evt_wl_registry_global(dst, reg, 3, EXT_IMAGE_COPY_CAPTURE_MANAGER_V1, 1);
                    write_evt_wl_registry_global(
                        dst,
                        reg,
                        4,
                        EXT_OUTPUT_IMAGE_CAPTURE_SOURCE_MANAGER_V1,
                        1,
                    );
                }
            }
        }));
        let (w, h, fmt) = (33, 17, WlShmFormat::Xrgb8888 as u32);
        let pool_sz: usize = 3 * (w * h * 4) / 2;
        let offset1: usize = 10;
        let offset2: usize = pool_sz - (w * h * 4);
        let seed_contents: Vec<u8> = (0..pool_sz).map(|x| (x * 101) as u8).collect();
        let mut copy_contents: Vec<u8> = seed_contents.clone();
        assert!(offset1 <= 333 && 666 <= offset1 + w * h * 4);
        copy_contents[333..666].fill(0xaa);
        let shm_fd = make_file_with_contents(&seed_contents).unwrap();

        let setup = build_msgs(|dst| {
            write_req_wl_registry_bind(dst, reg, 1, WL_SHM, 2, shm);
            write_req_wl_registry_bind(dst, reg, 2, WL_OUTPUT, 4, output);
            match style {
                ScreencopyType::WlrScreencopy => {
                    write_req_wl_registry_bind(dst, reg, 3, scrcopy_name, 3, screencopy)
                }
                ScreencopyType::ExtImageCopyCapture => write_req_wl_registry_bind(
                    dst,
                    reg,
                    3,
                    EXT_IMAGE_COPY_CAPTURE_MANAGER_V1,
                    1,
                    screencopy,
                ),
            }
            write_req_wl_shm_create_pool(dst, shm, false, shm_pool, pool_sz.try_into().unwrap());
            write_req_wl_shm_pool_create_buffer(
                dst,
                shm_pool,
                buffer1,
                offset1.try_into().unwrap(),
                w.try_into().unwrap(),
                h.try_into().unwrap(),
                (w * 4).try_into().unwrap(),
                fmt,
            );
            write_req_wl_shm_pool_create_buffer(
                dst,
                shm_pool,
                buffer2,
                offset2.try_into().unwrap(),
                w.try_into().unwrap(),
                h.try_into().unwrap(),
                (w * 4).try_into().unwrap(),
                fmt,
            );

            match style {
                ScreencopyType::WlrScreencopy => {
                    write_req_zwlr_screencopy_manager_v1_capture_output_region(
                        dst,
                        screencopy,
                        frame,
                        0,
                        output,
                        1,
                        1,
                        (w - 2) as _,
                        (h - 2) as _,
                    );
                }
                ScreencopyType::ExtImageCopyCapture => {
                    write_req_wl_registry_bind(
                        dst,
                        reg,
                        4,
                        EXT_OUTPUT_IMAGE_CAPTURE_SOURCE_MANAGER_V1,
                        1,
                        capture_manager,
                    );
                    write_req_ext_output_image_capture_source_manager_v1_create_source(
                        dst,
                        capture_manager,
                        capture_source,
                        output,
                    );
                    write_req_ext_image_copy_capture_manager_v1_create_session(
                        dst,
                        screencopy,
                        session,
                        capture_source,
                        0,
                    );
                }
            }
        });
        let fds = [&shm_fd];
        let (rmsgs, mut rfds) = ctx.prog_write(&setup, &fds).unwrap();
        assert!(rmsgs.concat() == setup);
        assert!(rfds.len() == 1);
        let rfd = rfds.pop().unwrap();
        update_file_contents(&rfd, &copy_contents).unwrap();

        ctx.comp_write_passthrough(build_msgs(|dst| match style {
            ScreencopyType::WlrScreencopy => {
                write_evt_zwlr_screencopy_frame_v1_buffer(
                    dst,
                    frame,
                    fmt,
                    w as _,
                    h as _,
                    (4 * w) as _,
                );
                write_evt_zwlr_screencopy_frame_v1_buffer_done(dst, frame);
            }
            ScreencopyType::ExtImageCopyCapture => {
                write_evt_ext_image_copy_capture_session_v1_buffer_size(
                    dst, session, w as _, h as _,
                );
                write_evt_ext_image_copy_capture_session_v1_shm_format(dst, session, fmt);
                write_evt_ext_image_copy_capture_session_v1_done(dst, session);
            }
        }));
        ctx.prog_write_passthrough(build_msgs(|dst| match style {
            ScreencopyType::WlrScreencopy => {
                write_req_zwlr_screencopy_frame_v1_copy(dst, frame, buffer2);
            }
            ScreencopyType::ExtImageCopyCapture => {
                write_req_ext_image_copy_capture_session_v1_create_frame(dst, session, frame);
                write_req_ext_image_copy_capture_frame_v1_attach_buffer(dst, frame, buffer2);
                write_req_ext_image_copy_capture_frame_v1_damage_buffer(
                    dst,
                    frame,
                    0,
                    0,
                    i32::MAX,
                    i32::MAX,
                );
                write_req_ext_image_copy_capture_frame_v1_capture(dst, frame);
            }
        }));
        ctx.comp_write_passthrough(build_msgs(|dst| match style {
            ScreencopyType::WlrScreencopy => write_evt_zwlr_screencopy_frame_v1_failed(dst, frame),
            ScreencopyType::ExtImageCopyCapture => {
                write_evt_ext_image_copy_capture_frame_v1_failed(dst, frame, 0)
            }
        }));
        /* Check that the failed screencopy does not lead to replication
         * (Although technically Waypipe _could_ eagerly make updates in this scenario,
         * since the buffer contents were already updated, doing so would be inefficient;
         * the wlr-screencopy protocol also says nothing about the buffer state on failure.)
         */
        let check1 = get_file_contents(&shm_fd, pool_sz).unwrap();
        assert!(check1 == seed_contents);

        // Check diff
        ctx.prog_write_passthrough(build_msgs(|dst| match style {
            ScreencopyType::WlrScreencopy => write_req_zwlr_screencopy_frame_v1_destroy(dst, frame),
            ScreencopyType::ExtImageCopyCapture => {
                write_req_ext_image_copy_capture_session_v1_destroy(dst, session);
                write_req_ext_image_copy_capture_frame_v1_destroy(dst, frame)
            }
        }));
        ctx.comp_write_passthrough(build_msgs(|dst| match style {
            ScreencopyType::WlrScreencopy => write_evt_wl_display_delete_id(dst, display, frame.0),
            ScreencopyType::ExtImageCopyCapture => {
                write_evt_wl_display_delete_id(dst, display, session.0);
                write_evt_wl_display_delete_id(dst, display, frame.0);
            }
        }));
        ctx.prog_write_passthrough(build_msgs(|dst| match style {
            ScreencopyType::WlrScreencopy => {
                write_req_zwlr_screencopy_manager_v1_capture_output(
                    dst, screencopy, frame, 0, output,
                );
            }
            ScreencopyType::ExtImageCopyCapture => {
                write_req_ext_image_copy_capture_manager_v1_create_session(
                    dst,
                    screencopy,
                    session,
                    capture_source,
                    0,
                );
            }
        }));
        ctx.comp_write_passthrough(build_msgs(|dst| match style {
            ScreencopyType::WlrScreencopy => {
                write_evt_zwlr_screencopy_frame_v1_buffer(
                    dst,
                    frame,
                    fmt,
                    w as _,
                    h as _,
                    (4 * w) as _,
                );
                write_evt_zwlr_screencopy_frame_v1_buffer_done(dst, frame);
            }
            ScreencopyType::ExtImageCopyCapture => {
                write_evt_ext_image_copy_capture_session_v1_buffer_size(
                    dst, session, w as _, h as _,
                );
                write_evt_ext_image_copy_capture_session_v1_shm_format(dst, session, fmt);
                write_evt_ext_image_copy_capture_session_v1_done(dst, session);
            }
        }));
        ctx.prog_write_passthrough(build_msgs(|dst| match style {
            ScreencopyType::WlrScreencopy => {
                write_req_zwlr_screencopy_frame_v1_copy(dst, frame, buffer1);
            }
            ScreencopyType::ExtImageCopyCapture => {
                write_req_ext_image_copy_capture_session_v1_create_frame(dst, session, frame);
                write_req_ext_image_copy_capture_frame_v1_attach_buffer(dst, frame, buffer1);
                write_req_ext_image_copy_capture_frame_v1_damage_buffer(
                    dst,
                    frame,
                    0,
                    0,
                    i32::MAX,
                    i32::MAX,
                );
                write_req_ext_image_copy_capture_frame_v1_capture(dst, frame);
            }
        }));

        send_screencopy_ready(&mut ctx, frame, style);

        /* Check that the update is replicated */
        let check2 = get_file_contents(&shm_fd, pool_sz).unwrap();
        assert!(check2 == copy_contents);
    })?;
    Ok(StatusOk::Pass)
}

/** Test that basic wlr-screencopy operations work with wl_shm buffers */
fn proto_screencopy_shm_wlr(info: TestInfo) -> TestResult {
    proto_screencopy_shm(info, ScreencopyType::WlrScreencopy)
}
/** Test that basic ext-image-copy-capture operations work with wl_shm buffers */
fn proto_screencopy_shm_ext(info: TestInfo) -> TestResult {
    proto_screencopy_shm(info, ScreencopyType::ExtImageCopyCapture)
}

/** Test that basic wlr_screencopy operations work with dmabufs */
#[cfg(feature = "dmabuf")]
fn proto_screencopy_dmabuf(
    info: TestInfo,
    device: RenderDevice,
    style: ScreencopyType,
) -> TestResult {
    let Ok(vulk) = setup_vulkan(device.id) else {
        return Ok(StatusOk::Skipped);
    };

    run_protocol_test_with_drm_node(&info, &device, &|mut ctx: ProtocolTestContext| {
        let [display, reg, dmabuf, feedback, output, screencopy, ..] = ID_SEQUENCE;

        let (capture_manager, capture_source, session, frame, params, buffer) = match style {
            ScreencopyType::WlrScreencopy => {
                (ObjId(0), ObjId(0), ObjId(0), ObjId(9), ObjId(10), ObjId(11))
            }
            ScreencopyType::ExtImageCopyCapture => (
                ObjId(9),
                ObjId(10),
                ObjId(11),
                ObjId(12),
                ObjId(13),
                ObjId(14),
            ),
        };

        ctx.prog_write_passthrough(build_msgs(|dst| {
            write_req_wl_display_get_registry(dst, display, reg);
        }));
        let scrcopy_name = ZWLR_SCREENCOPY_MANAGER_V1;
        ctx.comp_write_passthrough(build_msgs(|dst| {
            write_evt_wl_registry_global(dst, reg, 1, ZWP_LINUX_DMABUF_V1, 3);
            write_evt_wl_registry_global(dst, reg, 2, WL_OUTPUT, 4);
            match style {
                ScreencopyType::WlrScreencopy => {
                    write_evt_wl_registry_global(dst, reg, 3, scrcopy_name, 3)
                }
                ScreencopyType::ExtImageCopyCapture => {
                    write_evt_wl_registry_global(dst, reg, 3, EXT_IMAGE_COPY_CAPTURE_MANAGER_V1, 1);
                    write_evt_wl_registry_global(
                        dst,
                        reg,
                        4,
                        EXT_OUTPUT_IMAGE_CAPTURE_SOURCE_MANAGER_V1,
                        1,
                    );
                }
            }
        }));
        ctx.prog_write_passthrough(build_msgs(|dst| {
            write_req_wl_registry_bind(dst, reg, 1, ZWP_LINUX_DMABUF_V1, 3, dmabuf);
            write_req_zwp_linux_dmabuf_v1_get_default_feedback(dst, dmabuf, feedback);
            write_req_wl_registry_bind(dst, reg, 2, WL_OUTPUT, 4, output);
            match style {
                ScreencopyType::WlrScreencopy => {
                    write_req_wl_registry_bind(dst, reg, 3, scrcopy_name, 3, screencopy);
                    write_req_zwlr_screencopy_manager_v1_capture_output(
                        dst, screencopy, frame, 0, output,
                    );
                }
                ScreencopyType::ExtImageCopyCapture => {
                    write_req_wl_registry_bind(
                        dst,
                        reg,
                        3,
                        EXT_IMAGE_COPY_CAPTURE_MANAGER_V1,
                        1,
                        screencopy,
                    );
                    write_req_wl_registry_bind(
                        dst,
                        reg,
                        4,
                        EXT_OUTPUT_IMAGE_CAPTURE_SOURCE_MANAGER_V1,
                        1,
                        capture_manager,
                    );
                    write_req_ext_output_image_capture_source_manager_v1_create_source(
                        dst,
                        capture_manager,
                        capture_source,
                        output,
                    );
                    write_req_ext_image_copy_capture_manager_v1_create_session(
                        dst,
                        screencopy,
                        session,
                        capture_source,
                        0,
                    );
                }
            }
        }));

        let supported_modifier_table = send_linux_dmabuf_feedback(&mut ctx, &vulk, feedback);

        let fmt = wayland_to_drm(WlShmFormat::R8);
        let bpp = 1;
        let (w, h) = (8, 8);
        let Some(mod_list) = supported_modifier_table.get(&fmt) else {
            println!("Skipping test, format not supported");
            return;
        };

        let msg_batch = build_msgs(|dst| {
            write_evt_ext_image_copy_capture_session_v1_buffer_size(dst, session, w as _, h as _);
            write_evt_ext_image_copy_capture_session_v1_dmabuf_device(
                dst,
                session,
                &u64::to_le_bytes(device.id),
            );
            let mut mod_array = Vec::new();
            for m in mod_list {
                mod_array.extend_from_slice(&u64::to_le_bytes(*m));
            }
            write_evt_ext_image_copy_capture_session_v1_dmabuf_format(
                dst, session, fmt, &mod_array,
            );
            write_evt_ext_image_copy_capture_session_v1_done(dst, session);
        });
        let (rmsgs, rfds) = ctx.comp_write(&msg_batch, &[]).unwrap();
        assert!(rfds.is_empty());
        let nmsgs = rmsgs.len();
        let mut received_mod_table: Vec<(u32, Vec<u64>)> = Vec::new();
        let (mut has_size, mut has_device) = (false, false);
        for (i, msg) in rmsgs.into_iter().enumerate() {
            let (obj, _len, opcode) = parse_wl_header(&msg);
            assert!(obj == session);
            match MethodId::Event(opcode) {
                OPCODE_EXT_IMAGE_COPY_CAPTURE_SESSION_V1_BUFFER_SIZE => {
                    assert!(
                        parse_evt_ext_image_copy_capture_session_v1_buffer_size(&msg).unwrap()
                            == (w as _, h as _)
                    );
                    has_size = true;
                }
                OPCODE_EXT_IMAGE_COPY_CAPTURE_SESSION_V1_DMABUF_DEVICE => {
                    assert!(
                        parse_evt_ext_image_copy_capture_session_v1_dmabuf_device(&msg).unwrap()
                            == u64::to_le_bytes(device.id)
                    );
                    has_device = true;
                }
                OPCODE_EXT_IMAGE_COPY_CAPTURE_SESSION_V1_DMABUF_FORMAT => {
                    let (fmt, mods) =
                        parse_evt_ext_image_copy_capture_session_v1_dmabuf_format(&msg).unwrap();
                    assert!(mods.len() % 8 == 0);
                    received_mod_table.push((
                        fmt,
                        mods.chunks_exact(8)
                            .map(|x| u64::from_le_bytes(x.try_into().unwrap()))
                            .collect::<Vec<u64>>(),
                    ));
                }
                OPCODE_EXT_IMAGE_COPY_CAPTURE_SESSION_V1_DONE => {
                    assert!(i == nmsgs - 1);
                }
                _ => panic!("Unexpected message opcode {}", opcode),
            }
        }
        assert!(has_size);
        assert!(has_device);
        assert!(received_mod_table.len() == 1);
        assert!(received_mod_table[0].0 == fmt);
        received_mod_table[0].1.sort();
        let mut mod_copy = mod_list.clone();
        mod_copy.sort();
        /* All modifiers should make the roundtrip, since they were already filtered by linux-dmabuf */
        assert!(received_mod_table[0].1 == mod_copy);

        let img_size = (w * h) as usize * bpp;
        let img_data = vec![0x33u8; img_size];
        let mod_data = vec![0x44u8; img_size];

        let copy_buf = Arc::new(vulkan_get_buffer(&vulk, img_size, true).unwrap());

        let (prog_img, comp_img) = create_dmabuf_and_copy(
            &vulk, &mut ctx, params, dmabuf, buffer, w as _, h as _, fmt, mod_list, &img_data,
        )
        .unwrap();

        ctx.prog_write_passthrough(build_msgs(|dst| match style {
            ScreencopyType::WlrScreencopy => {
                write_req_zwlr_screencopy_frame_v1_copy(dst, frame, buffer);
            }
            ScreencopyType::ExtImageCopyCapture => {
                write_req_ext_image_copy_capture_session_v1_create_frame(dst, session, frame);
                write_req_ext_image_copy_capture_frame_v1_attach_buffer(dst, frame, buffer);
                write_req_ext_image_copy_capture_frame_v1_damage_buffer(
                    dst,
                    frame,
                    0,
                    0,
                    i32::MAX,
                    i32::MAX,
                );
                write_req_ext_image_copy_capture_frame_v1_capture(dst, frame);
            }
        }));

        copy_onto_dmabuf(&comp_img, &copy_buf, &mod_data).unwrap();

        send_screencopy_ready(&mut ctx, frame, style);

        let output = copy_from_dmabuf(&prog_img, &copy_buf).unwrap();
        assert!(output == mod_data);

        ctx.prog_write_passthrough(build_msgs(|dst| match style {
            ScreencopyType::WlrScreencopy => {
                write_req_zwlr_screencopy_frame_v1_destroy(dst, frame);
            }
            ScreencopyType::ExtImageCopyCapture => {
                write_req_ext_image_copy_capture_frame_v1_destroy(dst, frame);
                write_req_ext_image_copy_capture_session_v1_destroy(dst, session);
            }
        }));
    })?;
    Ok(StatusOk::Pass)
}

/** Test that basic wlr-screencopy operations work with dmabufs */
#[cfg(feature = "dmabuf")]
fn proto_screencopy_dmabuf_wlr(info: TestInfo, device: RenderDevice) -> TestResult {
    proto_screencopy_dmabuf(info, device, ScreencopyType::WlrScreencopy)
}
/** Test that basic ext-image-copy-capture operations work with dmabufs */
#[cfg(feature = "dmabuf")]
fn proto_screencopy_dmabuf_ext(info: TestInfo, device: RenderDevice) -> TestResult {
    proto_screencopy_dmabuf(info, device, ScreencopyType::ExtImageCopyCapture)
}

/** Register an array of video tests for various video format and hardware/software
 * encoding/decoding parameters */
#[cfg(feature = "video")]
fn register_video_tests<'a>(
    tests: &mut Vec<(String, Box<dyn Fn(TestInfo) -> TestResult + 'a>)>,
    filter: &Filter,
    devices: &[(String, u64)],
) {
    let table = [
        (VideoFormat::H264, false, false, true),
        (VideoFormat::H264, true, false, false),
        (VideoFormat::H264, true, true, false),
        (VideoFormat::H264, false, true, false),
        (VideoFormat::VP9, false, false, true),
        (VideoFormat::AV1, false, false, true),
        (VideoFormat::AV1, true, false, true),
        (VideoFormat::AV1, true, true, true),
        (VideoFormat::AV1, false, true, true),
    ];

    for (format, try_hwenc, try_hwdec, expect_accurate_on_small) in table {
        for (dev_name, dev_id) in devices {
            let name = format!(
                "proto::dmavid_{}::{}::{}::{}",
                match format {
                    VideoFormat::H264 => "h264",
                    VideoFormat::VP9 => "vp9",
                    VideoFormat::AV1 => "av1",
                },
                if try_hwenc { "try_hwenc" } else { "swenc" },
                if try_hwdec { "try_hwdec" } else { "swdec" },
                dev_name,
            );
            if !test_is_included(&name, filter) {
                continue;
            }
            let m: (String, u64) = (dev_name.clone(), *dev_id);
            tests.push((
                name,
                Box::new(move |info| {
                    let s = &m;
                    let dev = RenderDevice {
                        name: &s.0,
                        id: s.1,
                        /* Video encoding/decoding only works with Vulkan right now */
                        device_type: RenderDeviceType::Vulkan,
                    };
                    proto_dmavid(
                        info,
                        dev,
                        format,
                        try_hwdec,
                        try_hwenc,
                        expect_accurate_on_small,
                    )
                }),
            ));
        }
    }
}

/** Test that Waypipe correctly updates buffers when xdg_toplevel_icon_v1::add_buffer is used */
fn proto_toplevel_icon(info: TestInfo) -> TestResult {
    run_protocol_test(&info, &|mut ctx: ProtocolTestContext| {
        let [display, registry, shm, ico_mgr, pool, icon, buffer1, buffer2, buffer3, ..] =
            ID_SEQUENCE;

        ctx.prog_write_passthrough(build_msgs(|dst| {
            write_req_wl_display_get_registry(dst, display, registry);
        }));

        ctx.comp_write_passthrough(build_msgs(|dst| {
            write_evt_wl_registry_global(dst, registry, 1, WL_SHM, 2);
            write_evt_wl_registry_global(dst, registry, 2, XDG_TOPLEVEL_ICON_MANAGER_V1, 1);
        }));

        let size = 1300;
        let mut data = vec![0x01; size];
        let offsets: [u32; 3] = [0, 99, 600];
        let sizes: [u32; 3] = [1, 10, 13];
        let format = WlShmFormat::Argb8888;
        let buffers = [buffer1, buffer2, buffer3];
        let msgs = build_msgs(|dst| {
            write_req_wl_registry_bind(dst, registry, 1, WL_SHM, 2, shm);
            write_req_wl_registry_bind(dst, registry, 2, XDG_TOPLEVEL_ICON_MANAGER_V1, 1, ico_mgr);

            write_req_wl_shm_create_pool(dst, shm, false, pool, size as i32);
            write_req_xdg_toplevel_icon_manager_v1_create_icon(dst, ico_mgr, icon);

            for ((size, offset), buffer) in sizes.iter().zip(offsets.iter()).zip(buffers.iter()) {
                write_req_wl_shm_pool_create_buffer(
                    dst,
                    pool,
                    *buffer,
                    *offset as i32,
                    *size as i32,
                    *size as i32,
                    *size as i32 * 4,
                    format as u32,
                );
            }
        });
        let pool_fd = make_file_with_contents(&data).unwrap();

        let (rmsg, mut rfd) = ctx.prog_write(&msgs[..], &[&pool_fd]).unwrap();
        assert!(rmsg.concat() == msgs);
        assert!(rfd.len() == 1);
        let output_fd = rfd.remove(0);

        ctx.comp_write_passthrough(build_msgs(|dst| {
            for size in sizes {
                write_evt_xdg_toplevel_icon_manager_v1_icon_size(dst, ico_mgr, size as i32);
            }
            write_evt_xdg_toplevel_icon_manager_v1_done(dst, ico_mgr);
        }));

        // Fill icon buffer contents
        for (size, offset) in sizes.iter().zip(offsets.iter()) {
            for (z, c) in data[(*offset as usize)..(*offset + size * size * 4) as usize]
                .chunks_exact_mut(4)
                .enumerate()
            {
                c.copy_from_slice(&(z as u32 + 10).to_le_bytes());
            }
        }
        update_file_contents(&pool_fd, &data).unwrap();

        ctx.prog_write_passthrough(build_msgs(|dst| {
            for buffer in buffers {
                write_req_xdg_toplevel_icon_v1_add_buffer(dst, icon, buffer, 1);
            }
        }));

        // Verify buffers were replicated
        let rcontents = get_file_contents(&output_fd, data.len()).unwrap();

        for (size, offset) in sizes.iter().zip(offsets.iter()) {
            let range = (*offset as usize)..(*offset + size * size * 4) as usize;
            assert!(data[range.clone()] == rcontents[range.clone()]);
        }
    })?;
    Ok(StatusOk::Pass)
}

/** Main entry point. */
fn main() -> ExitCode {
    let command = ClapCommand::new(env!("CARGO_BIN_NAME"))
        .help_expected(true)
        .flatten_help(false)
        .about(
            "A collection of protocol tests to be run against the Waypipe client\n\
        and server connection subprocesses. This is not expected to be packaged\n\
        and has no stability guarantee.",
        )
        .next_line_help(false)
        .version(env!("CARGO_PKG_VERSION"))
        .arg(
            Arg::new("client-path")
                .help("waypipe binary to use as client")
                .required(true)
                .value_parser(value_parser!(OsString)),
        )
        .arg(
            Arg::new("server-path")
                .help("waypipe binary to use as server")
                .required(true)
                .value_parser(value_parser!(OsString)),
        )
        .arg(
            Arg::new("test-filter")
                .help("If present, run tests whose names contain any test filter string")
                .num_args(0..)
                .action(ArgAction::Append)
                .value_parser(value_parser!(String)),
        )
        .arg(
            Arg::new("list")
                .long("list")
                .action(ArgAction::SetTrue)
                .conflicts_with_all(["client-path", "server-path", "test-filter"])
                .help("List available tests"),
        )
        .arg(
            Arg::new("exact")
                .long("exact")
                .action(ArgAction::SetTrue)
                .help("When matching test names, treat parts between :: as indivisible tokens"),
        )
        .arg(
            Arg::new("core")
                .long("core")
                .action(ArgAction::SetTrue)
                .help("Allow subprocesses to create core dumps on crash"),
        )
        .arg(
            Arg::new("quiet")
                .long("quiet")
                .short('q')
                .action(ArgAction::SetTrue)
                .help("Hide captured output"),
        );

    let matches = command.get_matches();

    let substrings: Vec<&str> = matches
        .get_many::<String>("test-filter")
        .map(|x| x.map(|y| y.as_str()).collect())
        .unwrap_or_default();
    let f = Filter {
        substrings: &substrings,
        exact: matches.get_flag("exact"),
    };
    let core = matches.get_flag("core");

    let vk_device_ids = list_vulkan_device_ids();

    /* Construct a list of all tests */
    let mut tests: Vec<(String, Box<dyn Fn(TestInfo) -> TestResult>)> = Vec::new();
    let t = &mut tests;

    register_single(t, &f, "basic", proto_basic);
    register_single(t, &f, "base_wire", proto_base_wire);
    register_single(t, &f, "commit_timing", proto_commit_timing);
    register_single(t, &f, "damage_efficiency", proto_damage_efficiency);
    register_single(t, &f, "fixes", proto_fixes);
    register_single(t, &f, "flip_damage", proto_flip_damage);
    register_single(t, &f, "gamma_control", proto_gamma_control);
    register_single(t, &f, "keymap", proto_keymap);
    register_single(t, &f, "icc", proto_icc);
    register_single(t, &f, "many_fds", proto_many_fds);
    register_single(t, &f, "object_collision", proto_object_collision);
    register_single(t, &f, "oversized", proto_oversized);
    register_single(t, &f, "pipe_write", proto_pipe_write);
    register_single(t, &f, "presentation_time", proto_presentation_time);
    register_single(t, &f, "rotating_damage", proto_rotating_damage);
    register_single(t, &f, "screencopy_shm_wlr", proto_screencopy_shm_wlr);
    register_single(t, &f, "screencopy_shm_ext", proto_screencopy_shm_ext);
    register_single(t, &f, "shm_buffer", proto_shm_buffer);
    register_single(t, &f, "shm_damage", proto_shm_damage);
    register_single(t, &f, "shm_extend", proto_shm_extend);
    register_single(t, &f, "title_prefix", proto_title_prefix);
    register_single(t, &f, "toplevel_icon", proto_toplevel_icon);
    register_single(t, &f, "viewporter_damage", proto_viewporter_damage);

    #[cfg(feature = "dmabuf")]
    {
        register_per_device(t, &f, &vk_device_ids, "dmabuf", proto_dmabuf);
        register_per_device(t, &f, &vk_device_ids, "dmabuf_damage", proto_dmabuf_damage);
        register_per_device(
            t,
            &f,
            &vk_device_ids,
            "dmabuf_feedback_table",
            proto_dmabuf_feedback_table,
        );
        register_per_device(t, &f, &vk_device_ids, "dmabuf_pre_v4", proto_dmabuf_pre_v4);
        register_per_device(t, &f, &vk_device_ids, "explicit_sync", proto_explicit_sync);
        register_per_device(
            t,
            &f,
            &vk_device_ids,
            "screencopy_dmabuf_ext",
            proto_screencopy_dmabuf_ext,
        );
        register_per_device(
            t,
            &f,
            &vk_device_ids,
            "screencopy_dmabuf_wlr",
            proto_screencopy_dmabuf_wlr,
        );
    }

    #[cfg(feature = "video")]
    {
        register_video_tests(t, &f, &vk_device_ids);
    }

    if matches.get_flag("list") {
        for (name, _) in tests {
            println!("{}", name);
        }
        return ExitCode::SUCCESS;
    }

    let client_file: &OsString = matches.get_one("client-path").unwrap();
    let server_file: &OsString = matches.get_one("server-path").unwrap();
    let quiet: bool = matches.get_flag("quiet");

    let mut mask = signal::SigSet::empty();
    mask.add(signal::SIGCHLD);
    let mut pollmask = mask
        .thread_swap_mask(signal::SigmaskHow::SIG_BLOCK)
        .map_err(|x| tag!("Failed to set sigmask: {}", x))
        .unwrap();
    pollmask.remove(signal::SIGCHLD);

    let sigaction = signal::SigAction::new(
        signal::SigHandler::Handler(noop_signal_handler),
        signal::SaFlags::SA_NOCLDSTOP,
        signal::SigSet::empty(),
    );
    unsafe {
        // SAFETY: signal handler installed is trivial and replaces nothing
        signal::sigaction(signal::Signal::SIGCHLD, &sigaction)
            .map_err(|x| tag!("Failed to set sigaction: {}", x))
            .unwrap();
    }

    let mut nfail: usize = 0;
    let mut nskip: usize = 0;
    for (name, func) in &tests {
        let mut stdout = std::io::stdout().lock();
        write!(&mut stdout, "{} ...", name).unwrap();
        /* flush immediately so that buffer is empty before fork and will not be written twice */
        stdout.flush().unwrap();
        drop(stdout);

        let info = TestInfo {
            test_name: name,
            waypipe_client: client_file,
            waypipe_server: server_file,
        };

        let (read_out, new_stdouterr) = create_pipe(fcntl::OFlag::empty()).unwrap();
        let new_stdin = fcntl::open(
            "/dev/null",
            fcntl::OFlag::O_RDONLY | fcntl::OFlag::O_NOCTTY,
            nix::sys::stat::Mode::empty(),
        )
        .unwrap();

        let res = match unsafe {
            /* SAFETY: this program is not multi-threaded at this point.
             * (No vulkan setup or initialization has been done.) */
            unistd::fork().unwrap()
        } {
            unistd::ForkResult::Child => {
                /* Blocking wait for child to complete */
                /* Atomically replace STDOUT, STDERR, STDIN; this may break library code which
                 * incorrectly assumes standard io file descriptors never change properties
                 * (e.g., by caching isatty()). */
                unistd::dup2_stdout(&new_stdouterr).unwrap();
                unistd::dup2_stderr(&new_stdouterr).unwrap();
                unistd::dup2_stdin(&new_stdin).unwrap();

                drop(read_out);

                if !core {
                    /* Disable core dumps for child process, because the tests report errors
                     * using using panic! or a failed assert!, and core dumps should not need
                     * recording. This _also_ prevents the Waypipe instances from making a
                     * core dump. */
                    assert!(
                        unsafe {
                            let x = libc::rlimit {
                                rlim_cur: 0,
                                rlim_max: 0,
                            };
                            /* SAFETY: x is properly aligned, is not captured by setrlimit, and lives until end of scope */
                            libc::setrlimit(libc::RLIMIT_CORE, &x)
                        } != -1
                    );
                }

                // TODO: process group configuration to kill the child process and
                // everything it spawns on timeout events?

                let ret = func(info);
                return match ret {
                    Ok(StatusOk::Pass) => ExitCode::SUCCESS,
                    Ok(StatusOk::Skipped) => ExitCode::from(EXITCODE_SKIPPED),
                    Err(StatusBad::Fail(msg)) => {
                        println!("Test {} failed with error: {}", name, msg);
                        ExitCode::FAILURE
                    }
                    Err(StatusBad::Unclear(msg)) => {
                        println!("Test {} unclear, with error: {}", name, msg);
                        ExitCode::from(EXITCODE_UNCLEAR)
                    }
                };
            }
            unistd::ForkResult::Parent { child } => {
                drop(new_stdin);
                drop(new_stdouterr);

                set_nonblock(&read_out).unwrap();

                /* Wait for the child to complete, and capture its output */
                let mut log = Vec::new();
                let mut tmp = vec![0u8; 1 << 18];
                let child_status: WaitStatus;
                loop {
                    let status = wait::waitpid(child, Some(wait::WaitPidFlag::WNOHANG)).unwrap();
                    match status {
                        wait::WaitStatus::Exited(..) | wait::WaitStatus::Signaled(..) => {
                            child_status = status;
                            break;
                        }
                        _ => (),
                    }

                    let mut pfds = [poll::PollFd::new(read_out.as_fd(), poll::PollFlags::POLLIN)];
                    let res = system_poll(&mut pfds, None, Some(pollmask));
                    if let Err(errno) = res {
                        assert!(errno == Errno::EINTR || errno == Errno::EAGAIN);
                        continue;
                    }

                    let rev = pfds[0].revents().unwrap();
                    if rev.contains(poll::PollFlags::POLLERR)
                        || rev.contains(poll::PollFlags::POLLHUP)
                    {
                        /* Child closed connection (or more likely, died).
                         * Data remaining in pipe will be read. */
                        child_status = wait::waitpid(child, None).unwrap();
                        break;
                    }

                    if !rev.contains(poll::PollFlags::POLLIN) {
                        continue;
                    }

                    let eof_or_err = match unistd::read(&read_out, &mut tmp) {
                        Ok(n) => {
                            if n == 0 {
                                true
                            } else {
                                log.extend_from_slice(&tmp[..n]);
                                false
                            }
                        }
                        Err(Errno::EAGAIN) => false,
                        Err(Errno::EINTR) => false,
                        Err(_) => true,
                    };
                    if eof_or_err {
                        child_status = wait::waitpid(child, None).unwrap();
                        break;
                    }
                }

                /* Read all remaining data in the pipe, dropping anything
                 * that was read after receipt of the information of test process death */
                loop {
                    match unistd::read(&read_out, &mut tmp) {
                        Ok(n) => {
                            if n == 0 {
                                break;
                            }
                            log.extend_from_slice(&tmp[..n]);
                        }
                        Err(Errno::EINTR) => continue,
                        Err(_) => break,
                    }
                }

                if !quiet {
                    println!("captured output:");
                    println!("{}", String::from_utf8_lossy(&log));
                }

                match child_status {
                    wait::WaitStatus::Exited(pid, exit_code) => {
                        assert!(pid == child);
                        let e = if exit_code > (u8::MAX as i32) || exit_code < 0 {
                            u8::MAX
                        } else {
                            exit_code as u8
                        };
                        match e {
                            0 => TestCategory::Pass,
                            EXITCODE_SKIPPED => TestCategory::Skipped,
                            EXITCODE_UNCLEAR => TestCategory::Unclear,
                            _ => TestCategory::Fail,
                        }
                    }
                    wait::WaitStatus::Signaled(pid, signal, dump) => {
                        if false {
                            println!(
                                "Test process {} crashed with signal={}; coredump={}",
                                pid, signal, dump
                            );
                        }
                        /* The test process aborting signals either the Waypipe instance
                         * failing (making the test panic) or a major bug in the test. */
                        TestCategory::Fail
                    }
                    _ => {
                        todo!("Unexpected exit status");
                    }
                }
            }
        };

        let s = match res {
            TestCategory::Pass => "ok",
            TestCategory::Fail => {
                nfail += 1;
                "FAILED"
            }
            TestCategory::Skipped => {
                nskip += 1;
                "skipped"
            }
            TestCategory::Unclear => "UNCLEAR",
        };

        println!(" {}", s);
    }

    if nfail > 0 {
        ExitCode::FAILURE
    } else if nskip == tests.len() {
        /* This can happen in regular use, when there are no usable render devices */
        ExitCode::from(EXITCODE_SKIPPED)
    } else {
        ExitCode::SUCCESS
    }
}
