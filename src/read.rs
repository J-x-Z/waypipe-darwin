/* SPDX-License-Identifier: GPL-3.0-or-later */
/*! Structures to read messages from a socket and keep references to them */
use crate::tag;
use crate::util::*;
use log::debug;
use nix::errno::Errno;
use nix::libc;
use std::ffi::c_void;
use std::os::fd::AsRawFd;
use std::{collections::VecDeque, os::fd::OwnedFd, sync::Arc};

/** A region of memory filled by a ReadBuffer and referred to by some
 * number of ReadBufferView objects. */
struct ReadBufferChunk {
    data: *mut u8,
    size: usize,
}

// SAFETY: Multi-thread access is handled through ReadBufferView, of which only one is created
// for any given byte in data.
unsafe impl Sync for ReadBufferChunk {}
// SAFETY: It is safe to move and drop ReadBufferChunk in a different thread as dealloc is thread-safe
// (also because Arc is used, drop will only happen when there are no references.)
unsafe impl Send for ReadBufferChunk {}

/** A data structure to allow readv-ing a sequence of large packets, which can be
 * loaned to a different thread for some variable time before the space can be
 * reused.
 *
 * To limit the number of allocations and read operations needed, multiple packets may
 * share the same backing storage, which will only be freed when all packets are dropped. */
pub struct ReadBuffer {
    /* Buffer with one last message in it to complete; if present, last_msg_start/end refer
     * to _this_ buffer */
    old: Option<Arc<ReadBufferChunk>>,
    /* current.capacity() is the available space in the vector */
    current: Arc<ReadBufferChunk>,
    // span of the current partial message; everything _before_ this span has been loaned out
    // everything _after_ this span is uninitialized.
    last_msg_start: usize, /* Start of the in progress message/next message to be completed */
    end: usize,
    msg_queue: VecDeque<ReadBufferView>,
}

impl ReadBufferChunk {
    fn new(len: usize) -> ReadBufferChunk {
        // 4 alignment in theory allows for faster u32 reads when applying diff
        // (but only for uncompressed inputs), and may make Wayland parsing faster
        let layout = std::alloc::Layout::from_size_align(len, 4).unwrap();
        assert!(len > 0);
        unsafe {
            // SAFETY: have checked size is > 0
            let data = std::alloc::alloc(layout);
            assert!(!data.is_null());
            ReadBufferChunk { data, size: len }
        }
    }
}

impl Drop for ReadBufferChunk {
    fn drop(&mut self) {
        let layout = std::alloc::Layout::from_size_align(self.size, 4).unwrap();
        unsafe {
            // SAFETY: deallocating pointer only constructed through alloc with same layout
            std::alloc::dealloc(self.data, layout);
        }
    }
}

/** A reference to a single message from a ReadBuffer */
pub struct ReadBufferView {
    // Keep the chunk alive
    _base: Arc<ReadBufferChunk>, // alternatively: panic on drop, but let the main object consume these
    data: *mut u8,
    data_len: usize,
}

// SAFETY: Safe to move between threads, allocation in `.data` has no thread-specific ties.
// Furthermore, `._base` is Send+Sync (if Rc were used, moving to another thread could break it)
unsafe impl Send for ReadBufferView {}
// SAFETY: Safe to share between threads -- &/&mut self control data access
unsafe impl Sync for ReadBufferView {}

/** The maximum size expected for a typical message. Messages larger than this size may
 * require copying memory around to ensure an contiguous allocation. */
const MAX_NORMAL_MSG_SIZE: usize = 1 << 18;

impl ReadBuffer {
    /** Create a new ReadBuffer */
    pub fn new() -> Self {
        let chunksize = 4 * MAX_NORMAL_MSG_SIZE;

        ReadBuffer {
            old: None,
            current: Arc::new(ReadBufferChunk::new(chunksize)),
            last_msg_start: 0,
            end: 0,
            msg_queue: VecDeque::new(),
        }
    }

