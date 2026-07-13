Waypipe-Darwin
================================================================================

This fork keeps Waypipe usable as a native macOS client while staying close to
the upstream 0.11 codebase. The Darwin-specific work covers shared memory,
Unix sockets, process handling, and the transport path used by Cocoa-Way.

`waypipe` is a proxy for [Wayland][l0] clients. It forwards Wayland messages and
serializes changes to shared memory buffers over a single socket. This makes
application forwarding similar to [`ssh -X`][l1] feasible.

[l0]: https://wayland.freedesktop.org/
[l1]: https://wiki.archlinux.org/title/OpenSSH#X11_forwarding

## Install on macOS

```bash
brew tap J-x-Z/tap
brew install waypipe-darwin
```

## Usage

`waypipe` should be installed on both the local and remote computers. There is a
user-friendly command line pattern which prefixes a call to `ssh` and
automatically sets up a reverse tunnel for protocol data. For example,

```
waypipe ssh user@theserver weston-terminal
```

will run `ssh`, connect to `theserver`, and remotely run `weston-terminal`,
using local and remote `waypipe` processes to synchronize the shared memory
buffers used by Wayland clients between both computers. Command line arguments
before `ssh` apply only to `waypipe`; those after `ssh` belong to `ssh`.

Alternatively, one can launch the local and remote processes by hand, with the
following set of shell commands:

```
/usr/bin/waypipe -s /tmp/socket-local client &
ssh -R /tmp/socket-remote:/tmp/socket-local -t user@theserver \
    /usr/bin/waypipe -s /tmp/socket-remote server -- \
    /usr/bin/weston-terminal
kill %1
```

For more detailed examples, see `waypipe`'s man page.

## Installing

Waypipe's build uses a mixture of [meson][i0] and [cargo][i1]. For example:
```
cd /path/to/waypipe/
cargo fetch --locked
cd ..
mkdir build-waypipe
meson --buildtype debugoptimized waypipe build-waypipe
ninja -C build-waypipe install
```

Core build requirements:

- [meson][i0] (build, ≥ 0.57. with dependencies `ninja`, `pkg-config`, `python3`)
- [cargo][i1] (build)
- rust (build, edition 2021, version ≥ 1.77)
- [scdoc][i2] (optional, to generate a man page, )
- ssh (runtime, OpenSSH ≥ 6.7, for Unix domain socket forwarding)

Rust library dependencies (can be acquired through `cargo fetch --locked`):

- clap: command line parsing
- getrandom: picking random socket names
- log: logging
- nix: safer libc wrapper
- ash: (optional, provides vulkan bindings for dmabuf support)
- libloading: (optional, used by ffmpeg bindings)
- pkg-config: (optional, used to find libraries to link to)

Optional linked dependencies, broken out by feature:

- lz4 compression:
  - liblz4 (≥ 1.7.0)
  - bindgen (build, ≥ 0.70.0)
- zstd compression:
  - libzstd (≥ 0.4.6)
  - bindgen (build, ≥ 0.70.0)
- dmabuf support:
  - vulkan (to support programs using GPU rendering and DMABUFs)
  - vulkan validation layers (runtime, optional, used for tests and with --debug flag)
- dmabuf support fallback with gbm:
  - libgbm (mesa, ≥ 11.0.0)
  - bindgen (build, ≥ 0.70.0)
- video encoding/decoding support:
  - dmabuf support
  - ffmpeg (≥ 7.1, needs avcodec/avutil for lossy video encoding)
  - bindgen (build, ≥ 0.70.0)
  - glslc (build, to compile shaders for image format conversion)

Note: in practice, bindgen requires certain C library headers from clang, but some
distributions have not made them a dependency of bindgen. If the build fails
because `limits.h` or `stddef.h` is missing, try installing `clang`.

The program [xwayland-satellite][i3] may be used at runtime iff the `--xwls`
option is provided.

[i0]: https://mesonbuild.com/
[i1]: https://doc.rust-lang.org/cargo/
[i2]: https://git.sr.ht/~sircmpwn/scdoc
[i3]: https://github.com/Supreeeme/xwayland-satellite

### `waypipe-c`

Originally, Waypipe was developed in C; it was later ported to use Rust. The
C implementation, now producing an executable called `waypipe-c`, has been kept
in the repository for use on older systems which do not have the Rust version's
dependencies installed. `waypipe-c` also includes some features (like reconnection
support) dropped in later versions. There are two ways to build it: with `meson`,
```
cd /path/to/waypipe/ && cd ..
mkdir build-waypipe
meson --buildtype debugoptimized waypipe build-waypipe -Dbuild_c=true -Dbuild_rs=false
ninja -C build-waypipe install
```
or by running the `./minimal_build.sh` script. In addition to `meson`, `python`,
`liblz4`, `libzstd`, `ssh`, `waypipe-c` requires:

- ffmpeg (≥ 3.1, needs avcodec/avutil/swscale for lossy video encoding)
- libva (for hardware video encoding and decoding)
- libgbm (to support programs using OpenGL via DMABUFs)
- libdrm (same as for libgbm)
- sys/sdt.h (to provide static tracepoints for profiling)
- libwayland-client (for security context support)

