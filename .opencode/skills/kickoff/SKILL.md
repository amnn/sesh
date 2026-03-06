---
name: kickoff
description: Review TODO.md and pick the next best task to start now
---

## Goal

Start execution quickly by choosing the next most appropriate TODO item.

## Steps

1. Read `TODO.md` from the repository root.
2. Identify open items (`- [ ]`) and in-progress hints (`- [-]`).
3. Choose one concrete next task using these tie-breakers in order:
   - Prefer tasks that unblock other listed work.
   - Prefer tasks with clear acceptance criteria over vague investigations.
   - Prefer smaller, high-leverage tasks when multiple options are equal.
   - If a section is already active, continue within that section for momentum.
4. Propose a concrete plan first.
5. Ask for explicit approval before starting implementation.
6. If information is missing, ask targeted questions as needed, with at most
   one question per area (for example scope, risk, environment, acceptance).
   For each question, include a recommended default and explain what changes
   based on the answer.

## Output

- State the selected TODO item (quote the exact checklist line).
- Explain briefly why it was selected.
- Provide a short plan in the same response.
- End by asking for explicit go-ahead before implementation begins.

## Constraints

- Do not mark TODO items complete unless the implementation is actually done.
- Do not rewrite or reprioritize the whole TODO list unless explicitly asked.
- Do not start implementation until the user explicitly approves the plan.
