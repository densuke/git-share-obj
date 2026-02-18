# Safe Fsck/Rollback Implementation Memo

This memo captures implementation guidance for adding safety-first behavior to `git-share-obj`.

## Scope

- Default behavior must prioritize safety over speed.
- Add pre/post validation with `git fsck` by default.
- Add rollback-friendly hardlink replacement flow.
- Add a standalone `fsck-only` mode independent of hardlink processing.
- Add repository-level lock coordination (lock file + OS advisory lock).
- Keep implementation simple and composable (UNIX philosophy).

## Branch Policy

- Work branch: `feat-safe-fsck-rollback`
- Do not implement directly on `main`.

## Development Process

- TDD-first:
  1. Write/adjust failing test.
  2. Implement minimum code to pass.
  3. Refactor while preserving behavior.
- Commit policy: one topic per commit.
  - Examples:
    - CLI options only
    - fsck module only
    - rollback mechanics only
    - i18n/messages only

## CLI Design (Safety-First Defaults)

- Keep current behavior but add:
  - `--no-fsck`
    - Long option only.
    - Skip fsck checks for speed (opt-out from safe default).
  - `--fsck-only`
    - Long option only.
    - Detect repositories and run `git fsck` only.
    - No hardlink replacement in this mode.
  - `--no-lock`
    - Long option only.
    - Skip repository lock acquisition for speed/compatibility (unsafe opt-out).

## Processing Model

### Repository detection

- Detect repositories under given paths by scanning for `.git/objects`.
- Build unique repository roots for fsck targets.

### Normal mode (default)

1. Detect repos.
2. Acquire repo locks (unless `--no-lock`).
3. Run pre-fsck (unless `--no-fsck`).
4. If pre-fsck has failures, abort replacement.
5. Run duplicate scan + hardlink replacement.
6. Run post-fsck (unless `--no-fsck`).

### Fsck-only mode

1. Detect repos.
2. Acquire repo locks (unless `--no-lock`).
3. Run `git fsck` for each repo.
4. Report summary and exit.

## Locking Model (Minimum)

- Lock target per repository:
  - lock file path: `.git/objects/git-share-obj.lock`
- Two-layer lock:
  1. lock file created/held by this process
  2. OS advisory lock on the lock-file descriptor (`flock`)
- Platform:
  - UNIX only (Linux/macOS expected runtime)
- Failure policy:
  - lock acquisition failure is reported
  - failed repos are skipped from destructive processing
  - summary includes lock-failed count

## Hardlink Safety Logic

Replace current dangerous sequence:

- Current: `remove(target)` -> `hard_link(source, target)`

With rollback-capable sequence:

1. `rename(target, target.bak)` (same directory)
2. `hard_link(source, target)`
3. success: delete `target.bak`
4. failure: remove partial `target` if exists, then rename `target.bak` back to `target`

## Output/UX Requirements

- In `-v` mode:
  - Print explicit announcements before each repo fsck starts.
  - Print per-repo result details.
- Rollback message on failure:
  - Must be printed regardless of `-v`.
  - Must indicate whether rollback succeeded or failed.
- In `-v` mode:
  - Print lock acquisition start/result per repository.
- Lock failure:
  - Must be printed regardless of `-v`.

## UNIX Philosophy Constraints

- Keep modules small and focused.
  - `scanner`: discovery/grouping
  - `fsck`: command execution/result parsing
  - `lock`: repo lock acquisition/release
  - `hardlink`: atomic replacement + rollback
  - `main`: orchestration only
- Prefer simple, explicit control flow over complex abstractions.
- Avoid hidden side effects.

## Test Plan (TDD Targets)

- CLI parsing:
  - `--no-fsck`
  - `--fsck-only`
  - `--no-lock`
- Repo detection:
  - Unique repo extraction from scanned paths
- Fsck runner:
  - Success/failure result handling
- Hardlink replacement:
  - Success path with backup cleanup
  - Failure path with rollback restore
- Main flow:
  - fsck-only does not perform replacement
  - no-fsck skips pre/post fsck
  - no-lock skips lock acquisition
  - default runs pre/post fsck
  - default acquires repo locks

## Acceptance Criteria

- Safe mode (with fsck) is default.
- `--no-fsck` is the only bypass path for fsck checks.
- `--fsck-only` can be run independently from hardlink processing.
- lock acquisition is enabled by default and bypassed only by `--no-lock`.
- Replacement failure no longer silently loses target file.
- Verbose fsck announcements exist.
- Rollback failure/success messages are always visible.
