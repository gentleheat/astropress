# Claude Code guidance for this repo

## Quality principle

End-user satisfaction is the only measure of success. User/session satisfaction is irrelevant.
Do not take an easy path now that creates a harder problem later. If a fix requires real work,
do the real work. Suppressing a problem is not fixing it.

## Security fix verification protocol

A security fix is not done until evidence says so — not when local audits pass.

### CodeQL / GHAS alerts

**Verification step (required before declaring done):**
```
bun run tooling/scripts/check-pr-ghas.ts
```
This queries `refs/pull/<N>/merge` — the ref the GitHub CodeQL gate actually evaluates. The
pre-push gate checks the branch ref, which can show 0 alerts while the PR merge ref still shows
open alerts from a stale scan.

After pushing, wait for the PR merge ref scan to complete, then run `check-pr-ghas.ts`.
Do not declare "fixed" until this script exits 0 on a scan that post-dates the fix commit.

### GitHub Actions shell safety

Do not interpolate `${{ inputs.* }}`, `${{ github.* }}`, or `${{ secrets.* }}` directly inside
any `run:` block. Put the value in step `env:` first, then reference the quoted shell variable.
`bun run audit:github-actions-shell` enforces this.

**Taint chain rules:**
- `obj[taintedKey]` propagates taint even if `obj` only contains safe hardcoded values.
- The only way to break a taint chain is to not use user-derived data at the sink at all.
- Validate by tracing every variable in the flagged expression back to its origin.

**Suppression policy:**
- Fix the code first. Suppression is a last resort, not a first response.
- Comments that explain why a vulnerability is "safe" are security anti-patterns —
  they document attack surface for readers with malicious intent.
- For HTTP→file operations in import scripts: use `downloadMediaToFile()` from
  `packages/astropress/src/import/download-media.ts` which enforces URL scheme validation,
  SSRF prevention, content-type allowlist, and file size limits.
- Every suppression that remains must be registered in `tooling/scripts/audit-suppressions.ts`
  with a rubric explaining why a code fix is impossible.

## Pre-push gates

The pre-push hook runs a full suite (~10 minutes). Do not re-run `git push` while one is already
in progress. Wait for the background task notification before concluding anything.

## Mutation testing — module-level constants belong in `*-data.ts` siblings

When a source file defines pure-data module-level constants (string literal sets,
SQL strings, label tables, default-message strings, error-shape dispatch tables,
seed config arrays, etc.) and is otherwise mutation-tested, **split the constants
to a `<name>-data.ts` sibling** and add `// stryker-disable-file: data-only — <reason>`
in the first 10 lines of that file. The prepush mutation gate honors this marker
to exempt the data file from the score denominator.

The reason this convention exists rather than catalog-by-catalog equivalence
entries: the project's stryker config sets `ignoreStatic: true` and
`coverageAnalysis: "perTest"`, but `static: true` mutants on top-level consts
nevertheless report as Survived because vitest-runner's worker model caches the
imported module across per-test runs (the mutated source initialiser only fires
once at module load and the cached module is reused for the rest of the test
cycle). The same test that catches the mutation under `bun vitest run` doesn't
catch it under stryker. Splitting + the data-only marker is the cleanest fix;
see UPSTREAM_CONTRIBUTIONS.md #15 for the upstream ask. Existing precedents in
`packages/astropress/src/`: `*-data.ts`, `*-error-shapes.ts`, `*-seed-data.ts`,
`*-defaults.ts`.

Audits that filter source files by filename prefix (e.g. `audit-error-handling.ts`
matches `admin-action-*.ts`) **must also honor the `stryker-disable-file: data-only`
marker** so a constants-only sibling doesn't trip rules meant for runtime files.

## Mutation testing — what the gate scores, and two traps

The pre-push gate scores each changed file as **killed / (covered, scoreable) mutants**
— see `scoreForFile` in `tooling/scripts/prepush-mutation-gate.ts`. Excluded from the
denominator: `NoCoverage`, `Ignored`, `static` (module-load) mutants, and anything
catalogued in `tooling/stryker/equivalent-mutants.json`. So when a file regresses, the
fix targets its **Survived** mutants, not its NoCoverage ones — covering an untested
branch helps only if the new test actually *kills* the mutant (a covered-but-surviving
mutant counts *against* the score). Three legitimate ways to clear a Survived mutant:

1. Add a test that observes the mutated behavior and fails on it.
2. Simplify the code so the mutation point no longer exists — drop a redundant `?? ""`,
   a provably-dead guard (e.g. an `if (!db)` fast-path that `withSafeRouteRegistryFallback`
   already covers), or a duplicated regex.
3. If it is genuinely equivalent (no test can distinguish it), add an entry to
   `equivalent-mutants.json` with a one-line reason. Keep that catalog honest: when you
   delete or move the code an entry describes, delete the entry.

**Trap 1 — related-test association through the barrel.** Stryker runs with
`vitest: { related: true }`, so it only runs the tests it believes cover a mutated file.
A test that imports the unit under test through the package barrel
(`@astropress-diy/astropress`) rather than its source module (`../src/<file>`) may not be
associated with that file's mutants: the assertion passes under `bun vitest run` but never
runs against the mutant, which then survives for no visible reason. **In mutation-killing
tests, import the unit directly from its source module.**

**Trap 2 — the baseline can go stale.** `baseline-scores.json` records the last-captured
score per file (keyed by content hash). If the gate has been unable to complete — e.g. the
dry run timed out — regressions accumulate undetected and the stored scores describe code
the current tests no longer cover, so a file's baseline can read far higher than reality.
Fix the coverage (never hand-lower the baseline), then regenerate with `bun run raise:baseline`
once the gate runs clean.

## Final PR verification loop

Before calling a CI or security fix done:

1. Run the local audits and targeted tests for the files you changed.
2. Push the branch and wait for the PR checks to update.
3. Confirm the originally failing check is green.
4. Run `bun run check:pr-ghas` against the PR merge ref.
5. Only then declare the branch fixed.
