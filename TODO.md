### High priority (must finish before release)

- Put apps.xml somewhere more meaningful
- Give user feedback when a search returns no result

At the very end:
- Add license banner headers. Respect banner headers from Gibbed's Steam Achievement Manager when needed
- Populate the About dialog, do a better logo
- Write a Readme and documentation
- Share on Reddit

At the very very end (open-source repo required):
- Github action for snapcraft releases

### Medium priority

- Find out why this looks like crap on Linux. Find out if builds will also look like crap on other Linux machines.
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