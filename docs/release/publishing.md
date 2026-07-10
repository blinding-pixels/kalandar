# Publishing Kalandar

This page describes how to create a Git release for private JavaScript and SwiftPM consumers. npm registry and crates.io publication are disabled for this repository.

## Before You Begin

For a Git release, you need:

- Git repository access to `blinding-pixels/kalandar`.
- Git credentials for consumers that install the private repository.
- A clean release decision on the version number.

## Verify Everything

Run the release check:

```sh
npm run release:check
```

This command runs TypeScript checks, builds the package, runs Swift tests, runs Rust tests, checks Swift app-output parity, and performs an npm pack dry run. It does not validate separately distributed Core AI artifacts.

Before tagging a model-backed release, validate each Core AI artifact suite with the Python Core AI runtime and compare optimized model outputs against the approved parser baseline.

## Publish A Git Release

1. Commit the reviewed source and documentation changes.
2. Create a semantic version tag that includes the model-backed Swift implementation.
3. Push the commit and tag to the private repository.
4. Configure consumers to use the tag through npm Git dependencies or Swift Package Manager.

SwiftPM consumers can resolve a private tag without a Swift package registry or Swift Package Index entry.

## Publish Swift With Git Tags

Swift Package Manager commonly consumes packages from Git repositories.

To publish a Swift package release:

1. Ensure the root `Package.swift` builds.
2. Commit the release.
3. Create a semantic version tag such as `0.1.2`.
4. Push the tag to the private repository.
5. Configure the consuming app's Git credentials for repository access.

Swift package registries and the Swift Package Index are not needed. SwiftPM consumes the private Git tag directly.

## What's Next

- Run the full [Release checklist](./checklist.md).
- Review package contents in [Package layout](../reference/package-layout.md).
