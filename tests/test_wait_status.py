from pathlib import Path
import subprocess
import tempfile
import unittest

ROOT = Path(__file__).resolve().parents[1]


class WaitStatus(unittest.TestCase):
    def test_aros_exit_status_contract(self):
        # SDK aros/posixc/sys/wait.h: raw exit code, no POSIX signals.
        with tempfile.TemporaryDirectory(prefix="aros-wait-") as directory:
            root = Path(directory)
            library = root / "libaros_libc.rlib"
            result = subprocess.run([
                "rustc", "--edition=2021", "--crate-name", "aros_libc",
                "--crate-type", "rlib",
                str(ROOT / "std-groundwork/libc-aros/src/lib.rs"),
                "-o", str(library)], capture_output=True, text=True)
            self.assertEqual(result.returncode, 0, result.stderr)
            harness = root / "check.rs"
            harness.write_text(r'''extern crate aros_libc as libc;
fn main() {
    for status in [0, 1, 7, 9, 20, 127, 128, 255, 256, 0xffff] {
        assert!(libc::WIFEXITED(status));
        assert_eq!(libc::WEXITSTATUS(status), status & 255);
        assert!(!libc::WIFSIGNALED(status));
        assert!(!libc::WIFSTOPPED(status));
        assert!(!libc::WIFCONTINUED(status));
        assert!(!libc::WCOREDUMP(status));
        assert_eq!(libc::WTERMSIG(status), 0);
        assert_eq!(libc::WSTOPSIG(status), 0);
    }
}
''')
            binary = root / "check"
            result = subprocess.run(["rustc", str(harness), "--extern",
                                     f"aros_libc={library}", "-o", str(binary)],
                                    capture_output=True, text=True)
            self.assertEqual(result.returncode, 0, result.stderr)
            result = subprocess.run([str(binary)], capture_output=True, text=True)
            self.assertEqual(result.returncode, 0, result.stdout + result.stderr)
