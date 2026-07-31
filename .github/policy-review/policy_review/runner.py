# SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
#
# SPDX-License-Identifier: AGPL-3.0-only
"""Orchestration: one policy review, start to finish."""

from __future__ import annotations

import sys
from pathlib import Path

from . import context as ctx
from . import guard
from . import model as model_api
from . import policies as policy_loader
from . import prompts, report, slack
from .config import Config
from .guard import GuardHit
from .github_api import GitHubClient, GitHubError
from .redact import redact
from .verdict import Verdict, VerdictError, parse

EXIT_OK = 0
EXIT_VIOLATIONS = 1
EXIT_ERROR = 2


def log(message: str, *, config: Config | None = None) -> None:
    """Write a line to the workflow log, scrubbed of any secret value."""
    secrets = config.secrets() if config else ()
    print(redact(message, secrets), flush=True)


def _write_summary(text: str, summary_path: str | None) -> None:
    """Append to the GitHub Actions job summary, when one is available."""
    if not summary_path:
        return
    try:
        with open(summary_path, "a", encoding="utf-8") as handle:
            handle.write(text + "\n")
    except OSError:
        # A missing or unwritable summary file must never fail the check.
        pass


def _build_pr_context(
    config: Config,
    client: GitHubClient,
    repo_root: Path,
) -> ctx.PullRequestContext:
    """Collect everything about the pull request under review."""
    data = client.get_pull_request(config.pr_number)

    base_ref = config.base_ref or (data.get("base") or {}).get("ref") or ""
    head_sha = config.head_sha or (data.get("head") or {}).get("sha") or ""

    base = ctx.merge_base(base_ref, head_sha, repo_root)
    paths = ctx.changed_paths(base, head_sha, repo_root)
    diff, truncated = ctx.collect_diff(base, head_sha, repo_root, config.max_diff_bytes)

    body = data.get("body") or ""
    linked = None
    reference = ctx.find_issue_reference(body)
    if reference:
        owner, repo, number = reference
        issue = client.get_issue(owner, repo, number)
        linked = ctx.LinkedIssue(
            owner=owner,
            repo=repo,
            number=number,
            title=(issue or {}).get("title", ""),
            body=(issue or {}).get("body") or "",
        )

    return ctx.PullRequestContext(
        number=config.pr_number,
        title=data.get("title") or "",
        body=body,
        author=(data.get("user") or {}).get("login", "unknown"),
        base_ref=base_ref,
        head_sha=head_sha,
        is_draft=bool(data.get("draft")),
        changed_paths=paths,
        diff=diff,
        diff_truncated=truncated,
        linked_issue=linked,
    )


def _fallback_slack_text(
    config: Config,
    pr: ctx.PullRequestContext,
    verdict: Verdict,
    guard_hits: list[GuardHit],
) -> str:
    """A plain alert, used when the generated one is unavailable."""
    link = f"https://github.com/{config.repository}/pull/{pr.number}"
    if guard_hits and not verdict.violations:
        headline = (
            f"Policy review system changed in {config.repository} "
            f"#{pr.number} — {pr.title}. The review itself passed."
        )
    elif guard_hits:
        headline = (
            f"Policy violations *and* a change to the policy review system in "
            f"{config.repository} #{pr.number} — {pr.title}"
        )
    else:
        headline = (
            f"Policy violations in {config.repository} #{pr.number} — {pr.title}"
        )
    files = "".join(f"\n• {hit.path} ({hit.reason})" for hit in guard_hits[:5])
    return f"{headline}{files}\n{link}"


def _notify_slack(
    config: Config,
    pr: ctx.PullRequestContext,
    verdict: Verdict,
    guard_hits: list[GuardHit],
) -> None:
    """Generate and post the Slack alert. Never raises."""
    prompt = prompts.build_slack_prompt(
        message_prompt=config.slack_message_prompt,
        repository=config.repository,
        pr_number=pr.number,
        pr_title=pr.title,
        summary=verdict.summary,
        violations=[v.as_dict() for v in verdict.violations],
        guard_hits=guard_hits,
    )
    text = model_api.slack_message(
        api_key=config.anthropic_api_key, model=config.model, user_prompt=prompt
    )
    if not text:
        # Falling back to a plain message is better than staying silent about a
        # real violation, or about the machinery being changed.
        text = _fallback_slack_text(config, pr, verdict, guard_hits)
    try:
        slack.post_message(
            token=config.slack_bot_token,
            channel=config.slack_channel,
            text=redact(text, config.secrets()),
        )
        log(f"Slack alert posted to {config.slack_channel}.", config=config)
    except slack.SlackError as exc:
        log(f"Could not post the Slack alert: {exc}", config=config)


def _alert_or_log(
    config: Config,
    pr: ctx.PullRequestContext,
    verdict: Verdict,
    guard_hits: list[GuardHit],
) -> None:
    """Send the Slack alert if there is anything to announce, or say why not."""
    if not (verdict.violations or guard_hits):
        return
    if config.slack_enabled:
        _notify_slack(config, pr, verdict, guard_hits)
        return
    log(
        "Slack notification skipped: no channel or bot token configured. "
        + (
            "This pull request changes the policy review system itself, which "
            "would otherwise have been announced."
            if guard_hits
            else ""
        ),
        config=config,
    )


