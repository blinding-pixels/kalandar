# Publishing Kalandar

This page describes how to verify the release surfaces without publishing, then publish when credentials and tags are ready.

## Before You Begin

You need:

- npm account access for the `kalandar` package.
- crates.io account access for the `kalandar` crate.
- Git repository access for Swift Package Manager tag publishing.
- A clean release decision on the version number.

Do not run the final publish commands until the version, changelog, and account credentials are ready.

## Verify Everything

Run the release check:

```sh
npm run release:check
```

This command runs TypeScript checks, builds the package, runs Swift tests, runs Rust tests, checks Swift app-output parity, and performs an npm pack dry run.

## Verify npm Publishing

Run:

```sh
npm publish --dry-run --access public
```

This command prints the tarball contents and checks the package metadata. It does not publish.

When ready to publish:

```sh
npm publish --access public
```

## Verify crates.io Publishing

Run:

```sh
cargo publish --dry-run --manifest-path rust/kalandar/Cargo.toml
```

This packages and verifies the Rust crate. It does not upload a real release.

When ready to publish:

```sh
cargo publish --manifest-path rust/kalandar/Cargo.toml
```

## Publish Swift

Swift Package Manager commonly consumes packages from Git repositories.

To publish a Swift package release:

1. Ensure the root `Package.swift` builds.
2. Commit the release.
3. Create a semantic version tag such as `0.1.1`.
4. Push the tag to the public repository.
5. Add the repository to Swift Package Index after the public tag exists.

Swift package registries are optional. Use `swift package-registry publish` only if you choose to publish to a registry that implements the Swift package registry service.

## What's Next

- Run the full [Release checklist](./checklist.md).
- Review package contents in [Package layout](../reference/package-layout.md).
