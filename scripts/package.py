"""Package an already-built release binary and the pinned native runtime."""
import argparse
import hashlib
import json
import os
from pathlib import Path
import plistlib
import re
import shutil
import subprocess
import sys
import tarfile
import zipfile
from PIL import Image

ROOT = Path(__file__).resolve().parents[1]


def run(*args):
    subprocess.run([str(a) for a in args], check=True)


def copy_libraries(native, destination, pattern):
    destination.mkdir(parents=True, exist_ok=True)
    copied = []
    for directory in [native / "lib", native / "bin"]:
        if directory.exists():
            for library in directory.glob(pattern):
                if library.is_file():
                    shutil.copy2(library, destination / library.name)
                    copied.append(destination / library.name)
    if not any("heif" in p.name for p in copied) or not any("de265" in p.name for p in copied):
        raise RuntimeError("Both libheif and libde265 must be included in the package")
    return copied


def notices(destination, native):
    destination.mkdir(parents=True, exist_ok=True)
    for name in ["LICENSE", "THIRD_PARTY_NOTICES.md", "README.md"]:
        shutil.copy2(ROOT / name, destination / name)
    shutil.copy2(ROOT / "assets/fonts/LICENSE", destination / "NotoSans-LICENSE.txt")
    shutil.copytree(native / "share/printmarten/licenses", destination / "native-licenses", dirs_exist_ok=True)
    shutil.copy2(native / "versions.json", destination / "native-versions.json")
    # cargo-about emits the actual license texts of the locked Rust dependencies.
    rust_notices = ROOT / "target/THIRD_PARTY_LICENSES.html"
    if not rust_notices.exists():
        raise RuntimeError("Generate target/THIRD_PARTY_LICENSES.html with cargo-about before packaging")
    shutil.copy2(rust_notices, destination / rust_notices.name)


def macos(stage, out, name, binary, native, icon, arch, version):
    volume = stage / "volume"
    app = volume / "PrintMarten.app"
    contents = app / "Contents"
    executable = contents / "MacOS/printmarten"
    executable.parent.mkdir(parents=True)
    shutil.copy2(binary, executable)
    executable.chmod(0o755)
    resources = contents / "Resources"
    resources.mkdir()
    icon.save(resources / "PrintMarten.icns", format="ICNS")
    notices(resources / "Documentation", native)
    libraries = copy_libraries(native, contents / "Frameworks", "*.dylib")
    # Resolve every bundled Mach-O dependency relative to its own directory.
    for library in libraries:
        run("install_name_tool", "-id", f"@loader_path/{library.name}", library)
        deps = subprocess.check_output(["otool", "-L", str(library)], text=True).splitlines()[1:]
        for line in deps:
            dep = line.strip().split(" (", 1)[0]
            if (contents / "Frameworks" / Path(dep).name).exists() and dep != f"@loader_path/{Path(dep).name}":
                run("install_name_tool", "-change", dep, f"@loader_path/{Path(dep).name}", library)
        run("codesign", "--force", "--sign", "-", library)
    info = {"CFBundleName": "PrintMarten", "CFBundleDisplayName": "PrintMarten",
            "CFBundleIdentifier": "io.github.pewu1.printmarten", "CFBundleExecutable": "printmarten",
            "CFBundlePackageType": "APPL", "CFBundleIconFile": "PrintMarten.icns",
            "CFBundleShortVersionString": version, "CFBundleVersion": version,
            "LSMinimumSystemVersion": "13.0", "NSHighResolutionCapable": True}
    (contents / "Info.plist").write_bytes(plistlib.dumps(info))
    # Ad-hoc signing permits local Apple Silicon execution; this is not notarization.
    run("codesign", "--force", "--sign", "-", app)
    (volume / "Applications").symlink_to("/Applications")
    run("hdiutil", "create", "-volname", "PrintMarten", "-srcfolder", volume,
        "-ov", "-format", "UDZO", out / f"{name}.dmg")
    run("codesign", "--verify", "--deep", "--strict", app)
    run("hdiutil", "verify", out / f"{name}.dmg")
    actual = subprocess.check_output(["lipo", "-archs", str(executable)], text=True).strip()
    expected = "x86_64" if arch == "x64" else "arm64"
    if actual != expected:
        raise RuntimeError(f"Wrong Mach-O architecture: {actual}")


