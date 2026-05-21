### High priority

- Surface progress during bulk operations. Export/Import/UnlockAll/Reset fan out
  across many apps in the orchestrator but the GUI just shows a static
  "Working on N app(s)..." label and appears frozen for several seconds on large
  libraries. The plumbing already exists: `run_command_on_apps_concurrent` takes
  a `progress` callback that the orchestrator currently passes as `None`. Wire it
  to a progress IPC message and a determinate progress bar (step/total).

### Medium priority

- Let the user pick which Steam install to use when several are found. Today we
  only warn and silently take the first match (`dirs[0]`); the only override is
  the `SAM_STEAM_INSTALL_ROOT` env var. Replace the warning with a picker,
  persist the choice, and honor it in both the locator and the orchestrator.
- Replace the truncated "... and N more" lists with a scrollable, selectable
  view. Several dialogs (bulk unlock failures, export/import results, reset
  candidates) cut lists off at 10 entries inside a plain `AlertDialog` detail
  string. Add one reusable dialog backed by a `TextView` in a `ScrolledWindow`
  so the full list is visible and copyable.

### Low priority

- Show achievement progression when available
- Improve error handling (handle .expects, .unwraps, etc)
- Route `--auto-open` through the running orchestrator instead of spawning a
  second instance.

### Help needed

- Snapcraft packaging
