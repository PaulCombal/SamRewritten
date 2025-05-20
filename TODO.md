### High priority

- Show the icon 'next most achieved' next to the next most achieved achievement, maybe other icons like 'rare' too when achievement has a low global achievement achieved percent
- Show global achievement achieved percent
- Grey out controls where the achievements/stats do not have correct permissions (cf stat_definition.rs. Achievements and stats have a protected flag. If this flag is on, it will not be possible to edit their value, probably because they are managed server-side).
- Fix achievement lookup bug (Searching achievements by name doesn't bring expected results)
- Finish the context menu (Refresh the app list/refresh the achievement entries should show when the context is relevant)
- For increment_only stats, do not allow setting lower values than current (Indicate that it is increment only? With a tooltip?)

[APP SERVER] Received: {"SetIntStat":[480,"NumGames",6]}
[CLIENT] Response deserialization failed: invalid type: boolean `true`, expected unit at line 1 column 15
- Investigate the above issue

### Medium priority

- Before initial release, add license banner headers. Respect banner headers from Gibbed's Steam Achievement Manager when needed
- Optimize image loading by accessing steam local banner images
- Add status to to main window: Loading, Error, Connected as {username}

### Low priority

- Add a feature to build with adwaita
- Show achievement progression when available
- Rust fmt the whole thing