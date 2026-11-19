"""Package a compiled desktop application with documentation and dependency notices."""

import argparse
import hashlib
import json
from pathlib import Path
import plistlib
import shutil
import subprocess
import tarfile
import tempfile
import tomllib
import zipfile

ROOT = Path(__file__).resolve().parents[1]
TARGETS = {
    "x86_64-pc-windows-msvc": "windows-x86_64",
    "x86_64-unknown-linux-gnu": "linux-x86_64",
    "x86_64-apple-darwin": "macos-x86_64",
    "aarch64-apple-darwin": "macos-aarch64",
}


def dependency_notices(target, destination):
    metadata = json.loads(subprocess.check_output(
        ["cargo", "metadata", "--locked", "--format-version", "1", "--filter-platform", target],
        cwd=ROOT, text=True, encoding="utf-8",
    ))
    notices = ["Third-party dependency notices", "Inventory includes build dependencies."]
    inventory = []
    for package in sorted(metadata["packages"], key=lambda p: (p["name"], p["version"])):
        if package["id"] == metadata["resolve"]["root"]:
            continue
        directory = Path(package["manifest_path"]).parent
        inventory.append({key: package.get(key) for key in ("name", "version", "license", "repository", "source")})
        notices.extend(["", "=" * 72, f'{package["name"]} {package["version"]}',
                        f'License: {package.get("license") or "See license text"}',
                        f'Source: {package.get("repository") or package.get("source") or "Local dependency"}'])
        paths = set()
        if package.get("license_file"):
            paths.add(directory / package["license_file"])
        for path in directory.iterdir():
            if path.name.lower().startswith(("license", "licence", "copying", "notice", "copyright")):
                if path.is_dir():
                    paths.update(p for p in path.rglob("*") if p.is_file())
                elif path.is_file():
                    paths.add(path)
        # Embedded fonts and other assets can carry separate license terms.
        for folder in ("fonts", "assets"):
            asset_directory = directory / folder
            if asset_directory.is_dir():
                paths.update(p for p in asset_directory.rglob("*.txt") if p.is_file())
        for path in sorted(paths):
            notices.extend(["", f"--- {path.relative_to(directory)} ---", path.read_text(encoding="utf-8", errors="replace")])
    (destination / "THIRD_PARTY_LICENSES.txt").write_text("\n".join(notices) + "\n", encoding="utf-8")
    (destination / "dependency-licenses.json").write_text(json.dumps(inventory, indent=2) + "\n", encoding="utf-8")


def package(target, binary, output):
    version = tomllib.loads((ROOT / "Cargo.toml").read_text())["package"]["version"]
    if not binary.is_file():
        raise FileNotFoundError(binary)
    output.mkdir(parents=True, exist_ok=True)
    name = f"cadence-{version}-{TARGETS[target]}"
    with tempfile.TemporaryDirectory(prefix="cadence-package-") as temporary:
        stage = Path(temporary) / name
        stage.mkdir()
        if "apple" in target:
            bundle = stage / "Cadence.app" / "Contents"
            executable = bundle / "MacOS" / "cadence"
            resources = bundle / "Resources"
            resources.mkdir(parents=True)
            with (bundle / "Info.plist").open("wb") as stream:
                plistlib.dump({
                    "CFBundleName": "Cadence", "CFBundleDisplayName": "Cadence",
                    "CFBundleIdentifier": "org.marchingdrill.editor",
                    "CFBundleExecutable": "cadence", "CFBundlePackageType": "APPL",
                    "CFBundleShortVersionString": version.split("-")[0].split("+")[0],
                    "CFBundleVersion": version.split("-")[0].split("+")[0],
                    "NSHighResolutionCapable": True, "LSMinimumSystemVersion": "11.0",
                }, stream)
            shutil.copy2(ROOT / "assets/cadence-icon.svg", resources)
        else:
            executable = stage / ("cadence.exe" if "windows" in target else "cadence")
        executable.parent.mkdir(parents=True, exist_ok=True)
        shutil.copy2(binary, executable)
        executable.chmod(executable.stat().st_mode | 0o111)
        for file in ("README.md", "LICENSE", "NOTICE", "CHANGELOG.md"):
            shutil.copy2(ROOT / file, stage)
        shutil.copytree(ROOT / "docs", stage / "docs")
        shutil.copytree(ROOT / "assets", stage / "assets")
        dependency_notices(target, stage)
        if "linux" in target:
            shutil.copy2(ROOT / "assets/cadence-icon.svg", stage)
            shutil.copy2(ROOT / "packaging/cadence.desktop", stage)
            archive = output / f"{name}.tar.gz"
            with tarfile.open(archive, "w:gz") as stream:
                stream.add(stage, arcname=name)
        else:
            archive = output / f"{name}.zip"
            with zipfile.ZipFile(archive, "w", zipfile.ZIP_DEFLATED) as stream:
                for file in sorted(stage.rglob("*")):
                    if file.is_file():
                        stream.write(file, file.relative_to(stage.parent))
        digest = hashlib.sha256(archive.read_bytes()).hexdigest()
        archive.with_name(archive.name + ".sha256").write_text(f"{digest}  {archive.name}\n", encoding="utf-8")
        return archive


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--target", required=True, choices=TARGETS)
    parser.add_argument("--binary", type=Path, help="Override the compiled binary path")
    parser.add_argument("--output", type=Path, default=ROOT / "dist")
    arguments = parser.parse_args()
    suffix = ".exe" if "windows" in arguments.target else ""
    binary = arguments.binary or ROOT / "target" / arguments.target / "release" / f"cadence{suffix}"
    print(package(arguments.target, binary, arguments.output))
