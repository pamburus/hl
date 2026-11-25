#!/usr/bin/env -S uv run --script
# /// script
# requires-python = ">=3.10"
# ///
"""Waits until a pull request is merged, or until something on it needs attention.

Usage:
  wait-pr.py PR TARGET-BRANCH [--since TIME] [--timeout SECONDS]

Lets an agent wait for a pull request in the background without polling by itself. Checks the pull request with gh
right away, then after 15 seconds and at doubling intervals up to 15 minutes, for at most 24 hours by default, and
exits:

  0  when it is merged into TARGET-BRANCH; prints "merged"
  1  when it is closed without being merged, or moved to another target branch; prints "closed" or "retargeted BRANCH"
  2  when the timeout expires; prints "timeout"
  3  when there is activity newer than TIME: a review, a comment or a failed or cancelled check, which prints
     "activity TIME"; or when the pull request has a merge conflict, which prints "conflict"
  4  on an error of the script itself, such as wrong arguments or a missing gh

TIME is in the ISO 8601 form GitHub uses, such as 2026-11-25T10:00:00Z. Without --since, the time the pull request was
created is used, so activity that came before the start counts too. To keep waiting after handling activity, run the
script again with --since and the time it printed, so that activity that came in meanwhile is not missed.

Between the checks, it waits with `gh event listen` and checks right away when a review is submitted, the state of
the pull request changes, a check suite completes or a merge conflict appears. An event only triggers a check sooner;
the checks still run on schedule, because there are no events for comments and events are lost while the stream
reconnects. Event subscriptions are a preview of gh, so when `gh event listen` fails, the script sleeps until the
next check instead, and tries to listen again after it.

Failures of gh, for example network errors or a request that takes over 2 minutes, are reported to stderr and retried
at the next check.
"""

import argparse
import json
import subprocess
import sys
import time
import traceback

MIN_INTERVAL = 15
MAX_INTERVAL = 15 * 60
FAILED = {"FAILURE", "ERROR", "TIMED_OUT", "CANCELLED", "ACTION_REQUIRED", "STARTUP_FAILURE"}
EVENTS = ",".join(
    f"com.github.pull_request.{event}.v1"
    for event in ["review.submitted", "state.changed", "check_suite.status.changed", "merge_conflict"]
)
LISTEN_TIMED_OUT = 8
QUERY_TIMEOUT = 120
EXIT_ERROR = 4


class ArgumentParser(argparse.ArgumentParser):
    """Exits with EXIT_ERROR on wrong arguments, because the default exit code 2 means a timeout here."""

    def error(self, message):
        self.print_usage(sys.stderr)
        print(f"{self.prog}: error: {message}", file=sys.stderr)
        sys.exit(EXIT_ERROR)


def query(pr):
    """Returns the state of the pull request as JSON, or None if gh fails. Its messages, such as hints, are printed
    only when it fails, to keep the output of a long wait short."""
    fields = "url,state,baseRefName,createdAt,mergeable,reviews,comments,statusCheckRollup"
    try:
        # gh sets no timeout of its own, so a stalled request would keep the script past its deadline.
        result = subprocess.run(
            ["gh", "pr", "view", pr, "--json", fields], capture_output=True, text=True, timeout=QUERY_TIMEOUT
        )
    except subprocess.TimeoutExpired:
        result = None
    if result is not None and result.returncode == 0:
        try:
            return json.loads(result.stdout)
        except json.JSONDecodeError:
            pass
    if result is not None:
        sys.stderr.write(result.stderr)
    print(f"{sys.argv[0]}: cannot read pull request {pr}, retrying later", file=sys.stderr)
    return None


def latest_activity(info):
    """Returns the time of the latest review, comment or failed or cancelled check, or an empty string if there is
    none."""
    times = [review.get("submittedAt") for review in info.get("reviews") or []]
    times += [comment.get("createdAt") for comment in info.get("comments") or []]
    for check in info.get("statusCheckRollup") or []:
        if (check.get("conclusion") or check.get("state")) in FAILED:
            times.append(check.get("completedAt") or check.get("startedAt") or check.get("createdAt"))
    return max((t for t in times if t), default="")


class Waiter:
    """Waits until a given time or until an event on the pull request, whichever comes first."""

    def __init__(self, deadline):
        self.deadline = deadline
        self.url = None
        self.warned = False

    def wait(self, until):
        """Returns False when the total deadline has passed."""
        until = min(until, self.deadline)
        seconds = int(until - time.monotonic())
        if seconds <= 0:
            return time.monotonic() < self.deadline
        if self.url:
            command = ["gh", "event", "listen", "--resource", self.url, "--event", EVENTS, "--timeout", f"{seconds}s"]
            try:
                result = subprocess.run(command, capture_output=True, text=True, timeout=seconds + 60)
            except subprocess.TimeoutExpired:
                result = None
            if result is not None and result.returncode in (0, LISTEN_TIMED_OUT):
                return True
            # A failure may be transient, so listening is tried again after the next check; warn only once.
            if not self.warned:
                if result is not None:
                    sys.stderr.write(result.stderr)
                print(f"{sys.argv[0]}: cannot listen to events, sleeping until the next check", file=sys.stderr)
                self.warned = True
        time.sleep(max(0, until - time.monotonic()))
        return True


def main():
    parser = ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    parser.add_argument("pr", help="number or URL of the pull request")
    parser.add_argument("target", help="branch the pull request must be merged into")
    parser.add_argument("--since", help="report only activity newer than this time")
    parser.add_argument("--timeout", type=int, default=24 * 60 * 60, help="seconds to wait in total")
    args = parser.parse_args()

    waiter = Waiter(time.monotonic() + args.timeout)
    since = args.since
    interval = MIN_INTERVAL
    next_check = time.monotonic()

    while True:
        info = query(args.pr)
        if info is not None:
            waiter.url = info["url"]
            base = info["baseRefName"]
            activity = latest_activity(info)
            if since is None:
                since = info["createdAt"]
            if info["state"] == "CLOSED":
                print("closed")
                return 1
            if base != args.target:
                print(f"retargeted {base}")
                return 1
            if info["state"] == "MERGED":
                print("merged")
                return 0
            if activity > since:
                print(f"activity {activity}")
                return 3
            if info["mergeable"] == "CONFLICTING":
                print("conflict")
                return 3

        # An event triggers an earlier check but does not move the schedule.
        if time.monotonic() >= next_check:
            next_check = time.monotonic() + interval
            interval = min(interval * 2, MAX_INTERVAL)
        if not waiter.wait(next_check):
            print("timeout")
            return 2


if __name__ == "__main__":
    try:
        sys.exit(main())
    except Exception:
        traceback.print_exc()
        sys.exit(EXIT_ERROR)