def windows(stage, out, name, binary, native, icon, arch, version):
    portable = stage / "PrintMarten"
    portable.mkdir()
    shutil.copy2(binary, portable / "printmarten.exe")
    copy_libraries(native, portable, "*.dll")
    # MSVC runtime is installed separately as a system prerequisite. Do not
    # redistribute proprietary runtime DLLs inside the GPL application payload.
    notices(portable / "Documentation", native)
    icon.save(stage / "PrintMarten.ico", format="ICO", sizes=[(16,16),(32,32),(48,48),(64,64),(128,128),(256,256)])
    with zipfile.ZipFile(out / f"{name}-portable.zip", "w", zipfile.ZIP_DEFLATED) as archive:
        for path in sorted(portable.rglob("*")):
            if path.is_file():
                archive.write(path, path.relative_to(stage))
    nsis = shutil.which("makensis") or r"C:\Program Files (x86)\NSIS\makensis.exe"
    run(nsis, f"/DVERSION={version}", f"/DPAYLOAD={portable}",
        f"/DICON={stage / 'PrintMarten.ico'}", f"/DOUTPUT={out / (name + '-setup.exe')}",
        ROOT / "packaging/windows.nsi")


def linux(stage, out, name, binary, native, icon, arch, version):
    root = stage / "deb"
    app = root / "opt/printmarten"
    app.mkdir(parents=True)
    shutil.copy2(binary, app / "printmarten")
    (app / "printmarten").chmod(0o755)
    copy_libraries(native, app / "lib", "*.so*")
    notices(app / "Documentation", native)
    desktop = root / "usr/share/applications"
    desktop.mkdir(parents=True)
    shutil.copy2(ROOT / "packaging/printmarten.desktop", desktop)
    icons = root / "usr/share/icons/hicolor/256x256/apps"
    icons.mkdir(parents=True)
    icon.resize((256,256), Image.Resampling.LANCZOS).save(icons / "printmarten.png")
    control = root / "DEBIAN"
    control.mkdir()
    (control / "control").write_text(
        f"Package: printmarten\nVersion: {version}\nArchitecture: amd64\n"
        "Maintainer: PrintMarten contributors\nSection: graphics\nPriority: optional\n"
        "Depends: libc6 (>= 2.39), libgcc-s1, libstdc++6, libgtk-4-1 (>= 4.14), "
        "libgl1, libegl1, libxkbcommon0, libxkbcommon-x11-0, libwayland-client0, libx11-6, libxcb1, "
        "xdg-desktop-portal, xdg-desktop-portal-gtk\n"
        "Description: Print photos at exact sizes on automatically packed sheets\n")
    run("dpkg-deb", "--root-owner-group", "--build", root, out / f"{name}.deb")
    shutil.copy2(icons / "printmarten.png", app)
    (app / "printmarten.desktop").write_text((ROOT / "packaging/printmarten.desktop").read_text().replace("Exec=/opt/printmarten/printmarten", "Exec=printmarten"))
    with tarfile.open(out / f"{name}.tar.gz", "w:gz") as archive:
        archive.add(app, arcname="PrintMarten")


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--platform", choices=["macos","windows","linux"], required=True)
    parser.add_argument("--arch", choices=["x64","arm64"], required=True)
    parser.add_argument("--sha", required=True)
    parser.add_argument("--binary", type=Path)
    parser.add_argument("--native", type=Path, default=ROOT / "target/native")
    options = parser.parse_args()
    version = re.search(r'^version\s*=\s*"([^"]+)"', (ROOT / "Cargo.toml").read_text(), re.M)[1]
    name = f"PrintMarten-{version}-{options.platform}-{options.arch}-{options.sha[:8]}"
    out = ROOT / "dist"
    out.mkdir(exist_ok=True)
    stage = ROOT / "target/package" / name
    if stage.exists():
        shutil.rmtree(stage)
    stage.mkdir(parents=True)
    binary = options.binary or ROOT / "target/release" / ("printmarten.exe" if options.platform == "windows" else "printmarten")
    icon = Image.open(ROOT / "icon.png").convert("RGBA").resize((1024,1024), Image.Resampling.LANCZOS)
    globals()[options.platform](stage, out, name, binary.resolve(), options.native.resolve(), icon, options.arch, version)
    for path in sorted(out.glob(name + "*")):
        if path.suffix != ".sha256":
            digest = hashlib.sha256(path.read_bytes()).hexdigest()
            path.with_name(path.name + ".sha256").write_text(f"{digest}  {path.name}\n")


if __name__ == "__main__":
    main()
