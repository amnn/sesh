## Validation

Once you are happy with a solution, run the `nits` subagent to validate and
auto-fix style issues so the solution meets [code style guidelines](./STYLE.md).

Use a fix-first invocation: `@nits Fix style issues in-place; do not only
suggest changes. Report unresolved items with reasons.`

> [!IMPORTANT]
> Invoke it as a subagent (`@nits`), not as a shell command -- **there is no
> `nits` binary**.
