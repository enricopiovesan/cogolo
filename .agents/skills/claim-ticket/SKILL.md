---
name: "claim-ticket"
description: "Claim a GitHub issue for Claude Code before starting work, preventing Codex from picking it up simultaneously."
compatibility: "Requires gh CLI authenticated as enricopiovesan on github.com"
metadata:
  author: "claude-code"
---

## User Input

```text
$ARGUMENTS
```

Extract the issue number from the user input. It may be provided as a plain number (`183`), a hash reference (`#183`), or a full URL.

## Pre-flight Check

Before claiming, verify the ticket is not already claimed:

1. Run:
   ```bash
   gh issue view <NUMBER> --repo enricopiovesan/Traverse --json labels,assignees
   ```

2. If the labels include `agent:claude` or `agent:codex`, **stop** and report:
   ```
   Issue #<NUMBER> is already claimed by <agent>. Choose a different ticket.
   ```

3. If a branch named `codex/issue-<NUMBER>-*` or `claude/issue-<NUMBER>-*` already exists remotely, **stop** and report:
   ```
   A branch already exists for issue #<NUMBER>. Choose a different ticket.
   ```
   Check with:
   ```bash
   git ls-remote --heads origin | grep "issue-<NUMBER>-"
   ```

## Claim

If the ticket is free, claim it:

1. **Add label**:
   ```bash
   gh issue edit <NUMBER> --repo enricopiovesan/Traverse --add-label "agent:claude"
   ```

2. **Set Agent field on Project 1**:
   ```bash
   gh project item-list 1 --owner enricopiovesan --format json \
     | python3 -c "
   import json,sys
   items = json.load(sys.stdin)['items']
   match = [i for i in items if str(i.get('content',{}).get('number','')) == '<NUMBER>']
   if match: print(match[0]['id'])
   "
   ```
   Then set the field:
   ```bash
   gh project item-edit --project-id PVT_kwHOAEZXvs4BS6Ns \
     --id <ITEM_ID> \
     --field-id PVTSSF_lAHOAEZXvs4BS6NszhBK-Qk \
     --single-select-option-id 6673c204
   ```

3. **Set Status to In Progress** (option id `47fc9ee4`):
   ```bash
   gh project item-edit --project-id PVT_kwHOAEZXvs4BS6Ns \
     --id <ITEM_ID> \
     --field-id PVTSSF_lAHOAEZXvs4BS6NszhATmdM \
     --single-select-option-id 47fc9ee4
   ```

## Report

After claiming, output:

```
Claimed issue #<NUMBER> for Claude Code.
Label: agent:claude
Project board: Agent → Claude Code, Status → In Progress

Safe to start work. Suggested branch: claude/issue-<NUMBER>-<slug>
```

Where `<slug>` is a short kebab-case summary of the issue title.