## License

`waypipe` is licensed GPLv3-or-later; `waypipe-c` is MIT. In both cases, the
compiled executable is derived from the Wayland protocol files in `./protocols`,
which have their own (permissive) licenses. `waypipe`'s Rust dependency tree
can be shown with `cargo tree`. See git history for a list of authors.

## Reporting issues

Waypipe is developed at [gitlab.freedesktop.org/mstoeckl/waypipe][r0]; 
file bug reports or submit patches here.

In general, if a program does not work properly under Waypipe, it is a bug worth
reporting. If possible, before doing so ensure both computers are using the most
recently released version of Waypipe (or are built from git master).

A workaround that may help for some programs using OpenGL or Vulkan is to run
Waypipe with the `--no-gpu` flag, which may force them to use software rendering
and shared memory buffers. (Please still file a bug.)

Some programs may require specific environment variable settings or command line
flags to run remotely; a few examples are given in the [man page][r1].

Useful information for bug reports includes:

- If a Waypipe process has crashed on either end of the connection, a full stack
  trace, with debug symbols. (In gdb, `bt full`).
- If the program uses OpenGL or Vulkan, the graphics cards and drivers on both
  computers.
- The output of `waypipe --version` on both ends of the connection
- Logs when Waypipe is run with the `--debug` flag, or when the program is run
  with the environment variable setting `WAYLAND_DEBUG=1`.
- Screenshots of any visual glitches.

[r0]: https://gitlab.freedesktop.org/mstoeckl/waypipe/
[r1]: https://gitlab.freedesktop.org/mstoeckl/waypipe/-/blob/master/waypipe.scd

## Technical Limitations

### Security

Waypipe sends Wayland messages and updates to file descriptors (like window surface
content or copy-paste data transfers) over a socket. When these messages are
sent over a network, even if encrypted and with ssh's (lightweight) timing mitigations,
an observer may be able to guess what is being done (typing, mouse motion, scrolling
a page, and more...) from the sizes and timing of messages alone, even without
knowing the precise contents.

Against broken or malicious compositors or applications: Waypipe does not impose
resource limits, and the applications or compositors it forwards may instruct it
to allocate large amounts of memory or do CPU or GPU-intensive work. Waypipe exposes
compression and video encoding libraries which parse complicated formats and may
have potential vulnerabilities; of these the compression libraries are simpler
and better tested.

See the man page for more details.

### Partial protocol processing

Waypipe does not have a full view of the Wayland protocol. It includes a
compiled form of the base protocol and several extension protocols, but is not
able to parse all messages that the programs it connects send. Fortunately, the
Wayland wire protocol is partially self-describing, so Waypipe can parse the
messages it needs (those related to resources shared with file descriptors)
while ignoring the rest. This makes Waypipe partially forward-compatible: if a
future protocol comes out about details (for example, about window positioning)
which do not require that file descriptors be sent, then applications will be
able to use that protocol even with older versions of Waypipe. The tradeoff to
allowing messages that Waypipe can not parse is that Waypipe can only make minor
modifications to the wire protocol. In particular, adding or removing any
Wayland protocol objects would require changing all messages that refer to them,
including those messages that Waypipe does not parse. This precludes, for
example, global object deduplication tricks that could reduce startup time for
complicated applications.

### Latency

Shared memory buffer updates, including those for the contents of windows, are
tracked by keeping a "mirror" copy of the buffer the represents the view which
the opposing instance of Waypipe has. This way, Waypipe can send only the
regions of the buffer that have changed relative to the remote copy. This is
more efficient than resending the entire buffer on every update, which is good
for applications with reasonably static user interfaces (like a text editor or
email client). However, with programs with animations where the interaction
latency matters (like games or certain audio tools), major window updates will
unavoidably produce a lag spike. The additional memory cost of keeping mirrors
is moderate.

The ssh ObscureKeystrokeTiming feature may introduce delays to obscure when
input events and responses occur; reducing the delay interval should reduce
latency/improve frame rates at the cost of sending more packets.

### Other

The video encoding option for DMABUFs currently maintains a video stream for
each buffer that is used by a window surface. Since surfaces typically rotate
between a small number of buffers, a video encoded window will appear to flicker
as it switches rapidly between the underlying buffers, each of whose video
streams has different encoding artifacts.

As of writing, hardware video support with Vulkan is somewhat experimental and may
require that driver-specific environment variables be set.

Since little-endian computers are vastly more common than big-endian, Waypipe
only receives and produces little-endian Wayland protocol messages. For
big-endian machines, run applications under a tool like
[`wswapendian`](https://gitlab.freedesktop.org/mstoeckl/wswapendian) to adjust
the protocol endianness. (Having Waypipe do this itself would require that it
embed or load many more Wayland protocol descriptions and restrict clients to
use them; at the moment it is more practical to do the endianness conversion
in a separate program.)
