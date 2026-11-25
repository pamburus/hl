# Specifications

The specifications under [`specs/`](../../specs) describe how hl behaves: one directory per feature, named `NNN-feature-name`, with the requirements, scenarios and edge cases in its `spec.md`. They are the reference for what hl is meant to do. To keep them that way, every change of behavior updates them, and every bug report is checked against them.

## Behavior changes

A change of behavior is anything a user can observe: output, accepted input, command-line options, configuration, environment variables, exit codes and error messages. New features, changes to existing features and bug fixes are all changes of behavior.

A pull request that changes behavior MUST leave the specifications describing the behavior after it, including the cases it fixes:

* If a spec covers the behavior, update its requirements, scenarios and edge cases to match the change. Do not leave a requirement that the change contradicts.
* If no spec covers the behavior, add it to the spec of the feature it belongs to. If there is no such spec, create one as described in [New specs](#new-specs). A new spec MAY cover only the part of the feature that the change touches; it grows with later changes.
* If a spec already describes the fixed behavior correctly, and the code did not follow it, the spec needs no change. Mention the requirement the fix restores in the pull request.

When a spec is amended, record the change in its header:

* Add an entry with the date of the change and the issue it comes from, as `2026-11-02 (issue #N: summary)`, to its `**Updated**` line, after the entries of earlier amendments. Without an issue, use the pull request, as `(PR #N: summary)`, once it is opened. A spec with a `**Last Updated:**` line instead gets the entry there, after the date it already has. A spec with neither gets an `**Updated**` line after its `**Created**` line, or at the end of its header if it has none.
* Add the request as an `**Amendment input**` line, after those of earlier amendments, or at the end of the header for the first one.
* If decisions were taken along the way, add them as a new session under `## Clarifications`, and create that section after the header if it does not exist.

[Field Key Display](../../specs/009-key-prettification/spec.md) shows how a bug fix amends an existing spec. With `#N` and `#M` standing for the issue numbers, a spec amended twice has:

```markdown
**Updated**: 2026-10-21 (issue #N: quoting of ambiguous keys), 2026-11-02 (issue #M: ...)
```

### New specs

Create a new spec on the current branch, with `/speckit-specify` or from [the template](../../.specify/templates/spec-template.md):

* Name its directory `NNN-feature-name`, where `NNN` is one above the highest number ever used on any branch of the hl repository, also by specs removed since, so that no number is reused. With `origin` being the hl repository (in a fork, replace each `origin` with the remote that points to it), `git fetch origin '+refs/heads/*:refs/remotes/origin/*' && git log --full-history --remotes='origin/*' --format= --name-only -- specs | grep -o '^specs/[0-9]*' | sort -uV | tail -1` prints that highest number, as `specs/NNN`; in a shallow clone, add `--unshallow` to the fetch. `/speckit-specify` counts only the local `specs/`, so check the number it picks.
* Put the directory name in its `**Feature Branch**` line.
* Check that no other open pull request adds a spec with the same number, because git does not report it as a conflict. `gh pr list --repo pamburus/hl --state open --limit 1000 --json number,files --jq '.[] | {number, specs: ([.files[].path | capture("^specs/(?<n>[0-9]+)").n] | unique)} | select(.specs != []) | "#\(.number): specs/\(.specs | join(", specs/"))"'` lists each open pull request that touches specs, with the numbers it uses. If another pull request uses yours, the pull request that merges later renumbers its spec and updates the references to it, as described in [the constitution](../../.specify/memory/constitution.md#vi-specification--cross-reference-integrity).

### Other rules

Add new requirements without renumbering existing ones, as [the constitution](../../.specify/memory/constitution.md#vi-specification--cross-reference-integrity) requires. Describe behavior that belongs to another feature in that feature's spec and link to it, as described in [Cross-Feature References](cross-feature-guidelines.md).

Changes that do not change behavior, such as refactoring, performance improvements, dependency updates, tests, CI and documentation outside the program, such as the README, need no spec changes, unless a spec states a requirement that they affect. Built-in help is output of the program, so a change of it is a change of behavior. Documentation that contradicts a spec is a bug, either of the documentation or of the spec.

A test of the changed behavior SHOULD name the requirement or scenario it checks, for example in a comment, so that the two can be found from each other.

## Bug reports

A behavior is a bug only when it differs from what it is meant to be. Before reporting a bug, or treating a failing test as a bug, find the spec that covers the behavior, in the specs of the version that shows it. The [specs on `master`](https://github.com/pamburus/hl/tree/master/specs) are those of the latest release; for another release, select its tag in the branch selector on GitHub. In a checkout, search them with `git grep -il '<keyword>' <tag-or-commit> -- 'specs/*/spec.md'`, or, for the specs as your working tree has them, with `git grep --untracked -il '<keyword>' -- 'specs/*/spec.md'`. Then compare:

* **The behavior contradicts the spec**: it is a bug. Name the requirement or scenario it violates in the report, as the expected behavior.
* **The behavior follows the spec**: it is not a bug, even if it looks wrong, unless the spec itself is defective: it contradicts itself or another spec, or requires data loss, a crash or a security problem. Report such a defect as a bug against the spec. If the documentation describes other behavior, report the contradiction as a bug, of the documentation or of the spec. Otherwise, request a change of the specified behavior as a [feature](../../CONTRIBUTING.md#requesting-features) instead, or start a [discussion](https://github.com/pamburus/hl/discussions). If the spec is sound, a test that expects something else is wrong, even if the documentation agrees with the test, unless the change it belongs to updates the spec.
* **No spec covers the behavior**: judge it by the documentation, such as the README and the built-in help, and by the behavior of related features. Say in the report that you found no spec that covers the behavior. The fix adds it to the specifications, as described above.

## Rationale

Behavior that is written down only in the code cannot be checked against anything: a bug and an intended choice look the same. Updating the specifications with every change keeps them an accurate description of hl, and checking reports against them separates bugs, which the code fixes, from requests to change agreed behavior, which the spec decides first.
