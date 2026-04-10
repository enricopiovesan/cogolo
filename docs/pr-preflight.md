# PR Preflight

Run this before opening a PR:

```bash
bash scripts/ci/pr_preflight.sh /tmp/pr-body.md
```

The file passed to the script should contain the final PR body, including:

- `## Governing Spec`
- `## Project Item`
- `## Validation`

The preflight gate checks three things before a PR is opened:

1. The branch is not behind `origin/main`.
2. The PR body passes the same body and spec-alignment gates used in CI.
3. The repository checks pass locally.

If any of those fail, fix them before creating the PR.
