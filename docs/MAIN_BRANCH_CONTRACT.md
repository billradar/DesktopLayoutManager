# Main Branch Contract

This contract applies to every change proposed for `main`.

## Required flow

1. Work on a topic branch. Do not develop directly on `main`.
2. Submit changes through a pull request targeting `main`.
3. Wait for the required CI and Main Branch Contract checks to pass before merging.
4. Review the diff and confirm the PR contains only the intended changes.
5. Do not force-push or rewrite `main`.

## Required checks

- **Native Win32 CI**: Rust library tests and Windows x64 release build must pass.
- **Main Branch Contract**: the PR must target `main`, originate from a branch other than `main`, and include this contract file.
- **Release contract**: GitHub Releases attach only `DesktopLayoutManager.exe` and `SHA256SUMS.txt`. GitHub-generated source archives are left enabled; no custom application ZIP or DLL is uploaded.

## Failure policy

A failed required check blocks the PR from being considered ready to merge. Fix the cause and rerun checks; do not bypass the contract to get a green result.

## Repository setting required

For GitHub to block direct pushes and enforce required checks at the server level, configure a branch protection rule or repository ruleset for `main`:

- Require a pull request before merging.
- Require the `Native Win32 build / windows-x64` and `Main Branch Contract / contract` checks.
- Require branches to be up to date before merging (recommended).
- Block force pushes and branch deletion.

Workflow files can report contract violations, but cannot by themselves prevent a repository administrator or a token with bypass permissions from pushing directly to `main`. This contract is kept on the feature branch until reviewed and merged.
