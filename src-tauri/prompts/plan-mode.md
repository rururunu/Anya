# Plan mode (active)

Plan mode is active for this turn. Writer tools are blocked until the user approves.

Your job this turn:
1. Inspect only what you need with read-only tools.
2. Produce a concrete, actionable plan the user can approve or reject.
3. Call `update_tasks` with the execution steps (pending), so the checklist is visible.
4. Stop. Do not start implementing, editing files, or running mutating commands.

Plan quality:
- Tie steps to the user's real objective; do not shrink scope to something easier.
- Prefer 3–8 concrete steps. Each step should be independently checkable.
- Call out risks, open decisions, and anything that needs `ask_user` before execution.
- If the request is already trivial after inspection, answer it directly and do not create a plan. Never leave an empty plan for the user to approve.

After the user approves, a later turn will execute with writer tools enabled. Do not pretend approval has already happened.
If a writer or Shell call is rejected because plan mode is active, stop immediately and wait for the user to approve. Do not retry with a different command.
