# Branches and Versioning

## Branches

| Branch | Contents | Updated by |
| --- | --- | --- |
| `develop` | The next release, as `X.Y.Z-alpha.N` | Pull requests |
| `release/X.Y` | Patch releases of a line that `develop` has moved past | Pull requests with patch-level changes only |
| feature branches | Larger work that lands in several pull requests | Pull requests |
| `master` | Exactly the latest stable release | The release workflow only |

A release line is `develop` or a `release/*` branch.

The `master` branch stays the default branch on GitHub, so that tools reading the default branch, such as `cargo binstall --git`, see the latest release. It does not accept pull requests. The release workflow fast-forwards it to each stable release that is newer than the current one. Pull requests opened against it are moved to `develop` automatically.

GitHub reads some files only from the default branch: `.github/dependabot.yml`, scheduled workflows, and workflows triggered by `pull_request_target` for pull requests based on `master`. Changes to them take effect when the next release reaches `master`.

## Releases

Run the release workflow on `develop`, or on a `release/*` branch for a patch release. It sets the release version, builds and uploads the binaries, and merges the release commit into the branch it ran on. For a stable release that is newer than the current latest one, it also moves the `latest` tag and fast-forwards `master` to the release commit.

## Fixes for released lines

Make a fix on the oldest supported line that needs it, then merge it forward into each newer `release/*` branch and finally into `develop`. Cherry-picks are only for backports, where a change made on a newer line is needed on an older one.

A forward merge:

* MUST land as a merge commit, never squashed or rebased. Otherwise the target loses the ancestry, the same conflicts come back on the next forward merge, and `master` can no longer be fast-forwarded.
* MUST be resolved on a separate branch made from the target, for example `git switch -c merge/release-0.36-into-develop origin/develop && git merge origin/release/0.36`. Do not use the "Resolve conflicts" button on a pull request whose head is the source branch, because it merges the target into the source.
* MUST bump the target's version as described in [Forward merges](#forward-merges) below.
* For `nix/binary-hashes.nix`, keeps the target's side on conflict. The file changes only in releases, so a conflict means an older line was released after the target's release.

## Versioning of pull requests

The version identifies a state of a release line, not a working branch.

* Every pull request targeting a release line MUST bump the version according to the SemVer impact of its change. The impact is measured against the last release tag on that line.
* Pull requests targeting a feature branch MUST NOT change the version. Feature branches keep the version of the line they were branched from, and pick up its bumps by rebasing or merging as usual. The pull request that merges a feature branch into a release line follows the steps below like any other.
* `release/*` branches MUST accept only patch-level changes: fixes and changes with no SemVer impact.

Each such pull request MUST set the version as follows, after rebasing onto the latest target branch:

1. Run `git fetch -t && just previous-tag` to get the last release tag on the line (e.g., `v0.36.4`). Run `just version` to get the current version (e.g., `0.36.5-alpha.3`).
2. Determine the next release the line now requires, from the highest-impact change since that tag, including this pull request:
   * breaking change → next major (`(major+1).0.0`). While on 0.x, a breaking change bumps the minor version instead (`0.(minor+1).0`).
   * new feature → next minor (`major.(minor+1).0`)
   * fixes only → next patch (`major.minor.(patch+1)`)
3. If the current version is already a pre-release of that release, or of a higher one, run `just bump` to increment its alpha (e.g., `0.37.0-alpha.1` → `0.37.0-alpha.2`). Otherwise run `cargo set-version -p hl <next release>-alpha.1` (e.g., `cargo set-version -p hl 0.37.0-alpha.1`).
4. Changes with no SemVer impact (docs, chores, CI, refactoring, dependency updates without behavior changes) MUST still run `just bump`, so every merged state of the line has a unique version.

Example: after `v0.36.4`, the first fix sets `0.36.5-alpha.1`, a later feature sets `0.37.0-alpha.1`, and a fix after that sets `0.37.0-alpha.2`.

### Forward merges

A forward merge brings only patch-level changes, so the target's version follows from its current state:

| Target version | New target version | Example |
| --- | --- | --- |
| A pre-release (`X.Y.Z-alpha.N`) | `X.Y.Z-alpha.(N+1)` (`just bump`) | `develop` at `0.37.0-alpha.3` → `0.37.0-alpha.4` |
| A release (`X.Y.Z`) | `X.Y.(Z+1)-alpha.1` | `release/0.36` at `0.36.4` → `0.36.5-alpha.1` |

The conflict on the version line is resolved this way, not by taking either side.

### Rationale

Versions identify releases of a single line. Deriving the bump from the change's SemVer impact keeps the pre-release version an accurate preview of the next release. Assigning it only on the release line keeps every version unique and in order. If parallel feature branches each claimed the next minor alpha, they would produce identical pre-release versions with different contents, which violates SemVer's rule that a released version's contents never change, and early bumps would conflict with every bump on the line.
