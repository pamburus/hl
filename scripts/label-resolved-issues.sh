#!/usr/bin/env bash
#
# Adds a label to the open issues that commits on release lines close, until the commits reach master, and removes it
# from issues whose closing commits have all been reverted.
#
# Usage: label-resolved-issues.sh <owner/repo>
#
# Environment:
#   LABEL     label to manage (default: resolved)
#   REOPENED  number of a reopened issue, from which the label is removed unless unreleased commits still close it
#   DRY_RUN   print the changes instead of making them when set to 1
#
# Release lines are read from the remote-tracking branches of `origin`, so run `git fetch --prune origin` first.
#
# Reverts and backports are recognized by the lines that `git revert` and `git cherry-pick -x` add to the message, as
# required by docs/guidelines/commits.md.
#
# The label stays on an issue whose closing commit leaves a release line without a revert, for example when the branch
# is deleted or force-pushed, because the commit is no longer seen. Remove the label by hand in that case.
#
# A revert undoes the commits it names and the backports of them on the same line, but not the commits they were
# backported from. When a revert of a backport is merged forward into the line of the original, the original keeps the
# label. Remove the label by hand in that case as well.

if ((BASH_VERSINFO[0] < 4 || (BASH_VERSINFO[0] == 4 && BASH_VERSINFO[1] < 4))); then
	echo "$0: bash 4.4 or newer is required" >&2
	exit 1
fi

set -euo pipefail
shopt -s nocasematch

repo=${1:?usage: $0 <owner/repo>}
export LABEL=${LABEL:-resolved}
reopened=${REOPENED:-}
dry_run=${DRY_RUN:-0}

