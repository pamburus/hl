# Commits

## Validation

Before every commit, run `just ci`, which executes:

* `check`: Cargo check for compilation errors
* `test`: Full test suite execution
* `lint`: Clippy warnings resolution
* `audit`: Security vulnerability checks
* `fmt-check`: Code formatting verification
* `check-schema`: JSON schema validation

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

Closes: #1234
```

This is a valid Conventional Commits footer and a GitHub closing keyword. Pull requests target `develop` rather than the default branch, so a closing keyword in a pull request description has no effect. The footer closes the issue when the release workflow fast-forwards `master` to a release that contains the commit, which is when the change becomes available to users.

Other footers, such as `Refs: #N` for related issues, MAY be added alongside it.

## Rationale

This workflow ensures:

* Clean, atomic commits representing logical completion points
* All code passing quality gates before entering version control
* Clear commit history enabling easy navigation and rollback
* Prevention of broken commits that fail CI pipelines
* Release notes that are immediately useful to end users
