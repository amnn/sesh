## Validation

Once you are happy with a solution, run the `nits` subagent to validate and
auto-fix style issues so the solution meets [code style guidelines](./STYLE.md).

Use a fix-first invocation: `@nits Fix style issues in-place; do not only
suggest changes. Report unresolved items with reasons.`

> [!IMPORTANT]
> Invoke it as a subagent (`@nits`), not as a shell command -- **there is no
> `nits` binary**.

## Truth Seeking

Default to verified claims over plausible guesses.

- Do not assume facts about the codebase, runtime behavior, or external tools
  when they can be checked directly.
- Validate assumptions with repo evidence first (for example file reads,
  searches, tests, or command output) before acting on them.
- Prefer to ground factual claims in external documentation or another citable
  source; make a reasonable effort to find one before relying on memory alone,
  and cite the source you used in the response.
- If a key fact cannot be verified safely, call out the uncertainty explicitly,
  state the recommended default, and explain what would change if that default
  is wrong.
- Keep questions targeted and minimal: only ask when the missing fact would
  materially change the implementation or create risk.

## Reflection

After meaningful implementation work, use the `reflection` skill as the source
of truth for capturing durable lessons in repo-local agent guidance.
