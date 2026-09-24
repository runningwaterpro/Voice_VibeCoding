# Issue tracker: GitHub

Issues and specs for this repo live in GitHub Issues. Use the `gh` CLI for all operations.

## Repository selection

This clone has multiple remotes:

- `gh`: GitHub repository
- `origin`: local path
- `fromfix`: local path

Always target the GitHub repository explicitly:

```bash
gh issue list --repo runningwaterpro/Voice_VibeCoding
gh issue view <number> --repo runningwaterpro/Voice_VibeCoding --comments
```

Alternatively, set `GH_REPO=runningwaterpro/Voice_VibeCoding` for the session.

Never put access tokens in documentation, issue bodies, command examples, or remote URLs.

## Conventions

- **Create an issue**: `gh issue create --repo runningwaterpro/Voice_VibeCoding --title "..." --body "..."`.
- **Read an issue**: `gh issue view <number> --repo runningwaterpro/Voice_VibeCoding --comments`, filtering comments by `jq` and also fetching labels.
- **List issues**: `gh issue list --repo runningwaterpro/Voice_VibeCoding --state open --json number,title,body,labels,comments --jq '[.[] | {number, title, body, labels: [.labels[].name], comments: [.comments[].body]}]'` with appropriate `--label` and `--state` filters.
- **Comment on an issue**: `gh issue comment <number> --repo runningwaterpro/Voice_VibeCoding --body "..."`.
- **Apply / remove labels**: `gh issue edit <number> --repo runningwaterpro/Voice_VibeCoding --add-label "..."` / `--remove-label "..."`.
- **Close**: `gh issue close <number> --repo runningwaterpro/Voice_VibeCoding --comment "..."`.

## Pull requests as a triage surface

**PRs as a request surface: no.** _(Set to `yes` if this repo treats external PRs as feature requests; `/triage` reads this flag.)_

When set to `yes`, PRs run through the same labels and states as issues, using the `gh pr` equivalents.

GitHub shares one number space across issues and PRs, so a bare `#42` may be either: resolve with `gh pr view 42` and fall back to `gh issue view 42`.

## When a skill says "publish to the issue tracker"

Create a GitHub issue.

## When a skill says "fetch the relevant ticket"

Run `gh issue view <number> --repo runningwaterpro/Voice_VibeCoding --comments`.

## Wayfinding operations

Used by `/wayfinder`. The **map** is a single issue with **child** issues as tickets.

- **Map**: a single issue labelled `wayfinder:map`, holding the Notes / Decisions-so-far / Fog body.
- **Child ticket**: an issue linked to the map as a GitHub sub-issue. Where sub-issues are not enabled, add the child to a task list in the map body and put `Part of #<map>` at the top of the child body.
- **Blocking**: use GitHub native issue dependencies. Where dependencies are unavailable, fall back to a `Blocked by: #<n>, #<n>` line at the top of the child body.
- **Frontier query**: list the map's open children, dropping any with an open blocker or an assignee.
- **Claim**: `gh issue edit <n> --repo runningwaterpro/Voice_VibeCoding --add-assignee @me`.
- **Resolve**: `gh issue comment <n> --body "<answer>"`, then `gh issue close <n>`, then append a context pointer to the map's Decisions-so-far.
