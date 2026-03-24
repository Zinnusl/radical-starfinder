# Parallel Agent Workflow

## Ultrawork

When the user says **"ultrawork"**, dispatch N parallel agents (number depends on the task) to work on independent features simultaneously using git worktrees. Merge results back when all agents complete.

## Worktree Convention

Use numbered worker directories alongside this repo:

```
../radical-starfinder-worker01/   # branch: worker01-<feature>
../radical-starfinder-worker02/   # branch: worker02-<feature>
...
```

## Workflow

1. Create feature branches: `git branch worker01-<feature>`
2. Create worktrees: `git worktree add ../radical-starfinder-worker01 worker01-<feature>`
3. Dispatch one general-purpose agent per worktree with full context
4. Each agent: implements, builds (`cargo check`), commits on its branch
5. After all complete: merge each branch back, resolve conflicts, verify `cargo test`
6. Clean up: `git worktree remove`, delete branches if merged

## Playwright

Each agent runs in a shared environment. To avoid browser conflicts:

- **Use unique browser profiles**: Pass `--user-data-dir` with a worker-specific path (e.g., `../radical-starfinder-worker01/.browser-profile/`)
- **Use distinct ports**: If launching a dev server, each worker must bind to a unique port (e.g., worker01 → 3001, worker02 → 3002, etc.)
- **Prefer headless mode**: Always use `headless: true` to avoid stealing focus or blocking other agents
- **Close when done**: Always call `browser_close` when finished — never leave browser sessions open

## Agent Prompts

Give each agent:
- The worktree path as working directory
- Key files relevant to their feature
- Clear scope boundaries (what to touch, what NOT to touch)
- Instruction to run `cargo check` before committing
- Playwright isolation rules (see above) if browser interaction is needed
