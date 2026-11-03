"""Check packaged payloads without opening the GUI or submitting a print job."""
import argparse
import hashlib
import os
from pathlib import Path
import plistlib
import struct
import subprocess
import tarfile
import tempfile
import zipfile

ROOT = Path(__file__).resolve().parents[1]


def smoke(binary):
    subprocess.run([str(binary), "--version"], check=True, timeout=30)
    fixture=ROOT / "target/native-sources/libheif-1.23.1/examples/example.heic"
    # Do not let development overrides hide missing package libraries.
    environment=os.environ.copy()
    environment.pop("PRINTMARTEN_NATIVE_LIB_DIR",None)
    subprocess.run([str(binary), "--check-image", str(fixture)], check=True, timeout=60, env=environment)


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--platform", required=True)
    parser.add_argument("--arch", required=True)
    parser.add_argument("--sha", help="Validate only this build suffix")
    options = parser.parse_args()
    packages = [p for p in (ROOT / "dist").glob(f"*{options.platform}-{options.arch}*") if p.suffix != ".sha256" and (not options.sha or f"-{options.sha[:8]}" in p.name)]
    expected = 1 if options.platform == "macos" else 2
    if len(packages) != expected:
        raise RuntimeError(f"Expected {expected} packages, found {packages}")
    for package in packages:
        checksum = package.with_name(package.name + ".sha256").read_text().split()[0]
        if hashlib.sha256(package.read_bytes()).hexdigest() != checksum:
            raise RuntimeError(f"Checksum mismatch: {package}")
        if package.suffix == ".dmg":
            with tempfile.TemporaryDirectory() as directory:
                mount = Path(directory) / "PrintMarten"
                subprocess.run(["hdiutil", "attach", "-readonly", "-nobrowse", "-mountpoint", str(mount), str(package)], check=True)
                try:
                    app = mount / "PrintMarten.app"
                    info = plistlib.loads((app / "Contents/Info.plist").read_bytes())
                    assert info["CFBundleExecutable"] == "printmarten"
                    assert (mount / "Applications").is_symlink()
                    assert list((app / "Contents/Frameworks").glob("*heif*.dylib"))
                    subprocess.run(["codesign", "--verify", "--deep", "--strict", str(app)], check=True)
                    smoke(app / "Contents/MacOS/printmarten")
                finally:
                    subprocess.run(["hdiutil", "detach", str(mount)], check=True)
        elif package.suffix == ".zip":
            with tempfile.TemporaryDirectory() as directory:
                with zipfile.ZipFile(package) as archive:
                    archive.extractall(directory)
                binary = Path(directory) / "PrintMarten/printmarten.exe"
                # Inspect the PE machine type instead of relying on the archive name.
                data = binary.read_bytes()
                pe = struct.unpack_from("<I", data, 0x3C)[0]
                assert data[pe:pe+4] == b"PE\0\0" and struct.unpack_from("<H", data, pe+4)[0] == 0x8664
                assert list(binary.parent.glob("*heif*.dll"))
                assert not list(binary.parent.glob("vcruntime*.dll"))
                assert not list(binary.parent.glob("msvcp*.dll"))
                assert not list(binary.parent.glob("concrt*.dll"))
                smoke(binary)
        elif package.suffix == ".exe":
            subprocess.run([str(package), "/S"], check=True, timeout=120)
            installed = Path(os.environ["LOCALAPPDATA"]) / "Programs/PrintMarten"
            smoke(installed / "printmarten.exe")
            subprocess.run([str(installed / "Uninstall.exe"), "/S", f"_?={installed}"], check=True, timeout=120)
            assert not (installed / "printmarten.exe").exists()
        elif package.suffix == ".deb":
            subprocess.run(["dpkg-deb", "--info", str(package)], check=True)
        elif package.name.endswith(".tar.gz"):
            with tempfile.TemporaryDirectory() as directory:
                with tarfile.open(package) as archive:
                    archive.extractall(directory, filter="data")
                binary = Path(directory) / "PrintMarten/printmarten"
                data = binary.read_bytes()
                assert data[:4] == b"\x7fELF" and struct.unpack_from("<H", data, 18)[0] == 62
                assert list((binary.parent / "lib").glob("*heif*.so*"))
                subprocess.run(["ldd", str(binary)], check=True)
                smoke(binary)
        print(f"Validated {package.name}")


if __name__ == "__main__":
    main()
