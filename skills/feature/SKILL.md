---
name: feature
description: Execute the "feature" command to (1) update the Unreleased changelog entry for recent changes and (2) provide a Conventional Commits headline + description. Trigger when the user mentions $feature or asks for a single command that does both tasks.
---

# Feature command workflow

Follow this two-step workflow and ask for confirmation before each action.

## Step 1: Update changelog (with confirmation)

1) Locate `CHANGELOG.md` and the `## [Unreleased]` section.
2) Propose a single bullet under `### Added` (or the most appropriate subsection) describing the new feature(s) from the current changes.
3) Ask the user to confirm the exact changelog text before editing the file.
4) If confirmed, apply the change. If not, revise and ask again.

## Step 2: Provide commit message (with confirmation)

1) Draft a Conventional Commits headline and a short description that covers all changes.
2) Ask the user to confirm before finalizing the commit message.
3) If confirmed, return the final message. If not, revise and ask again.
