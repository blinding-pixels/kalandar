# Security Policy

## Supported Versions

Until Kalandar reaches `1.0.0`, only the latest released version is supported for security fixes.

## Reporting A Vulnerability

If GitHub private vulnerability reporting is enabled for the repository, use GitHub's "Report a vulnerability" flow.

If private reporting is not available yet, open a GitHub issue with only a high-level description of the impact and request a private follow-up channel. Do not include exploit code, secrets, access tokens, private user data, or enough detail for someone else to reproduce the issue publicly.

Security pull requests are not accepted. Please report the issue first so it can be reviewed and fixed by the maintainer.

## Scope

Security reports may include:

- Incorrect parsing behavior that could create unsafe app state.
- Runtime artifact integrity or model-loading issues.
- Supply-chain, packaging, or release-process problems.
- Accidental inclusion of secrets or private data in a release artifact.

This project does not currently run a bug bounty program and does not promise a fixed response SLA.
