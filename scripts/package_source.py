"""Package corresponding application, Rust-crate, and native-codec sources."""
import argparse
import ast
import hashlib
import json
from pathlib import Path
import shutil
import subprocess
import tarfile
import tempfile
import re

ROOT = Path(__file__).resolve().parents[1]
# Explicit source inputs keep caches, credentials, and build outputs out of releases.
INPUTS = ["Cargo.toml", "Cargo.lock", "rust-toolchain.toml", "build.rs", "LICENSE",
          "README.md", "THIRD_PARTY_NOTICES.md", "about.toml", "icon.png", "icon.jpg",
          "src", "assets", "docs", "packaging", "scripts", ".github"]


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--sha", required=True)
    parser.add_argument("--offline", action="store_true", help="Use cached Rust crates only")
    options = parser.parse_args()
    version = re.search(r'^version\s*=\s*"([^"]+)"', (ROOT / "Cargo.toml").read_text(), re.M)[1]
    name = f"PrintMarten-{version}-source-{options.sha[:8]}"
    output = ROOT / "dist"
    output.mkdir(exist_ok=True)
    (ROOT / "target").mkdir(exist_ok=True)
    with tempfile.TemporaryDirectory(prefix="source-", dir=ROOT / "target") as temporary:
        source = Path(temporary) / name
        source.mkdir()
        for item in INPUTS:
            path = ROOT / item
            if path.is_dir():
                shutil.copytree(path, source / item, ignore=shutil.ignore_patterns("__pycache__", "*.pyc"))
            else:
                shutil.copy2(path, source / item)
        command = ["cargo", "vendor", "--locked", "--versioned-dirs"]
        if options.offline:
            command.append("--offline")
        command.append(str(source / "vendor"))
        subprocess.run(command, cwd=ROOT, check=True, stdout=subprocess.DEVNULL)
        config = source / ".cargo"
        config.mkdir()
        (config / "config.toml").write_text(
            '[source.crates-io]\nreplace-with = "vendored-sources"\n\n'
            '[source.vendored-sources]\ndirectory = "vendor"\n')
        # Resolve the entire graph from the archive's vendor configuration before
        # publishing anything; no compiler or target platform SDK is needed here.
        metadata = json.loads(subprocess.check_output(
            ["cargo", "metadata", "--locked", "--offline", "--format-version", "1"], cwd=source))
        for package in metadata["packages"]:
            manifest = Path(package["manifest_path"])
            if package["source"] and source / "vendor" not in manifest.parents:
                raise RuntimeError(f"Dependency was not vendored: {package['name']}")
        native = source / "target/native-sources"
        native.mkdir(parents=True)
        build_script = ast.parse((ROOT / "scripts/build_native.py").read_text())
        native_sources = next(ast.literal_eval(node.value) for node in build_script.body
                              if isinstance(node, ast.Assign)
                              and any(isinstance(target, ast.Name) and target.id == "SOURCES"
                                      for target in node.targets))
        for name_, version_, expected_digest in native_sources:
            archive = ROOT / f"target/native-sources/{name_}-{version_}.tar.gz"
            if hashlib.sha256(archive.read_bytes()).hexdigest() != expected_digest:
                raise RuntimeError(f"Native source checksum mismatch: {archive.name}")
            shutil.copy2(archive, native / archive.name)
        (source / "SOURCE.md").write_text(
            f"# Corresponding source for PrintMarten\n\nBuild revision: {options.sha}\n\n"
            "This archive includes the application, locked Rust crate sources, native codec\n"
            "source archives, asset notices, and build/install scripts. Read docs/development.md.\n"
            "With the documented Rust toolchain and platform SDK installed, Rust dependencies\n"
            "resolve from vendor/; cargo build --release --locked --offline needs no crate\n"
            "downloads. scripts/build_native.py uses the included target/native-sources archives.\n"
            "General-purpose compilers, SDKs, Python build tools, and OS system libraries are\n"
            "separate prerequisites. No proprietary Microsoft runtime DLLs are included.\n")
        archive = output / f"{name}.tar.gz"
        with tarfile.open(archive, "w:gz") as tar:
            tar.add(source, arcname=name)
        digest = hashlib.sha256(archive.read_bytes()).hexdigest()
        archive.with_name(archive.name + ".sha256").write_text(f"{digest}  {archive.name}\n")
        print(f"Created {archive.name}")


if __name__ == "__main__":
    main()