    /* Caller is responsible for verifying base[start..start+4] is available, initialized, constant. */
    unsafe fn get_message_padded_len(base: *const u8, start: usize) -> Result<usize, String> {
        // SAFETY: caller responsibility that start is inbounds and < isize::MAX
        let header_start = base.add(start);
        // SAFETY: *u8 is always aligned; caller responsibility that header_start
        // is in bounds, that base[start..start+4] is initialized, and nothing mutates
        // it. Slice length 4 < isize::MAX.
        let header_slice = std::ptr::slice_from_raw_parts(header_start, 4);
        let header = u32::from_le_bytes((&*header_slice).try_into().unwrap());
        let (msg_len, _typ) = parse_wmsg_header(header)
            .ok_or_else(|| tag!("Failed to parse wmsg header: {}", header))?;

        if msg_len < 4 {
            return Err(tag!("Message lengths must be at least 4, not {}", msg_len));
        }
        Ok(align4(msg_len))
    }

    /* bool is true iff EOF; usize = number of bytes read, may be zero if EINTR.
     * Caller must ensure iovecs indicate valid *mut pointers to regions of u8 data
     */
    unsafe fn read_inner(iovs: &[libc::iovec], src_fd: &OwnedFd) -> Result<(bool, usize), String> {
        assert!(iovs.iter().map(|x| x.iov_len).sum::<usize>() > 0);

        /* Cannot use uio::readv wrapper here, because it takes IoSliceMut which take
         * &mut [u8] which require "initialized" memory. Rust also currently has no
         * way to make memory as initialized, so to avoid UB, use raw libc and pointers
         * to also handle the first-write scenario. */

        // SAFETY: caller ensures iovs are valid and exclusively owned intervals; there
        // is no alignment requirement. Also, iovs.len() (typically 1 or 2) is < i32::MAX.
        #[cfg(not(miri))]
        let ret = libc::readv(src_fd.as_raw_fd(), iovs.as_ptr(), iovs.len() as _);
        #[cfg(miri)]
        let ret = test_readv(src_fd.as_raw_fd(), iovs.as_ptr(), iovs.len() as _);

        if ret < 0 {
            return match Errno::last() {
                Errno::ECONNRESET => Ok((true, 0)),
                Errno::EINTR | Errno::EAGAIN => Ok((false, 0)),
                x => Err(tag!("Error reading from channel socket: {:?}", x)),
            };
        }
        debug!("Read {} bytes from channel", ret);
        Ok((ret == 0, ret.try_into().unwrap()))
    }

    /* Copy the last incomplete message into a new buffer */
    fn move_tail_to_new(&mut self, new_size: usize) {
        let base = self.current.data;
        let nxt = Arc::new(ReadBufferChunk::new(new_size));

        /* Copy data over and adjust */
        assert!(new_size >= self.end - self.last_msg_start);
        unsafe {
            let c_dst = nxt.data;
            // SAFETY: inside allocation, because last_msg_start < old.size < isize::MAX
            let c_src = base.add(self.last_msg_start);
            // SAFETY: c_src/c_dst are from different allocations because self.current/
            // self.old never refer to the same chunk. u8 pointers are always aligned.
            // regions are valid, because self.end stays in [0,base.size]:
            // (c_src + self.end - self.last_msg_start) = base + self.end <= base.size
            // and have asserted (c_dst + self.end - self.last_msg_start) <= nxt.size
            std::ptr::copy_nonoverlapping(c_src, c_dst, self.end - self.last_msg_start);
        }

        self.current = nxt;
        self.end -= self.last_msg_start;
        self.last_msg_start = 0;
    }

    fn extract_messages(&mut self) -> Result<(), String> {
        let cur_ptr = self.current.data;
        while self.end - self.last_msg_start >= 4 {
            let msg_len = unsafe {
                // SAFETY: everything in cur_ptr[..self.end] is initialized,
                // last_msg_start + 4 < self.end, and self.end < current.size <= isize::MAX
                Self::get_message_padded_len(cur_ptr, self.last_msg_start)?
            };
            if self.end - self.last_msg_start >= msg_len {
                /* Message is complete, queue it */
                let ptr = unsafe {
                    // SAFETY: cur_ptr + self.last_msg_start is inside allocation of
                    // length current.size, which is > self.end > self.last_msg_start
                    cur_ptr.add(self.last_msg_start)
                };

                self.msg_queue.push_back(ReadBufferView {
                    _base: self.current.clone(),
                    data: ptr,
                    data_len: msg_len,
                });
                self.last_msg_start += msg_len;
            } else {
                /* Incomplete message, stop scanning */
                break;
            }
        }
        Ok(())
    }

