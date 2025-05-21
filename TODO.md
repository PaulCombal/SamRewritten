### High priority (must finish before release)

- Show the icon 'next most achieved' next to the next most achieved achievement, maybe other icons like 'rare' too when achievement has a low global achievement achieved percent
- Show global achievement achieved percent
- Grey out controls where the achievements/stats do not have correct permissions (cf stat_definition.rs. Achievements and stats have a protected flag. If this flag is on, it will not be possible to edit their value, probably because they are managed server-side).
- Fix achievement lookup bug (Searching achievements by name doesn't bring expected results)
- Improve the "big buttons" look from the app list

At the very end:
- Add license banner headers. Respect banner headers from Gibbed's Steam Achievement Manager when needed
- Populate the About dialog, do a better logo
- Github action for snapcraft releases
- Write a Readme and documentation
- Share on Reddit

### Medium priority

- Optimize the stats page, as I'm fairly confident the timeout solution is a "hack" more than a correct solution
- Optimize image loading by accessing steam local banner images
- Github action to generate Windows build artifacts

### Low priority

- Add a feature to build with the Adwaita theme
- Show achievement progression when available
- Rust fmt the whole thing
- Improve error handling (handle .expects, .unwraps, etc)

### Nices to have

- Rounded corners around stats and achievements lists
- Is there no padding around spinboxes on other builds too?
- Is the spinner not always spinning always on my machine?
- Context menu "Refresh ach & statsF5" → add a gap between label and accel

### Help needed

- Find a solution to this problem: https://github.com/PaulCombal/achievement-poc