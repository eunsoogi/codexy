# Applying selected batch changes

The engineering workflow keeps transformation results separate from the user's
workspace. After `batch_change_resume.py` has produced a JSON result, review and
apply only explicitly selected successful items:

```sh
python plugins/codexy/skills/engineering/scripts/batch_change.py \
  --workspace-root /path/to/workspace \
  --results /path/to/resume-result.json \
  --select item-001 item-004
```

The command emits one result for every input item. Selected successful items
include a unified diff and a readback fingerprint. The source original is
checked again immediately before each replacement. A changed source, changed
destination, missing artifact, failed result, or interrupted item is reported
without overwriting the affected file. `completed` means the output was applied
or already matched the validated artifact; `conflict` and `incomplete` remain
distinct so a partial application is never reported as full success.

Apply state is stored only in the workspace's `.codexy-batch-apply` directory;
it is not component inventory or journal state. Repeating the same selection is
safe and reports already-applied outputs as `completed`. The workflow performs
independent per-file replacements and never commits or pushes user changes.

Use `preview` or `--dry-run` to emit selected diffs without replacing files:

```sh
python plugins/codexy/skills/engineering/scripts/batch_change.py preview \
  --workspace-root /path/to/workspace \
  --results /path/to/resume-result.json \
  --select item-001
```
