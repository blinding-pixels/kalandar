# Kalandar Parity Fixtures

This directory is the retained handoff point for the Rust parser parity fixtures generated during the Kalandar port.

- `cases.json` is the checked-in input inventory.
- `manifest.json` records the frozen fixture corpus metadata.
- Family subdirectories contain normalized oracle outputs captured before the vendored TypeScript parser source was removed.

The TypeScript oracle source is no longer checked into this repository. The live parser path is the Rust implementation plus the bundled desktop parser artifacts.
