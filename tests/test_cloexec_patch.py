import ast
from pathlib import Path
import subprocess
import tempfile
import unittest

ROOT = Path(__file__).resolve().parents[1]
# Complementary alternatives matching std's FileDesc layout. Compile the
# patched selection, so putting AROS in only one list fails rather than passes.
FIXTURE = r'''impl FileDesc {
    #[cfg(not(any(
        target_os = "haiku",
    )))]
    pub fn set_cloexec(&self) -> &'static str { "ioctl" }
    #[cfg(any(
        target_os = "haiku",
    ))]
    pub fn set_cloexec(&self) -> &'static str { "fcntl" }
    pub fn set_nonblocking(&self) {}
}
'''


class CloexecPatch(unittest.TestCase):
    def patcher(self):
        path = ROOT / 'std-groundwork/std-patch/patch-std.py'
        source = path.read_text()
        function = next(node for node in ast.parse(source).body
                        if isinstance(node, ast.FunctionDef)
                        and node.name == 'cloexec_with_fcntl')
        namespace = {}
        exec(ast.get_source_segment(source, function), namespace)
        return namespace['cloexec_with_fcntl']

    def test_aros_selects_fcntl_and_other_targets_are_preserved(self):
        patched = self.patcher()(FIXTURE)
        for target, expected in [('aros', 'fcntl'), ('haiku', 'fcntl'), ('other', 'ioctl')]:
            with self.subTest(target=target), tempfile.TemporaryDirectory(prefix='aros-cloexec-') as directory:
                root = Path(directory)
                source = root / 'check.rs'
                source.write_text('struct FileDesc;\n' + patched +
                                  f'fn main() {{ assert_eq!(FileDesc.set_cloexec(), "{expected}"); }}')
                binary = root / 'check'
                result = subprocess.run(['rustc', str(source), '--cfg',
                                         f'target_os="{target}"',
                                         '-Aexplicit_builtin_cfgs_in_flags', '-o', str(binary)],
                                        capture_output=True, text=True)
                self.assertEqual(result.returncode, 0, result.stderr)
                result = subprocess.run([str(binary)], capture_output=True, text=True)
                self.assertEqual(result.returncode, 0, result.stderr)

    def test_changed_cfg_layout_is_rejected(self):
        with self.assertRaises(AssertionError):
            self.patcher()(FIXTURE.replace('        target_os = "haiku",', '', 1))