repo_re=${repo//./\\.}
# GitHub closing keywords, optionally followed by a colon, referring to an issue of this repository.
closing_re="(^|[^[:alnum:]_])(close[sd]?|fix(e[sd])?|resolve[sd]?):?[[:space:]]+(#|${repo_re}#|https://github\\.com/${repo_re}/issues/)([0-9]+)([^[:alnum:]_]|$)"
# The start of a line, also of one listed in the message of a squashed pull request.
line_start="(^|"$'\n'")[[:space:]*-]*"
# A full or abbreviated commit hash.
hash_re='[0-9a-f]{7,40}'
# Lines written by `git revert`, which quote the reverted subject, closing keywords included: the subject, also when
# listed in the message of a squashed pull request, and the references to the reverted commit and, for a merge, to its
# parent made with `--reference`.
revert_subject_re="${line_start}((Revert|Reapply) \"|This reverts commit |changes made to ${hash_re} \\()"
# The start of the line written by `git revert`, also when reworded.
revert_start_re="${line_start}This reverts commit (${hash_re})"
# Written by `git revert`, with a full or abbreviated hash, followed by the subject and date with `--reference`; the
# second part is added when reverting a merge.
revert_re="${revert_start_re}(( \\([^"$'\n'"]*\\))?,[[:space:]]+reversing[[:space:]]+changes[[:space:]]+made[[:space:]]+to[[:space:]]+(${hash_re}))?(\\.|[[:space:]]\\()"
# Written by `git cherry-pick -x`.
cherry_pick_re="${line_start}\(cherry picked from commit ([0-9a-f]{40})\)"

declare -A resolved=() reverted=()

# Calls a function for each match of a pattern in a text, with the groups of the match in BASH_REMATCH.
for_each_match() {
	local pattern=$1 rest=$2 callback=$3 match
	while [[ ${rest} =~ ${pattern} ]]; do
		match=${BASH_REMATCH[0]}
		"${callback}"
		rest=${rest#*"${match}"}
	done
}

# Adds the issues that a commit message closes to the named associative array.
collect_closed() {
	# shellcheck disable=SC2034 # Used by add_closed, which it calls.
	local -n closed=$1
	local line
	while IFS= read -r line; do
		[[ ${line} =~ ${revert_subject_re} ]] || for_each_match "${closing_re}" "${line}" add_closed
	done <<< "$2"
}

# shellcheck disable=SC2329 # Called by for_each_match.
add_closed() {
	# shellcheck disable=SC2004,SC2034 # A nameref to an associative array, which shellcheck does not follow.
	closed[${BASH_REMATCH[5]}]=1
}

# shellcheck disable=SC2329 # Called by for_each_match.
add_identity() {
	identities+=("${BASH_REMATCH[2]}")
}

# shellcheck disable=SC2329 # Called by for_each_match.
add_revert_targets() {
	local target parent merged
	((++recognized))
	# The reverted commit may be missing from the clone if it was made on a branch that is gone.
	if ! target=$(git rev-parse --verify --quiet "${BASH_REMATCH[2]}^{commit}"); then
		echo "warning: ${commit} reverts ${BASH_REMATCH[2]}, which is not available" >&2
		# A full hash can still match the origin of a cherry-picked commit.
		[[ ${#BASH_REMATCH[2]} != 40 ]] || targets+=("${BASH_REMATCH[2]}")
		return
	fi
	targets+=("${target}")
	if [[ -n ${BASH_REMATCH[5]} ]]; then
		# A reverted merge undoes every commit it brought in.
		if parent=$(git rev-parse --verify --quiet "${BASH_REMATCH[5]}^{commit}"); then
			merged=$(git rev-list "${target}" "^${parent}")
			[[ -z ${merged} ]] || mapfile -t -O "${#targets[@]}" targets <<< "${merged}"
		else
			echo "warning: ${commit} reverts a merge onto ${BASH_REMATCH[5]}, which is not available" >&2
		fi
	fi
}

# shellcheck disable=SC2329 # Called by for_each_match.
count_revert_line() {
	((++mentioned))
}

# Succeeds if a revert seen so far cancels the commit or a commit it was cherry-picked from.
is_cancelled() {
	local identity
	for identity in "${identities[@]}"; do
		[[ -z ${cancelled[${identity}]:-} ]] || return 0
	done
	return 1
}

# Commits released as cherry-picks, for example when a backport to a release line made the latest release.
declare -A released_origins=()

# shellcheck disable=SC2329 # Called by for_each_match.
add_released_origin() {
	released_origins[${BASH_REMATCH[2]}]=1
}

while IFS= read -r -d '' text; do
	for_each_match "${cherry_pick_re}" "${text}" add_released_origin
done < <(git log -z --regexp-ignore-case --grep='cherry picked from commit' --format=%B origin/master)
wait $!

# Succeeds if a commit has been released, itself or as a cherry-pick.
is_released() {
	[[ -n ${released_origins[$1]:-} ]] || git merge-base --is-ancestor "$1" origin/master 2> /dev/null
}

# Adds the issues that the commit closes to resolved or reverted. A squashed pull request can bundle several
# cherry-picked commits, so an issue that one of them closes is skipped if that commit has been released, counts as
# reverted if a revert cancels it, and as resolved otherwise, in this order of precedence. An issue that none of them
# closes counts as reverted if a revert cancels some of them and none stands, and as resolved otherwise.
classify_closed() {
	local -A own=() released=() undone=() kept=()
	local origin text number standing='' cancelled_any=''
	# The scan covers only commits that are not on master, so the commit itself can only have been released as a cherry-pick.
	[[ -z ${released_origins[${commit}]:-} ]] || return 0
	collect_closed own "${message}"
	for origin in "${identities[@]:1}"; do
		text=$(git log -1 --format=%B "${origin}" 2> /dev/null) || text=''
		if is_released "${origin}"; then
			collect_closed released "${text}"
		elif [[ -n ${cancelled[${origin}]:-} ]]; then
			collect_closed undone "${text}"
			cancelled_any=1
		else
			collect_closed kept "${text}"
			standing=1
		fi
	done
	for number in "${!own[@]}"; do
		if [[ -n ${released[${number}]:-} ]]; then
			continue
		elif [[ -n ${undone[${number}]:-} || (-z ${kept[${number}]:-} && -n ${cancelled_any} && -z ${standing}) ]]; then
			reverted[${number}]=1
		else
			resolved[${number}]=1
		fi
	done
}

branches=$(git for-each-ref --format='%(refname)' refs/remotes/origin/develop 'refs/remotes/origin/release/*')
if [[ ${branches} != *refs/remotes/origin/develop* ]]; then
	echo "$0: origin/develop is not available" >&2
	exit 1
fi

declare -A cancelled=()
for branch in ${branches}; do
	# Walk from the newest commit, so that a revert is seen before the commits it reverts.
	# A revert that is itself reverted has no effect.
	cancelled=()
	while IFS= read -r -d '' commit && IFS= read -r -d '' message; do
		identities=("${commit}")
		for_each_match "${cherry_pick_re}" "${message}" add_identity
		if [[ -n ${cancelled[${commit}]:-} ]]; then
			collect_closed reverted "${message}"
			continue
		fi

		if ! is_cancelled; then
			targets=() recognized=0 mentioned=0
			for_each_match "${revert_re}" "${message}" add_revert_targets
			for_each_match "${revert_start_re}" "${message}" count_revert_line
			if ((mentioned > recognized)); then
				echo "warning: ${commit} has a reworded \`This reverts commit\` line, which is ignored" >&2
			fi
			for target in "${targets[@]}"; do
				cancelled[${target}]=1
			done
		fi

		classify_closed
	done < <(git log -z --topo-order --format='%H%x00%B' "origin/master..${branch}")
	wait $!
done

# Open issues, mapped to whether they have the label. Pull requests, closed and transferred issues are not listed.
declare -A open=()
while read -r number labeled; do
	open[${number}]=${labeled}
done < <(gh issue list --repo "${repo}" --state open --limit 10000 --json number,labels \
	--jq '.[] | "\(.number) \(any(.labels[]; .name == env.LABEL))"')
wait $!

failed=0

# Runs a command, or prints it in a dry run, and records a failure instead of stopping.
apply() {
	if [[ ${dry_run} == 1 ]]; then
		echo "  (dry run) $*"
	elif ! "$@"; then
		failed=1
	fi
}

for number in $(printf '%s\n' "${!resolved[@]}" | sort -n); do
	if [[ ${open[${number}]:-} == false ]]; then
		echo "Labeling #${number}"
		apply gh issue edit "${number}" --repo "${repo}" --add-label "${LABEL}"
	fi
done

for number in $(printf '%s\n' "${!reverted[@]}" | sort -n); do
	if [[ -z ${resolved[${number}]:-} && ${open[${number}]:-} == true ]]; then
		echo "Unlabeling #${number}: its closing commits have been reverted"
		apply gh issue edit "${number}" --repo "${repo}" --remove-label "${LABEL}"
		open[${number}]=false
	fi
done

if [[ -n ${reopened} && -z ${resolved[${reopened}]:-} && ${open[${reopened}]:-} == true ]]; then
	echo "Unlabeling #${reopened}: it has been reopened and no unreleased commit closes it"
	apply gh issue edit "${reopened}" --repo "${repo}" --remove-label "${LABEL}"
fi

exit "${failed}"