def run(config: Config, repo_root: Path, summary_path: str | None = None) -> int:
    """Execute one policy review. Returns the process exit code."""
    policies, source = policy_loader.load_policies(
        config.policies_path, config.base_ref, repo_root
    )
    if not policies:
        log(
            f"No policy files found under {config.policies_path!r}; nothing to "
            "enforce. Add a Markdown file there to start enforcing a policy.",
            config=config,
        )
        _write_summary("### Policy review\n\nNo policies configured.", summary_path)
        return EXIT_OK

    log(
        f"Loaded {len(policies)} policy file(s) from {source}: "
        + ", ".join(p.policy_id for p in policies),
        config=config,
    )

    client = GitHubClient(config.github_token, config.repository)
    pr = _build_pr_context(config, client, repo_root)
    log(
        f"Reviewing #{pr.number} by {pr.author}: {len(pr.changed_paths)} file(s) "
        f"changed, diff {'truncated' if pr.diff_truncated else 'complete'}.",
        config=config,
    )

    guard_hits = guard.find_hits(
        pr.changed_paths, config.policies_path, config.guarded_paths
    )
    if guard_hits:
        log(
            "This pull request changes the policy review system itself: "
            + ", ".join(str(hit) for hit in guard_hits),
            config=config,
        )

    system_prompt = prompts.build_system_prompt(policy_loader.render_for_prompt(policies))
    user_prompt = prompts.build_user_prompt(
        pr,
        repository=config.repository,
        repo_type=config.repo_type,
        policies_source=source,
        guard_hits=guard_hits,
    )

    try:
        raw = model_api.review(
            api_key=config.anthropic_api_key,
            model=config.model,
            effort=config.effort,
            system_prompt=system_prompt,
            user_prompt=user_prompt,
        )
        verdict = parse(raw)
    except (model_api.ModelError, VerdictError) as exc:
        reason = redact(str(exc), config.secrets())
        log(f"Policy review failed: {reason}", config=config)
        try:
            client.upsert_comment(
                pr.number, report.render_error_comment(reason, guard_hits)
            )
        except GitHubError as post_error:
            log(f"Could not post the failure comment: {post_error}", config=config)
        _write_summary(
            f"### ⚠️ Policy review could not be completed\n\n{reason}", summary_path
        )
        # A change to the machinery still deserves an alert even when the review
        # that would have judged it could not run — arguably more so.
        if guard_hits and config.slack_enabled:
            _notify_slack(config, pr, Verdict(summary=reason), guard_hits)
        return EXIT_ERROR

    blocking = verdict.blocking(config)
    comment = report.render_comment(
        verdict,
        policies_source=source,
        policy_count=len(policies),
        blocking_count=len(blocking),
        diff_truncated=pr.diff_truncated,
        guard_hits=guard_hits,
    )
    client.upsert_comment(pr.number, redact(comment, config.secrets()))

    if verdict.passed:
        log("All policies passed.", config=config)
        if guard_hits:
            _write_summary(
                "### ✅ Policy review — all policies passed\n\n"
                "⚠️ This pull request changes the policy review system itself "
                f"({len(guard_hits)} file(s)); a Slack alert was sent.",
                summary_path,
            )
            _alert_or_log(config, pr, verdict, guard_hits)
        else:
            _write_summary("### ✅ Policy review — all policies passed", summary_path)
        return EXIT_OK

    log(
        f"{len(verdict.violations)} violation(s), {len(blocking)} blocking.",
        config=config,
    )
    _write_summary(
        f"### ❌ Policy review — {len(verdict.violations)} violation(s), "
        f"{len(blocking)} blocking",
        summary_path,
    )

    # A formal "changes requested" review is reserved for the moment the pull
    # request is offered for human review; during drafting the comment is enough.
    if blocking and config.is_ready_for_review:
        body = report.render_review_body(verdict, blocking)
        if client.request_changes(pr.number, redact(body, config.secrets())):
            log("Requested changes on the pull request.", config=config)
        else:
            log(
                "Could not request changes (GitHub declined the review); the "
                "findings comment stands.",
                config=config,
            )

    _alert_or_log(config, pr, verdict, guard_hits)

    return EXIT_VIOLATIONS if blocking else EXIT_OK


def main(argv: list[str] | None = None) -> int:
    """Entry point. Reads configuration from the environment."""
    import os

    from .config import ConfigError, from_env

    try:
        config = from_env()
    except ConfigError as exc:
        print(f"Policy review is misconfigured: {exc}", file=sys.stderr, flush=True)
        return EXIT_ERROR

    if not config.anthropic_api_key:
        # Pull requests from forks do not receive secrets. Skipping is the
        # correct outcome: a maintainer's re-run will produce the review.
        log(
            "No model credentials available to this run (expected for a pull "
            "request from a fork). Skipping the policy review.",
            config=config,
        )
        return EXIT_OK
    if not config.github_token:
        print("Policy review needs a GitHub token.", file=sys.stderr, flush=True)
        return EXIT_ERROR

    try:
        return run(config, Path.cwd(), os.environ.get("GITHUB_STEP_SUMMARY"))
    except (GitHubError, policy_loader.PolicyLoadError, RuntimeError) as exc:
        log(f"Policy review failed: {exc}", config=config)
        return EXIT_ERROR
