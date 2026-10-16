"""Explicit native panic profiles shared by WAT and acceptance builds."""

def native_options(repo, profile, environ):
    if profile not in ('abort', 'unwind'):
        raise ValueError('panic profile must be abort or unwind')
    overrides = [name for name in ('RUSTFLAGS', 'CARGO_ENCODED_RUSTFLAGS', 'CARGO_BUILD_RUSTFLAGS',
                                   'CARGO_TARGET_X86_64_AROS_RUSTFLAGS', 'CARGO_PROFILE_RELEASE_PANIC')
                 if environ.get(name)]
    if overrides:
        raise ValueError('clear ambient ' + ', '.join(overrides) + ' before selecting a native profile')
    flags = ['-Cpanic=' + profile]
    if profile == 'unwind':
        flags.append('-Cforce-unwind-tables=yes')
    import json
    return ['--target', str(repo / 'x86_64-aros.json'),
            '-Zbuild-std=std,panic_' + profile, '-Zjson-target-spec',
            '--config', 'build.rustflags=' + json.dumps(flags)]
