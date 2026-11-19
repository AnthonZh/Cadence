# Release Procedure

## Pipeline

The `Build and release` GitHub Actions workflow runs on pushes, pull requests, and manual dispatch. A formatting job gates four native build jobs:

| Runner | Target | Output |
| --- | --- | --- |
| Ubuntu 22.04 | `x86_64-unknown-linux-gnu` | `.tar.gz` |
| Windows Server 2022 | `x86_64-pc-windows-msvc` | `.zip` |
| macOS 15, Intel | `x86_64-apple-darwin` | `.zip` containing an `.app` bundle |
| macOS 15, Apple Silicon | `aarch64-apple-darwin` | `.zip` containing an `.app` bundle |

Each build runs Clippy and tests, compiles the release executable, and packages documentation, the Apache license, project notices, dependency license metadata, and available dependency license texts. Each archive has a SHA-256 checksum. Actions retains build artifacts for 14 days.

A pushed `v*` tag enables the release job after all build jobs pass. The tag must match the package version exactly, including any prerelease suffix. The release job verifies the checksums and creates a **draft** GitHub Release with the archives attached. A tag containing a hyphen marks the draft as a prerelease. Manual dispatch creates build artifacts without publishing a release.

Repository contents access is read-only except in the release job. GitHub supplies `GITHUB_TOKEN`; no additional release token is required. Repository or organization policies must permit GitHub Actions and the release job's contents-write permission.

## Local packaging

Compile for the intended target, then run:

```sh
cargo build --locked --release --target x86_64-unknown-linux-gnu
python3 scripts/package.py --target x86_64-unknown-linux-gnu
```

The script writes archives and checksums to `dist/`. Use `--binary` to specify another compiled executable and `--output` to select an output directory. Package each target on its native runner; the script does not cross-compile or validate a binary's architecture. Cargo metadata may download target-specific dependency sources to collect license notices.

The Linux archive includes an optional desktop entry and SVG icon. Installing the entry requires placing the executable on `PATH` and the icon in the desktop icon search path. The macOS bundle includes `Info.plist` and a native executable; application signing and notarization require separate credentials and configuration.

## Release checklist

1. Update `Cargo.toml` and the matching entry in `CHANGELOG.md` on `dev`. Open a release pull request from `dev` to `main` and merge after review and passing checks.
2. Run the development checks and confirm all four CI targets pass.
3. Inspect dependency notices and any dependency-specific distribution requirements.
4. Check out the reviewed `main` commit and create a matching tag:

   ```sh
   git switch main
   git pull --ff-only origin main
   git tag -a v0.1.0 -m "Cadence 0.1.0"
   git push origin v0.1.0
   ```

5. Download each draft release asset and verify its checksum.
6. Test launch, file dialogs, save/reopen, unsaved-change prompts, playback, music, PNG export, and PDF packets on the corresponding operating system. Check PNG labels and executable permissions after extraction.
7. Review the release notes, unsigned-build disclosure, architecture labels, and known limitations.
8. Publish the reviewed draft through GitHub Releases.

The workflow does not sign binaries, notarize macOS applications, build installers, publish to package registries, or install updates on user systems. Public distribution of signed applications requires platform signing credentials and an additional signing stage.
