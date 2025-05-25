### High priority (must finish before release)

- Show the icon 'next most achieved' next to the next most achieved achievement, maybe other icons like 'rare' too when achievement has a low global achievement achieved percent
- Show global achievement achieved percent
- Add an entry "Launch appId X" when only numbers are typed inside the app search bar

At the very end:
- Add license banner headers. Respect banner headers from Gibbed's Steam Achievement Manager when needed
- Populate the About dialog, do a better logo
- Github action for snapcraft releases
- Write a Readme and documentation
- Share on Reddit

### Medium priority

- Support for multiple simultaneous instances (launch in new window button)
- Add a context menu entry for the App view: Reset stats & achievements (steamuserstats.reset_all_stats)
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

In connect_bind closures, instead of calling unsafe steal_data, store the SignalHandlerId in the listItem
```rust
let handler_id = spin_button.connect_value_changed(|spin_button| {
println!("SpinButton value changed: {}", spin_button.value());
});
list_item.set_data("spin-button-value-changed-handler", handler_id);

...

spin_button.disconnect(handler_id);

```


### Help needed

- Find a solution to this problem: https://github.com/PaulCombal/achievement-poc