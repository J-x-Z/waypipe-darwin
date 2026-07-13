project(
    'waypipe',
    'c',
    license: ['GPL3+', 'MIT/Expat'],
    meson_version: '>=0.56.0',
    default_options: ['c_std=c11', 'warning_level=3', 'werror=true'],
    version: '0.11.0',
)

# mention version
raw_version = '@0@'.format(meson.project_version())
git = find_program('git', native: true, required: false)
if git.found()
    dir_arg = '--git-dir=@0@/.git'.format(meson.project_source_root())
    commit = run_command(
        [git, dir_arg, 'rev-parse', '--verify', '-q', 'HEAD'],
        check: false,
    )
    if commit.returncode() == 0
        raw_version = '@0@ (commit @1@)'.format(
            meson.project_version(),
            commit.stdout().strip(),
        )
    endif
endif
version = '"@0@"'.format(raw_version)

if get_option('build_rs')
    cargo = find_program('cargo', native: true)
    sh = find_program('sh', native: true)

    # Because meson and cargo interact badly, option implementation and dependency resolution are deferred to cargo
    # However, to 'autodetect' what features are available, we duplicate some of the build logic
    features = []
    has_lz4 = false
    has_zstd = false
    has_gbm = false
    has_dmabuf = 'false'
    has_video = false
    is_linux = host_machine.system() == 'linux'

    # DMA-BUF, GBM and the video path depend on Linux kernel APIs such as
    # eventfd and DRM ioctls. MoltenVK can satisfy the Vulkan probe on macOS,
    # but it cannot make those Linux interfaces available.
    if not is_linux
        foreach option_name : ['with_dmabuf', 'with_gbm', 'with_video']
            if get_option(option_name).enabled()
                error('@0@ is only supported on Linux'.format(option_name))
            endif
        endforeach
    endif

    # compute maximum of with_lz4/with_zstd/with_gbm/with_video features
    max_feature = get_option('with_lz4')
    if (max_feature.auto() and get_option('with_zstd').enabled()) or (not max_feature.enabled() and get_option(
        'with_zstd',
    ).auto())
        max_feature = get_option('with_zstd')
    endif
    if is_linux
        if (max_feature.auto() and get_option('with_gbm').enabled()) or (not max_feature.enabled() and get_option(
            'with_gbm',
        ).auto())
            max_feature = get_option('with_gbm')
        endif
        if (max_feature.auto() and get_option('with_video').enabled()) or (not max_feature.enabled() and get_option(
            'with_video',
        ).auto())
            max_feature = get_option('with_video')
        endif
    endif

    bindgen = find_program(
        'bindgen',
        version: '>= 0.66.0',
        native: true,
        required: max_feature,
    )
    liblz4 = dependency(
        'liblz4',
        version: '>=1.7.0',
        required: get_option('with_lz4'),
    )
    if bindgen.found() and liblz4.found()
        has_lz4 = true
        features += ['lz4']
    endif
    libzstd = dependency(
        'libzstd',
        version: '>=0.4.6',
        required: get_option('with_zstd'),
    )
    if bindgen.found() and libzstd.found()
        has_zstd = true
        features += ['zstd']
    endif
    if is_linux
        libgbm = dependency(
            'gbm',
            version: '>=11.0.0',  # gbm is part of Mesa and has a matching version
            required: get_option('with_gbm'),
        )
        if bindgen.found() and libgbm.found()
            has_dmabuf = 'gbm'
            features += ['gbmfallback']
        endif
        libvulkan = dependency('vulkan', required: get_option('with_dmabuf'))
        if libvulkan.found()
            # Note: 'ash' is the vulkan wrapper used, and may require other libraries/programs to be present
            has_dmabuf = 'true'
            features += ['dmabuf']
        endif
        glslc = find_program(
            'glslc',
            native: true,
            required: get_option('with_video'),
        )
        libavutil = dependency(
            'libavutil',
            version: '>=58.11.100',
            required: get_option('with_video'),
        )
        libavcodec = dependency(
            'libavcodec',
            version: '>=59.0.100',
            required: get_option('with_video'),
        )
        if bindgen.found() and libavcodec.found() and libavutil.found() and glslc.found()
            has_video = true
            features += ['video']
        endif
    endif

    target_dir = meson.project_build_root() / 'target'
    output = target_dir / 'release/waypipe'
    manifest = meson.project_source_root() / 'Cargo.toml'

    # Cargo has no direct analogue for 'g' or 'plain' optimization levels,
    # so the choices for them are somewhat arbitrary
    if get_option('optimization') == '0' or get_option('optimization') == 'g'
        profile = 'meson-0'
    elif get_option('optimization') == '1' or get_option('optimization') == 'plain'
        profile = 'meson-1'
    elif get_option('optimization') == '3'
        profile = 'meson-3'
    elif get_option('optimization') == 's'
        profile = 'meson-s'
    else
        profile = 'meson-2'
    endif

    if get_option('debug') == false
        profile += '-strip'
    endif
    message('Cargo will be run with profile: @0@'.format(profile))

    binary = custom_target(
        'waypipe',
        build_always_stale: true,
        output: 'waypipe',
        command: [
            sh,
            '@SOURCE_ROOT@' / 'compile_wrapper.sh',
            profile,
            manifest,
            target_dir,
            ','.join(features),
            '@OUTPUT@',
            raw_version,
            has_lz4.to_string('true', 'false'),
            has_zstd.to_string('true', 'false'),
            has_dmabuf,
            has_video.to_string('true', 'false'),
        ],
        install: true,
        install_dir: get_option('bindir'),
    )

    test(
        'running cargo test',
        cargo,
        args: [
            'test',
            '--frozen',
            '--bin',
            'waypipe',
            '--test',
            'proto',
            '-v',
            '--manifest-path',
            manifest,
            '--no-default-features',
            '--target-dir',
            target_dir,
            '--features',
            ','.join(features + ['test_proto']),
        ],
        timeout: 200,
        priority: 100,
        is_parallel: false,
    )

    test(
        'running (shortened) waypipe bench',
        sh,
        args: [
            '-c',
            ' '.join([binary.full_path(), '--test-fast-bench', 'bench']),
        ],
        depends: binary,
        is_parallel: false,
    )
endif

if get_option('build_c')
    subdir('waypipe-c')
endif

scdoc = dependency(
    'scdoc',
    version: '>=1.9.4',
    native: true,
    required: get_option('man-pages'),
)
if scdoc.found()
    scdoc_prog = find_program(
        scdoc.get_variable(pkgconfig: 'scdoc'),
        native: true,
    )
    sh = find_program('sh', native: true)
    mandir = get_option('mandir')
    custom_target(
        'waypipe.1',
        input: 'waypipe.scd',
        output: 'waypipe.1',
        command: [
            sh,
            '-c',
            '@0@ < @INPUT@ > @1@'.format(scdoc_prog.full_path(), 'waypipe.1'),
        ],
        install: true,
        install_dir: '@0@/man1'.format(mandir),
    )
endif
