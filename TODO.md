### High priority (must finish before release)

- Give user feedback when a search returns no result

At the very end:
- Share on Reddit
- Desktop entry / .exe icon
- Add screenshots to the Readme
- Upload an AppImage

### Medium priority

- Instead of having small bars under the achievements to represent gloable achievement percentage, can we make it the background? would it look any better?
- Support for multiple simultaneous instances ('launch in new window' button)
- Add a context menu entry for the App view: Reset stats & achievements (steamuserstats.reset_all_stats)
- Optimize image loading by accessing steam local banner images
- Github action to generate Windows build artifacts

### Low priority

- Add a feature to build with the Adwaita theme
- Show achievement progression when available
- Rust fmt the whole thing
- Improve error handling (handle .expects, .unwraps, etc)

### Nices to have

- In utils, instead of checking for snap variables at runtime, only compile the necessary check.

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