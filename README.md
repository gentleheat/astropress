# stryker-state

This branch is the shared backend for `tooling/scripts/run-mutants-shared.ts`.

- `incremental.json` — the latest `.stryker-incremental.json` from any successful run.
- `lock.json` — present while a run holds the lock; contains `{host,pid,startedAt,ttlHours}`.

Do not edit by hand. Do not merge into `main`.
