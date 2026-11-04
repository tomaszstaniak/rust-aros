import os
from pathlib import Path
import subprocess
import unittest

SOURCE = Path(os.environ['AROS_OFFICIAL_SOURCE']).resolve()


class MetaMakeTests(unittest.TestCase):
    def evaluate(self, cpu, toolchain):
        makefile = ('AROS_TARGET_CPU := ' + cpu + '\nAROS_TOOLCHAIN := ' + toolchain +
                    '\nTARGET_RUST_VER := nightly-2026-08-04\nAROS_HOST_CPU := aarch64\nAROS_HOST_ARCH := darwin\n' +
                    'CROSSTOOLSDIR := /private/tools\ninclude ' + str(SOURCE / 'tools/crosstools/rust.cfg') +
                    '\nall:\n\t@echo $(RUST_OFFICIAL) $(RUST_TARGET_TRIPLE)\n')
        return subprocess.run(['make', '-f', '-'], input=makefile, text=True, capture_output=True)

    def test_specific_cpu_toolchain_admission(self):
        for cpu, toolchain in [('aarch64', 'llvm'), ('x86_64', 'llvm'), ('x86_64', 'gnu')]:
            result = self.evaluate(cpu, toolchain)
            self.assertEqual(result.returncode, 0, result.stderr)
            self.assertEqual(result.stdout.strip(), 'yes ' + cpu + '-unknown-aros')
        for cpu, toolchain in [('aarch64', 'gnu'), ('i386', 'llvm'), ('powerpc', 'gnu')]:
            self.assertNotEqual(self.evaluate(cpu, toolchain).returncode, 0)

    def test_explicit_tools_and_driver_are_forwarded(self):
        recipe = (SOURCE / 'tools/crosstools/rust/mmakefile.src').read_text()
        self.assertIn('--crosstools $(RUST_OFFICIAL_TOOLS)', recipe)
        self.assertIn('--cc $(RUST_TARGET_CC)', recipe)
        self.assertIn('--target $(RUST_TARGET_TRIPLE)', recipe)
        self.assertIn('--posix-stubs $(RUST_OFFICIAL_POSIX_STUBS)', recipe)

    def test_gnu_does_not_gain_undeclared_llvm_default(self):
        for name in ('configure', 'configure.in'):
            code = (SOURCE / name).read_text()
            default = code.split('default_rust_version="1.98.1"', 1)[1].split('fi', 1)[0]
            self.assertNotIn('x86_64:gnu', default)
