"""Build the small, pinned, decode-only HEIC runtime. No system installation."""
import argparse
import hashlib
import json
import os
from pathlib import Path
import shutil
import subprocess
import sys
import tarfile
import urllib.request
import cmake as cmake_package
from native_source_fixes import apply_libde265_fixes

ROOT = Path(__file__).resolve().parents[1]
SOURCES = [
    ("libde265", "1.0.15", "d4e55706dfc5b2c5c9702940b675ce2d3e7511025c6894eaddcdbaf0b15fd3f3"),
    ("libheif", "1.23.1", "0b14d6bdf5680488e3aede354b1e11be1444b3fc4a30fcf2ae06bd6b601466be"),
]


def run(*args):
    subprocess.run([str(a) for a in args], check=True)


def fetch(name, version, digest, downloads):
    archive = downloads / f"{name}-{version}.tar.gz"
    if not archive.exists():
        url = f"https://codeload.github.com/strukturag/{name}/tar.gz/refs/tags/v{version}"
        print(f"Downloading {url}", flush=True)
        with urllib.request.urlopen(url, timeout=90) as response:
            archive.write_bytes(response.read())
    if hashlib.sha256(archive.read_bytes()).hexdigest() != digest:
        raise RuntimeError(f"Source checksum mismatch: {archive}")
    source = downloads / f"{name}-{version}"
    if not source.exists():
        with tarfile.open(archive) as tar:
            for member in tar.getmembers():
                destination = (downloads / member.name).resolve()
                if downloads.resolve() not in destination.parents or member.issym() or member.islnk():
                    raise RuntimeError(f"Unsafe source archive entry: {member.name}")
            tar.extractall(downloads)
    return source


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--prefix", type=Path, default=ROOT / "target/native")
    parser.add_argument("--downloads", type=Path, default=ROOT / "target/native-sources")
    options = parser.parse_args()
    prefix = options.prefix.resolve()
    downloads = options.downloads.resolve()
    downloads.mkdir(parents=True, exist_ok=True)
    prefix.mkdir(parents=True, exist_ok=True)
    licenses = prefix / "share/printmarten/licenses"
    licenses.mkdir(parents=True, exist_ok=True)
    cmake = Path(cmake_package.CMAKE_BIN_DIR) / ("cmake.exe" if sys.platform == "win32" else "cmake")
    if not cmake:
        raise RuntimeError("Install scripts/requirements.txt first")
    for name, version, digest in SOURCES:
        source = fetch(name, version, digest, downloads)
        if name == "libde265":
            apply_libde265_fixes(source)
        build = downloads / f"build-{name}"
        common = [f"-DCMAKE_INSTALL_PREFIX={prefix}", "-DCMAKE_INSTALL_LIBDIR=lib",
                  f"-DCMAKE_PREFIX_PATH={prefix}", "-DCMAKE_BUILD_TYPE=Release",
                  "-DBUILD_SHARED_LIBS=ON"]
        if sys.platform == "darwin":
            common += ["-DCMAKE_OSX_DEPLOYMENT_TARGET=13.0", "-DCMAKE_INSTALL_NAME_DIR=@rpath", "-DCMAKE_INSTALL_RPATH=@loader_path"]
        elif sys.platform == "win32":
            common += ["-DCMAKE_WINDOWS_EXPORT_ALL_SYMBOLS=ON"]
        else:
            common += ["-DCMAKE_INSTALL_RPATH=$ORIGIN", "-DCMAKE_BUILD_WITH_INSTALL_RPATH=ON"]
        if name == "libde265":
            flags = ["-DENABLE_SDL=OFF", "-DENABLE_DECODER=OFF", "-DENABLE_ENCODER=OFF"]
        else:
            # One built-in decoder; no plugin discovery, encoders, or optional codecs.
            flags = ["-DENABLE_PLUGIN_LOADING=OFF", "-DWITH_LIBDE265=ON",
                     "-DBUILD_TESTING=OFF",
                     "-DWITH_LIBDE265_PLUGIN=OFF", "-DWITH_EXAMPLES=OFF", "-DWITH_GDK_PIXBUF=OFF",
                     "-DWITH_EXAMPLE_HEIF_THUMB=OFF", "-DWITH_EXAMPLE_HEIF_VIEW=OFF",
                     "-DBUILD_DOCUMENTATION=OFF", "-DWITH_UNCOMPRESSED_CODEC=OFF",
                     "-DWITH_HEADER_COMPRESSION=OFF", "-DWITH_LIBSHARPYUV=OFF"]
            for codec in ["AOM_DECODER", "AOM_ENCODER", "DAV1D", "RAV1E", "SvtEnc", "X264", "X265",
                          "JPEG_DECODER", "JPEG_ENCODER", "KVAZAAR", "OPENJPH_ENCODER",
                          "OpenJPEG_DECODER", "OpenJPEG_ENCODER", "FFMPEG_DECODER",
                          "OpenH264_DECODER", "UVG266", "VVDEC", "VVENC"]:
                flags.append(f"-DWITH_{codec}=OFF")
        # CMake caches absolute source/build paths and compiler locations. CI may
        # restore them under a different checkout path or runner image.
        run(cmake, "--fresh", "-S", source, "-B", build, *common, *flags)
        run(cmake, "--build", build, "--config", "Release", "--parallel", "3")
        run(cmake, "--install", build, "--config", "Release")
        for candidate in ["COPYING", "COPYING.LESSER", "LICENSE"]:
            license_file = source / candidate
            if license_file.exists():
                shutil.copy2(license_file, licenses / f"{name}-{candidate}.txt")
    (prefix / "versions.json").write_text(json.dumps({n: v for n, v, _ in SOURCES}, indent=2))
    print(f"HEIC runtime installed in {prefix}")


if __name__ == "__main__":
    main()