    fn read_with_old(&mut self, src_fd: &OwnedFd) -> Result<bool, String> {
        let old = self.old.as_ref().unwrap();
        assert!(self.end - self.last_msg_start >= 4);

        let msg_len = unsafe {
            // SAFETY: everything in old.data[..self.end] is initialized,
            // have asserted last_msg_start + 4 < self.end,
            // and self.end < current.size <= isize::MAX
            Self::get_message_padded_len(old.data, self.last_msg_start)?
        };
        let msg_end = self.last_msg_start + msg_len;

        let (eof, mut nread) = unsafe {
            // SAFETY: self.end < old.size, so the pointer addition remains in bounds
            let iovs = [
                libc::iovec {
                    iov_base: old.data.add(self.end) as *mut c_void,
                    iov_len: msg_end - self.end,
                },
                libc::iovec {
                    iov_base: self.current.data as *mut c_void,
                    iov_len: self.current.size - MAX_NORMAL_MSG_SIZE,
                },
            ];
            // SAFETY: both intervals are contained in existing allocations
            // (The first ends at old + msg_end < old + last_msg_start + (last message len)
            // which read_more() ensures is < old.data + old.size; the second ends well
            // before current.data + current.size since MAX_NORMAL_MSG_SIZE >= 0.)
            // They are also not used anywhere else -- data after old.data + last_msg_start
            // has not yet been shared through a ReadBufferView, and nothing in self.current
            // was shared yet.
            Self::read_inner(&iovs, src_fd)?
        };

        if nread < msg_end - self.end {
            /* Did not complete reading current message */
            self.end += nread;
            return Ok(eof);
        }
        nread -= msg_end - self.end;

        let ptr = unsafe {
            // SAFETY: old.data + self.last_msg_start is inside allocation of length old.size
            old.data.add(self.last_msg_start)
        };

        let mut tmp = None;
        std::mem::swap(&mut self.old, &mut tmp);
        self.msg_queue.push_back(ReadBufferView {
            _base: tmp.unwrap(),
            data: ptr,
            data_len: msg_len,
        });

        self.last_msg_start = 0;
        self.end = nread;

        self.extract_messages()?;

        Ok(eof)
    }

