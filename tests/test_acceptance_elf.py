import importlib.util
from pathlib import Path
import unittest

spec = importlib.util.spec_from_file_location('acceptance_build', Path(__file__).resolve().parents[1] / 'tools/build-acceptance.py')
builder = importlib.util.module_from_spec(spec)
spec.loader.exec_module(builder)


class ElfProfileTest(unittest.TestCase):
    def header(self, abi, machine):
        data = bytearray(64)
        data[:9] = bytes.fromhex('7f454c460201010f') + bytes([abi])
        data[16:18] = (1).to_bytes(2, 'little')
        data[18:20] = machine.to_bytes(2, 'little')
        return data

    def test_v11_is_accepted_only_for_x86_profile(self):
        header = self.header(11, 62)
        builder.validate_elf(header, 'x86_64-v11')
        with self.assertRaises(ValueError):
            builder.validate_elf(header, 'aarch64-v1-raspi')

    def test_arm_v1_is_accepted_and_mixed_abi_is_rejected(self):
        builder.validate_elf(self.header(1, 183), 'aarch64-v1-raspi')
        with self.assertRaises(ValueError):
            builder.validate_elf(self.header(11, 183), 'aarch64-v1-raspi')
