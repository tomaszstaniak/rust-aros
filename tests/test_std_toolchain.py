import os
from pathlib import Path
import shutil
import subprocess
import tempfile
import unittest


ROOT = Path(__file__).resolve().parents[1]


class InstallerTests(unittest.TestCase):
    def setUp(self):
        self.tmp = tempfile.TemporaryDirectory(prefix='rust aros ')
        self.addCleanup(self.tmp.cleanup)
        self.root = Path(self.tmp.name)
        self.repo = self.root / 'repo'
        patch = self.repo / 'std-groundwork/std-patch'
        patch.mkdir(parents=True)
        self.script = patch / 'make-std-toolchain.sh'
        shutil.copy2(ROOT / 'std-groundwork/std-patch/make-std-toolchain.sh', self.script)
        (patch / 'patch-std.py').write_text('import sys\n')
        libc = patch.parent / 'libc-aros'
        for name in ('src', 'csrc'):
            (libc / name).mkdir(parents=True)
        for name in ('Cargo.toml', 'build.rs'):
            (libc / name).write_text('')
        self.base = self.root / 'base'
        library = self.base / 'lib/rustlib/src/rust/library'
        (library / 'std/src').mkdir(parents=True)
        (library / 'Cargo.toml').write_text('[patch.crates-io]\n')
        (self.base / 'bin').mkdir()
        rustc = self.base / 'bin/rustc'
        rustc.write_text('#!/bin/sh\nprintf \'%s\\n\' "$@" > "$TEST_RUSTC_ARGS"\necho compiler\n')
        rustc.chmod(0o755)
        bindir = self.root / 'bin'
        bindir.mkdir()
        rustup = bindir / 'rustup'
        rustup.write_text('''#!/bin/sh
case "$*" in
  *"--print sysroot"*) printf '%s\\n' "$TEST_SYSROOT" ;;
  *"rustc -V"*) echo "${TEST_VERSION:-rustc 1.100.0-nightly (0dfb098f3 2026-08-31)}" ;;
  "toolchain list") printf '%s\\n' "${TEST_NAMES:-nightly-aarch64-apple-darwin}" ;;
  "toolchain link "*) [ "${TEST_LINK_FAIL:-0}" = 0 ] || exit 9
                       printf '%s\\n' "$*" >> "$TEST_LINK_LOG" ;;
  *) exit 2 ;;
esac
''')
        rustup.chmod(0o755)
        self.log = self.root / 'links'
        self.rustc_args = self.root / 'rustc-args'
        self.env = dict(os.environ, PATH=f'{bindir}:{os.environ["PATH"]}',
                        TEST_SYSROOT=str(self.base), TEST_LINK_LOG=str(self.log),
                        TEST_RUSTC_ARGS=str(self.rustc_args))
        self.dest = self.root / 'output with spaces'

    def run_script(self, *args):
        return subprocess.run(['sh', str(self.script), *map(str, args)],
                              env=self.env, text=True, capture_output=True)

    def test_requires_destination_and_name(self):
        result = self.run_script()
        self.assertNotEqual(result.returncode, 0)
        self.assertIn('usage:', result.stderr)
        self.assertFalse(self.log.exists())

    def test_refuses_existing_destination_without_changing_it(self):
        self.dest.mkdir()
        (self.dest / 'sentinel').write_text('keep')
        result = self.run_script(self.dest, 'aros-test')
        self.assertNotEqual(result.returncode, 0)
        self.assertEqual((self.dest / 'sentinel').read_text(), 'keep')
        self.assertFalse(self.log.exists())

    def test_refuses_shared_name(self):
        result = self.run_script(self.dest, 'aros-nightly')
        self.assertNotEqual(result.returncode, 0)
        self.assertFalse(self.dest.exists())
        self.assertFalse(self.log.exists())

    def test_wrong_compiler_does_not_create_destination(self):
        self.env['TEST_VERSION'] = 'rustc 1.101.0-nightly (new 2026-10-01)'
        result = self.run_script(self.dest, 'aros-test')
        self.assertNotEqual(result.returncode, 0)
        self.assertFalse(self.dest.exists())

    def test_patch_failure_cleans_only_new_destination(self):
        (self.script.parent / 'patch-std.py').write_text('raise SystemExit(7)\n')
        result = self.run_script(self.dest, 'aros-test')
        self.assertNotEqual(result.returncode, 0)
        self.assertFalse(self.dest.exists())
        self.assertFalse(self.log.exists())

    def test_builds_named_toolchain_with_spaces_and_refuses_rerun(self):
        result = self.run_script(self.dest, 'aros-test')
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertTrue((self.dest / 'lib/rustlib/src/rust/library/libc-aros/build.rs').is_file())
        self.assertIn(f'toolchain link aros-test {self.dest}', self.log.read_text())
        result = self.run_script(self.dest, 'aros-test')
        self.assertNotEqual(result.returncode, 0)
        self.assertEqual(len(self.log.read_text().splitlines()), 1)

    def test_wrapper_preserves_literal_shell_characters(self):
        self.dest = self.root / 'output $literal `text` "quote" \'single\''
        result = self.run_script(self.dest, 'aros-test')
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertEqual(self.rustc_args.read_text().splitlines(),
                         ['--sysroot', str(self.dest), '-V'])

    def test_refuses_registered_name(self):
        self.env['TEST_NAMES'] = 'aros-test (default)'
        result = self.run_script(self.dest, 'aros-test')
        self.assertNotEqual(result.returncode, 0)
        self.assertFalse(self.dest.exists())
        self.assertFalse(self.log.exists())

    def test_link_failure_cleans_new_destination(self):
        self.env['TEST_LINK_FAIL'] = '1'
        result = self.run_script(self.dest, 'aros-test')
        self.assertNotEqual(result.returncode, 0)
        self.assertFalse(self.dest.exists())
        self.assertFalse(self.log.exists())

    def test_refuses_dangling_destination_symlink(self):
        self.dest.symlink_to(self.root / 'missing')
        result = self.run_script(self.dest, 'aros-test')
        self.assertNotEqual(result.returncode, 0)
        self.assertTrue(self.dest.is_symlink())
        self.assertFalse(self.log.exists())

    def test_base_wrapper_preserves_literal_shell_characters(self):
        moved = self.root / 'base $literal `text` "quote" \'single\''
        self.base.rename(moved)
        self.env['TEST_SYSROOT'] = str(moved)
        result = self.run_script(self.dest, 'aros-test')
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertEqual(self.rustc_args.read_text().splitlines(),
                         ['--sysroot', str(self.dest), '-V'])


if __name__ == '__main__':
    unittest.main()
