"""Archive layout and license-notice regression tests; no native binaries are executed."""

import hashlib
import json
from pathlib import Path
import plistlib
import tarfile
import tempfile
import unittest
from unittest.mock import patch
import zipfile

from package import package


class PackageTests(unittest.TestCase):
    def test_platform_layouts_checksums_and_font_notices(self):
        for target, executable in [
            ("x86_64-unknown-linux-gnu", "cadence"),
            ("x86_64-pc-windows-msvc", "cadence.exe"),
            ("x86_64-apple-darwin", "Cadence.app/Contents/MacOS/cadence"),
            ("aarch64-apple-darwin", "Cadence.app/Contents/MacOS/cadence"),
        ]:
            with self.subTest(target=target), tempfile.TemporaryDirectory() as temporary:
                root = Path(temporary)
                binary = root / "fixture-binary"
                binary.write_bytes(b"Packaging fixture; not an executable")
                crate = root / "dependency"
                (crate / "fonts").mkdir(parents=True)
                (crate / "fonts/OFL.txt").write_text("Embedded font license", encoding="utf-8")
                (crate / "LICENSE-MIT").write_text("Dependency license", encoding="utf-8")
                metadata = {"resolve": {"root": "application"}, "packages": [{
                    "id": "dependency", "name": "fixture", "version": "1.0.0",
                    "license": "MIT AND OFL-1.1", "manifest_path": str(crate / "Cargo.toml"),
                }]}
                with patch("package.subprocess.check_output", return_value=json.dumps(metadata)):
                    archive = package(target, binary, root / "output")
                checksum = Path(str(archive) + ".sha256").read_text().split()[0]
                self.assertEqual(checksum, hashlib.sha256(archive.read_bytes()).hexdigest())
                if archive.name.endswith(".tar.gz"):
                    with tarfile.open(archive) as stream:
                        members = stream.getmembers()
                        files = {m.name.split("/", 1)[1]: stream.extractfile(m).read() for m in members if m.isfile()}
                        self.assertTrue(next(m for m in members if m.name.endswith("/cadence")).mode & 0o111)
                else:
                    with zipfile.ZipFile(archive) as stream:
                        self.assertIsNone(stream.testzip())
                        files = {n.split("/", 1)[1]: stream.read(n) for n in stream.namelist()}
                        if "apple" in target:
                            info = next(i for i in stream.infolist() if i.filename.endswith("/MacOS/cadence"))
                            self.assertTrue((info.external_attr >> 16) & 0o111)
                self.assertIn(executable, files)
                self.assertIn("LICENSE", files)
                self.assertIn("docs/USER_GUIDE.md", files)
                self.assertIn(b"Embedded font license", files["THIRD_PARTY_LICENSES.txt"])
                if "apple" in target:
                    plist = plistlib.loads(files["Cadence.app/Contents/Info.plist"])
                    self.assertEqual(plist["CFBundleExecutable"], "cadence")
                    self.assertEqual(plist["CFBundlePackageType"], "APPL")


if __name__ == "__main__":
    unittest.main()
