# Commits

## Validation

Before every commit, run `just ci`, which executes:

* `check`: Cargo check for compilation errors
* `test`: Full test suite execution
* `lint`: Clippy warnings resolution
* `audit`: Security vulnerability checks
* `fmt-check`: Code formatting verification
* `check-schema`: JSON schema validation
* `stress-test`: CPU time and memory growth checks on a release build, see [stress-test.py](../../scripts/README.md#stress-testpy)

All reported issues MUST be resolved before committing.

## Commits in the speckit workflow

After completing each speckit step, create a commit using the conventional commit format:

* After `/speckit-specify`: `docs(spec): add [feature-name] specification`
* After `/speckit-clarify`: `docs(spec): clarify [aspect] in [feature-name]`
* After `/speckit-tasks`: `docs(tasks): generate task breakdown for [feature-name]`
* After `/speckit-implement`: `feat([scope]): [user-visible change]` or `fix([scope]): [user-visible fix]`

## Commit message content

Commit messages are used to automatically generate release notes and MUST focus on user-visible behavior.

Write about:

* New features users can access
* Bug fixes that affect user experience
* Configuration options added or changed
* Command-line flags or arguments modified
* Output format changes
* Performance improvements users will notice
* Breaking changes requiring user action

Do not write about:

* Implementation details (functions, structs, modules)
* Source code references (file names, line numbers)
* Test statistics ("1464 tests passing")
* CI validation status ("Full CI validation passed")
* Internal refactoring unless it affects users
* Code quality metrics

Exception: internal changes with no user-visible impact may include brief technical context, but still avoid source code specifics. Focus on "what changed" in system behavior, not "how it was implemented."

Examples:

* ✅ Good: `feat(config): add prettify-field-keys option to control key prettification`
* ✅ Good: `fix(parser): handle timestamps with microsecond precision correctly`
* ❌ Bad: `feat(formatting): implement prettify_field_keys in RecordFormatter struct`
* ❌ Bad: `fix: update formatting.rs line 1211 to check prettify flag. All 1464 tests pass`

## Issue references

A commit that resolves an issue MUST end with a `Closes: #N` footer, one per resolved issue:

```text
fix(parser): handle timestamps with microsecond precision correctly

Closes: #N
```

This is a valid Conventional Commits footer and a GitHub closing keyword. Pull requests target `develop` rather than the default branch, so a closing keyword in a pull request description has no effect. The footer closes the issue when the release workflow fast-forwards `master` to a release that contains the commit, which is when the change becomes available to users.

Other footers, such as `Refs: #N` for related issues, MAY be added alongside it.

A commit that fixes a regression not released yet SHOULD have a `Regressed-by: #N` footer when the cause is known, where N is the number of the pull request that landed the commit introducing the regression, not the hash of that commit. The pull request description links that pull request as well:

```text
fix(parser): handle empty keys in dotted paths again

Regressed-by: #N
Closes: #M
```

The release notes then list the pull request of the fix on the line of pull request N instead of on a line of its own, because users of releases never saw the regression. If N is not part of the same release, the fix gets its own line as usual.

Because the fix gets no line of its own, a commit with a `Regressed-by` footer MUST contain only the fix of that regression. Other changes go to their own pull requests. A commit of another type than `fix` with the footer, which breaks this rule, keeps its own line in the release notes as well.

Until the release, the `resolved` label marks open issues that commits on a release line close. To keep it accurate:

* A revert MUST keep the lines that `git revert` writes about the reverted commit unchanged, also when the message is rewritten or squashed: `This reverts commit <hash>.`, or for a merge `This reverts commit <hash>, reversing changes made to <parent>.` Otherwise the reverted commit still counts as closing its issues.
* A backport MUST be made with `git cherry-pick -x` of the commit as it landed on the newer line, such as the squashed commit on `develop`, not a commit of the pull request branch, so that the original commit is recorded. Otherwise a backported revert of the original does not undo the backport, and the backport keeps the label on an issue that was reopened after the original was released.

## Rationale

This workflow ensures:

* Clean, atomic commits representing logical completion points
* All code passing quality gates before entering version control
* Clear commit history enabling easy navigation and rollback
* Prevention of broken commits that fail CI pipelines
* Release notes that are immediately useful to end users
