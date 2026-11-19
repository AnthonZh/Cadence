# Cadence

Cadence is a desktop application for designing 2D marching band drill. It provides a football-field workspace, formation guides, performer matching, transition playback, music synchronization, and printable drill packets.

Version 0.1.0 is a preview release. See [known limitations](#known-limitations) before production use.

## Features

- Performer placement, instrument assignments, multi-selection, copy/paste, and undo/redo.
- Lines, circles, arcs, splines, grids, polygons, and compound formations.
- Set and subset editing with exact counts, holds, staggered starts, gates, floats, and follow-the-leader movement.
- Compass proposals with travel balancing, optional collision avoidance, and symmetry.
- Persistent guide bindings, fixed anchors, equal intervals, and concentric circles.
- Music playback with BPM and audio offset settings.
- PNG charts and multi-page PDF packets per performer.
- Configurable keyboard shortcuts and versioned `.mdrill` documents.

## Downloads and installation

Download the archive matching the operating system and processor from [GitHub Releases](https://github.com/AnthonZh/Cadence/releases). Extract the complete archive before launching.

| Platform | Archive | Launch |
| --- | --- | --- |
| Windows x64 | `windows-x86_64.zip` | `cadence.exe` |
| Linux x64 | `linux-x86_64.tar.gz` | `./cadence` |
| macOS, Apple Silicon | `macos-aarch64.zip` | `Cadence.app` |
| macOS, Intel | `macos-x86_64.zip` | `Cadence.app` |

Actual archive names include the version. Windows and macOS downloads are unsigned; macOS releases are not notarized. Operating system security settings may prevent launch. No installer, automatic update service, or bundled music utility is included.

Linux builds target Ubuntu 22.04 and later compatible distributions, with glibc 2.35 or newer, an X11 or Wayland desktop, and a Vulkan-capable graphics driver. File selection requires `kdialog` or `zenity`; music playback requires `ffplay` from FFmpeg. PNG labels require a supported system font, such as DejaVu Sans.

Windows builds target Windows 10/11 x64. macOS bundles target macOS 11 or newer. Native runtime testing on each supported operating system remains part of the release checklist.

Each archive has a SHA-256 checksum file:

```sh
sha256sum --check cadence-<version>-linux-x86_64.tar.gz.sha256
```

On macOS, use `shasum -a 256 <archive>`. On Windows, use `Get-FileHash <archive> -Algorithm SHA256` and compare the result with the checksum file.

## Getting started

1. Launch the application and select **New Drill**.
2. Place performers with the toolbar or **P**, then assign instruments in the inspector.
3. Add a set, position performers, and set the transition count.
4. Scrub the timeline or press **Space** to preview.
5. Save the project as `.mdrill`. Export individual PDF packets through **File → Export Performer Packets…**.

Use **Ctrl/Cmd+T** to open Tool Properties for transforms, timing, paths, and constraints. See the [user guide](docs/USER_GUIDE.md) for detailed controls.

## Build from source

Install Rust through [rustup](https://rustup.rs/). The repository pins the compiler in `rust-toolchain.toml`.

On Debian or Ubuntu, install build dependencies:

```sh
sudo apt-get install pkg-config libxkbcommon-x11-dev libwayland-dev libx11-dev libxi-dev libxcursor-dev libxrandr-dev libgl1-mesa-dev
cargo build --locked --release
cargo run --locked --release
```

Windows requires the MSVC Rust toolchain and Visual Studio C++ Build Tools. macOS requires the Xcode Command Line Tools. Release compilation uses the checked-in `Cargo.lock`.

## Project files and preferences

Projects use the versioned `.mdrill` format. Legacy JSON projects open as imports. Music remains an external file reference; moving a project between computers may require reattaching the track.

Cadence retains compatibility with documents created under the former Marching Drill name. The application copies existing hotkey preferences and recent-project lists into the Cadence configuration directory on first use.

| Platform | Preferences directory |
| --- | --- |
| Windows | `%APPDATA%\Cadence` |
| macOS | `~/Library/Application Support/Cadence` |
| Linux | `$XDG_CONFIG_HOME/Cadence`, or `~/.config/Cadence` |

## Known limitations

- Collision avoidance applies to compass proposals. Generated gates, floats, rotations, and follow-the-leader moves require separate clearance review.
- Sketch constraints apply to editable keyframes, rather than every playback frame.
- Music uses constant BPM; tempo maps and time signatures are not implemented.
- The field uses fixed college-football dimensions and hashes.
- PNG exports and formation prefab files use the working directory.
- Unsigned builds require platform testing before public release.

## Development and releases

[CONTRIBUTING.md](CONTRIBUTING.md) describes local checks and issue reports. [RELEASING.md](docs/RELEASING.md) documents build artifacts, version tags, and publication. Changes appear in [CHANGELOG.md](CHANGELOG.md).

## License

Licensed under the [Apache License, Version 2.0](LICENSE). Third-party dependencies retain their own licenses; binary archives include dependency notices and license texts.

Cadence is an independent project with no affiliation with Pyware or Altium.
