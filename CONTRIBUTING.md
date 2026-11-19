# Contributing

## Development environment

Install Rust through rustup and the platform build dependencies listed in [README.md](README.md). The repository specifies the compiler and components in `rust-toolchain.toml`.

Run these checks before submitting a pull request:

```sh
cargo fmt --all -- --check
cargo clippy --locked --all-targets -- -D warnings
cargo test --locked
cargo build --locked --release
```

Release packaging requires Python 3.11 or newer. `scripts/package.py` uses the Python standard library.

Validate archive layouts with `python3 -m unittest discover -s scripts -p 'test_*.py'`.

## Changes

Development takes place on `dev`. Create feature branches from the current `dev` branch and open pull requests back into `dev`. Use short branch names such as `feature/motion-preview` or `fix/subset-timing`.

```sh
git switch dev
git pull --ff-only origin dev
git switch -c feature/short-description
```

The `main` branch contains release candidates and published versions. Promote tested changes through a pull request from `dev` into `main`. Create release tags from the reviewed `main` commit. Avoid direct pushes to `main` after the initial import.

Require pull requests and passing build checks on `main` through GitHub branch protection. Enable protection after the initial `main` and `dev` branches exist. Repository administrators configure these settings in GitHub.

- Describe the problem, resulting behavior, and relevant verification.
- Keep drill coordinates and document compatibility independent of UI presentation.
- Add regression coverage for changes to matching, paths, timing, constraints, and persistence.
- Define UI colors, dimensions, and font sizes in `src/theme.rs`.
- Update the user guide and changelog when behavior changes.
- Preserve third-party copyright and license notices.

## Bug reports

Include the application version, operating system, processor architecture, reproduction steps, expected behavior, and observed behavior. Attach a minimal project file when possible. Remove personal information and copyrighted audio before uploading attachments.

## License

Contributions are provided under the Apache License, Version 2.0, unless an explicit alternative agreement applies.
