---
name: reflection
description: Learn from recent repo work; track PR reflection separately
---

## State

Latest PR analyzed: none

## Goal

Capture useful lessons from recent local work or a separate pull request
reflection pass, then encode those lessons in concrete repo-local agent
guidance updates.

## When to Use

- After finishing a meaningful implementation, especially one that exposed a
  new workflow, pitfall, or reusable pattern.
- When the user asks to improve agent behavior, repo guidance, skills, or
  subagents.
- When the user explicitly wants a pull request reflection pass; treat that as a
  separate activity from local-session reflection.
- Before closing a cluster of related TODO items if the work surfaced stable
  guidance that should apply in future sessions.

## Inputs to Review

### Local Reflection

Start with the strongest available evidence in this order:

1. The current conversation, current diff, and files changed in this session.
2. Recent local history using the version control tools available in the
   checkout.
3. Existing repo guidance such as `AGENTS.md`, `.agents/skills/`,
   `.agents/agents/`, and `TODO.md`.

### Pull Request Reflection

Treat pull request reflection as a separate pass from local reflection.

1. Start from the next unreviewed PR after the number recorded in this file's
   "State" section.
2. Analyze a contiguous chunk of PRs and their authoritative review feedback.
3. Update the description to the highest PR number reviewed in that pass.
4. Use existing repo guidance such as `AGENTS.md`, `.agents/skills/`,
   `.agents/agents/`, and `TODO.md` as the destination for durable lessons.

## What to Look For

- Repeated instructions the agent had to rediscover.
- Mistakes, false starts, or reviewer feedback that can be prevented next time.
- Repo-specific workflows that differ from generic defaults.
- New conventions worth encoding in a skill, agent, or top-level guidance.
- Gaps where the current guidance caused wasted motion or ambiguity.

For pull request reflection passes, focus on merged or otherwise authoritative
PRs and the review feedback attached to them.

Ignore one-off preferences unless they are likely to matter again.

## Steps

1. Decide whether this run is:
   - local reflection from the current session and recent commits, or
   - pull request reflection from the next unreviewed PR range.
2. Gather evidence from that source before proposing changes; for local
   reflection, start with the current diff and recent local history using the
   version control tools available in the checkout.
3. For pull request reflection, record the starting PR number, inspect the
   chosen PR chunk, and note the highest PR number fully analyzed.
4. Extract 1-3 durable lessons; prefer specific, actionable lessons over vague
   observations.
5. Map each lesson to the smallest appropriate home:
   - `AGENTS.md` for repo-wide standing instructions.
   - `.agents/skills/<name>/SKILL.md` for repeatable workflows.
   - `.agents/agents/<name>.md` for subagent behavior.
   - `TODO.md` only for follow-up work that should happen later.
6. Update the relevant files directly.
7. During local reflection, update `TODO.md` to match the finished work: tick
   off items that are complete, and remove or clean up sections whose items are
   all done.
8. If this was a pull request reflection pass, update this file's state so the
   `latest PR analyzed` value matches the highest PR number reviewed.
9. Summarize the evidence used, the lesson captured, and where it was encoded.

## Output

- State the evidence reviewed.
- State whether the run was local reflection or pull request reflection.
- List the lessons captured.
- Name the files updated.
- For pull request reflection, state the PR range reviewed and the new
  `latest PR analyzed` value.
- Note any follow-up that still belongs in `TODO.md` instead of guidance.

## Constraints

- Do not invent lessons without evidence from the repo, history, or PRs.
- Prefer tightening existing guidance over adding redundant new documents.
- Keep updates short, specific, and durable.
- When updating repo guidance such as `AGENTS.md`, preserve the existing
  section structure and formatting style; place new notes in the most specific
  section and keep admonitions attached to the guidance they qualify.
- Do not mix local-session reflection progress with pull request reflection
  progress; the tracked PR number applies only to PR analysis.
- Do not mark TODO items complete unless the corresponding implementation is
  actually done.
