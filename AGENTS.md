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

## Agent Prompts

Give each agent:
- The worktree path as working directory
- Key files relevant to their feature
- Clear scope boundaries (what to touch, what NOT to touch)
- Instruction to run `cargo check` before committing