    /** Try to read more input. Returns `true` iff source has no more data.
     *
     * Use `pop_next_msg()` to get the complete messages read, if there are any.
     */
    pub fn read_more(&mut self, src_fd: &OwnedFd) -> Result<bool, String> {
        if self.old.is_some() {
            return self.read_with_old(src_fd);
        }

        if self.end - self.last_msg_start >= 4 {
            let chunk: &ReadBufferChunk = &self.current;
            let cap = chunk.size;

            /* Message length is known */
            let msg_len = unsafe {
                // SAFETY: everything in chunk.data[..self.end] is initialized,
                // this branch requires last_msg_start + 4 < self.end,
                // and self.end < current.size <= isize::MAX
                Self::get_message_padded_len(chunk.data, self.last_msg_start)?
            };
            let msg_end = self.last_msg_start + msg_len;

            if msg_end > cap {
                /* Not enough space: must move message and create a new chunk for it */
                debug!(
                    "oversized message, length {}, end {} is > capacity {}",
                    msg_len, msg_end, cap
                );
                let new_size = std::cmp::max(2 * msg_len, 4 * MAX_NORMAL_MSG_SIZE);
                assert!(new_size > msg_len + MAX_NORMAL_MSG_SIZE);
                self.move_tail_to_new(new_size);
            } else if cap - msg_end <= MAX_NORMAL_MSG_SIZE {
                /* Complete this message, and start a new chunk */
                let new_size = 4 * MAX_NORMAL_MSG_SIZE;
                let mut nxt = Arc::new(ReadBufferChunk::new(new_size));

                std::mem::swap(&mut self.current, &mut nxt);
                self.old = Some(nxt);
                return self.read_with_old(src_fd);
            }
        } else {
            let chunk: &ReadBufferChunk = &self.current;
            let cap = chunk.size;

            /* No message or incomplete header */
            let msg_end = self.last_msg_start;
            assert!(msg_end <= cap);
            if cap - msg_end <= MAX_NORMAL_MSG_SIZE {
                /* Not enough space for a long message; start a new chunk (and move any partial header) */
                /* Not enough space: must move message and create a new chunk for it */
                debug!(
                    "partial header move, {} {}",
                    msg_end,
                    cap - MAX_NORMAL_MSG_SIZE,
                );
                let new_size = 4 * MAX_NORMAL_MSG_SIZE;
                self.move_tail_to_new(new_size);
            }
        }

        /* Try to read up to (cap - MAX_NORMAL_MSG_SIZE), which may be small; the exact message
         * breaking point can only be determined after reading. */
        let chunk: &ReadBufferChunk = &self.current;
        assert!(chunk.size - self.end >= MAX_NORMAL_MSG_SIZE);

        let (eof, nread) = unsafe {
            // SAFETY: at this point, have self.end < chunk.size, so iov_base is inside
            // chunk's allocation
            let iovs = [libc::iovec {
                iov_base: chunk.data.add(self.end) as *mut c_void,
                iov_len: chunk.size - MAX_NORMAL_MSG_SIZE - self.end,
            }];
            // SAFETY: Have asserted chunk.size - self.end >=  MAX_NORMAL_MSG_SIZE,
            // so iov_len does not overflow. Since MAX_NORMAL_MSG_SIZE >= 0, the
            // interval ends at or before chunk.data + chunk.size and is part of chunk's allocation
            // Exclusive access, because data past current.data + self.end has not
            // been shared via ReadBufferView
            Self::read_inner(&iovs, src_fd)?
        };
        self.end += nread;
        self.extract_messages()?;
        Ok(eof)
    }

    /** If there is a complete message available, extract and return it */
    pub fn pop_next_msg(&mut self) -> Option<ReadBufferView> {
        self.msg_queue.pop_front()
    }
}

impl ReadBufferView {
    /** Get an &mut-reference to the data of this buffer */
    pub fn get_mut(&mut self) -> &mut [u8] {
        unsafe {
            // SAFETY: ReadBufferView has exclusive access to the portion
            // self.data[..self.data_len] of the underlying ReadBufferChunk,
            // which is kept alive as long as self by self._base.
            // Data was initialized by ReadBuffer. u8 pointers are always aligned.
            // &mut self delegates exclusivity from ReadBufferView
            let dst = std::ptr::slice_from_raw_parts_mut(self.data, self.data_len);
            &mut *dst
        }
    }

    /** Get an &-reference to the data of this buffer */
    pub fn get(&self) -> &[u8] {
        unsafe {
            // SAFETY: ReadBufferView has exclusive access to the portion
            // self.data[..self.data_len] of the underlying ReadBufferChunk,
            // which is kept alive as long as self by self._base.
            // Data was initialized by ReadBuffer. u8 pointers are always aligned.
            // Mutable access only provided through &mut self which conflicts with &self
            let dst = std::ptr::slice_from_raw_parts_mut(self.data, self.data_len);
            &mut *dst
        }
    }

    /** Move the start of the buffer forward by the indicated amount
     *
     * This will panic if skip is larger than the remaining buffer size. */
    pub fn advance(&mut self, skip: usize) {
        assert!(skip % 4 == 0); // preserve alignment
        assert!(skip <= self.data_len, "{} <?= {}", skip, self.data_len);

        unsafe {
            // SAFETY: have asserted skip <= self.data_len, so the pointer addition
            // will be in the interval self.data .. self.data + self.data_len
            // all of which are in bounds of the ReadBufferChunk's allocation
            self.data = self.data.add(skip);
            self.data_len -= skip;
        }
    }
}

#[cfg(miri)]
use std::ffi::c_int;
/* miri does not currently support readv(), so implement it using read() */
#[cfg(miri)]
unsafe fn test_readv(fd: c_int, iovs: *const libc::iovec, len: c_int) -> isize {
    let mut nread: isize = 0;
    let mut first = true;
    for i in 0..(len as isize) {
        let iov = *iovs.offset(i);
        if iov.iov_len == 0 {
            continue;
        }
        let r = libc::read(fd, iov.iov_base, iov.iov_len);
        if r == -1 {
            if !first {
                return nread;
            } else {
                return -1;
            }
        }
        nread = nread.checked_add(r).unwrap();
        first = false;
    }
    nread
}

#[test]
fn test_read_buffer() {
    use nix::fcntl;
    use nix::poll;
    use nix::unistd;
    use std::os::fd::AsFd;
    use std::time::Instant;

    let mut rb = ReadBuffer::new();
    let (pipe_r, pipe_w) =
        crate::util::create_pipe(fcntl::OFlag::O_CLOEXEC | fcntl::OFlag::O_NONBLOCK).unwrap();

    #[cfg(not(miri))]
    fn read_all(rb: &mut ReadBuffer, fd: &OwnedFd) {
        loop {
            let mut p = [poll::PollFd::new(fd.as_fd(), poll::PollFlags::POLLIN)];
            let r = poll::poll(&mut p, poll::PollTimeout::ZERO);
            match r {
                Err(Errno::EINTR) => {
                    continue;
                }
                Err(Errno::EAGAIN) => {
                    break;
                }
                Err(x) => panic!("{:?}", x),
                Ok(_) => {
                    let rev = p[0].revents().unwrap();
                    if rev.contains(poll::PollFlags::POLLIN) {
                        rb.read_more(fd).unwrap();
                    } else if rev.intersects(poll::PollFlags::POLLHUP | poll::PollFlags::POLLERR) {
                        panic!();
                    } else {
                        return;
                    }
                }
            }
        }
    }
    #[cfg(miri)]
    fn read_all(rb: &mut ReadBuffer, fd: &OwnedFd) {
        // Calling `read_more` multiple times will consume the entire pipe
        // buffer, as long as it is not too large.
        for i in 0..20 {
            rb.read_more(fd).unwrap();
        }
    }

    let start = Instant::now();
    println!(
        "Many small messages, immediately dequeued: {}",
        Instant::now().duration_since(start).as_secs_f32()
    );
    {
        for i in 0..(MAX_NORMAL_MSG_SIZE / 2) {
            let mut small_msg = [0u8; 16];
            small_msg[..4]
                .copy_from_slice(&build_wmsg_header(WmsgType::AckNblocks, 16).to_le_bytes());
            small_msg[4..8].copy_from_slice(&(i as u32).to_le_bytes());

            unistd::write(&pipe_w, &small_msg).unwrap();

            read_all(&mut rb, &pipe_r);

            let nxt = rb.pop_next_msg().unwrap();
            assert!(nxt.get() == small_msg);
        }
    }

    println!(
        "Long input with many small chunks: {}",
        Instant::now().duration_since(start).as_secs_f32()
    );
    {
        let mut long_fragmented_input = Vec::<u8>::new();
        for i in 0..(2 * MAX_NORMAL_MSG_SIZE) {
            let mut small_msg = [0u8; 8];
            /* actual length of 7 pads to 8 */
            small_msg[..4]
                .copy_from_slice(&build_wmsg_header(WmsgType::AckNblocks, 7).to_le_bytes());
            small_msg[4..].copy_from_slice(&(i as u32).to_le_bytes());
            long_fragmented_input.extend_from_slice(&small_msg);
        }
        let mut x = 0;
        while x < long_fragmented_input.len() {
            let y = unistd::write(&pipe_w, &long_fragmented_input[x..]).unwrap();
            x += y;
            assert!(y > 0);
            rb.read_more(&pipe_r).unwrap();
        }
        /* Read remainder of data in case writing outpaced reads */
        read_all(&mut rb, &pipe_r);
        for i in 0..(2 * MAX_NORMAL_MSG_SIZE) {
            let nxt = rb.pop_next_msg().unwrap();
            let val = u32::from_le_bytes(nxt.get()[4..].try_into().unwrap());
            assert!(val == i as u32);
        }
    }

    println!(
        "Very long input, needs oversize buffer: {}",
        Instant::now().duration_since(start).as_secs_f32()
    );
    {
        let mut ultra_long_input = Vec::<u8>::new();
        let len = 10 * MAX_NORMAL_MSG_SIZE;
        ultra_long_input.resize(len, 0);
        ultra_long_input[..4]
            .copy_from_slice(&build_wmsg_header(WmsgType::Protocol, len).to_le_bytes());

        let mut x = 0;
        while x < ultra_long_input.len() {
            let y = unistd::write(&pipe_w, &ultra_long_input[x..]).unwrap();
            x += y;
            assert!(y > 0);

            rb.read_more(&pipe_r).unwrap();
        }
        read_all(&mut rb, &pipe_r);
        assert!(rb.pop_next_msg().unwrap().get_mut().len() == len);
    }

    /* This has the side effect of clearing out the overcapacity buffer */
    println!(
        "Many long chunks: {}",
        Instant::now().duration_since(start).as_secs_f32()
    );
    {
        let mut long_block_input = Vec::<u8>::new();
        let mut long_msg = vec![0; align4(MAX_NORMAL_MSG_SIZE)];
        long_msg[..4].copy_from_slice(
            &build_wmsg_header(WmsgType::AckNblocks, MAX_NORMAL_MSG_SIZE).to_le_bytes(),
        );
        for _ in 0..20 {
            long_block_input.extend_from_slice(&long_msg);
        }
        let mut x = 0;
        while x < long_block_input.len() {
            let y = unistd::write(&pipe_w, &long_block_input[x..]).unwrap();
            x += y;
            assert!(y > 0);

            rb.read_more(&pipe_r).unwrap();
        }
        read_all(&mut rb, &pipe_r);
        let mut concat = Vec::<u8>::new();
        while concat.len() < long_block_input.len() {
            concat.extend_from_slice(rb.pop_next_msg().unwrap().get());
        }
        assert!(concat == long_block_input);
    }

    println!(
        "Mixture of lengths, initially sent byte-by-byte: {}",
        Instant::now().duration_since(start).as_secs_f32()
    );
    {
        let mut long_mixed_input = Vec::<u8>::new();
        let mut i = 0;
        let zero_slice = &[0; 1004];
        while long_mixed_input.len() < 8 * MAX_NORMAL_MSG_SIZE {
            let length = 4 + i % 1000;
            i += 1;

            long_mixed_input
                .extend_from_slice(&build_wmsg_header(WmsgType::AckNblocks, length).to_le_bytes());
            long_mixed_input.extend_from_slice(&zero_slice[..align4(length - 4)]);
        }
        let mut x = 0;
        while x < long_mixed_input.len() {
            let step = if x < 10000 { 1 } else { 100 };
            let y = unistd::write(
                &pipe_w,
                &long_mixed_input[x..std::cmp::min(x + step, long_mixed_input.len())],
            )
            .unwrap();
            x += y;
            assert!(y > 0);

            rb.read_more(&pipe_r).unwrap();
        }
        read_all(&mut rb, &pipe_r);
        let mut concat = Vec::<u8>::new();
        while concat.len() < long_mixed_input.len() {
            let mut nxt = rb.pop_next_msg().unwrap();
            concat.extend_from_slice(&nxt.get()[..4]);
            nxt.advance(4);
            concat.extend_from_slice(nxt.get());
        }
        assert!(concat == long_mixed_input);
    }

    println!(
        "Done: {}",
        Instant::now().duration_since(start).as_secs_f32()
    );
}
