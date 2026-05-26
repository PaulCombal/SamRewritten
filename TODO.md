### VERY HIGH

Add the install form snap store buttons in readme and docs/index.html

### Low priority

- Recover when Flatpak Steam is started *after* SamRewritten. The orchestrator
  only joins Flatpak Steam's PID namespace at startup (`enter_flatpak_steam_ns_if_needed`),
  and the join must happen before any threads exist, so a Flatpak Steam launched
  later can't be connected to in-process — it fails with a broken pipe and stays
  on the "Is Steam running?" screen until the app is restarted. (Native/Snap
  installs already recover on refresh via the per-message liveness check in
  `ensure_connected`; only Flatpak has this gap.) Likely fix: respawn the
  orchestrator on demand so the namespace join is re-evaluated fresh.
- Show achievement progression when available
- Improve error handling (handle .expects, .unwraps, etc)
- Route `--auto-open` through the running orchestrator instead of spawning a
  second instance.
- Third-party license attribution. The statically-linked Cargo crates are mostly
  MIT/Apache-2.0/BSD, whose notices must be reproduced in distributions; GTK4 and
  libadwaita are LGPL (dynamically linked, lighter requirement). Auto-generate the
  transitive license report (`cargo about` or `cargo-bundle-licenses`), embed it,
  and surface it in the About dialog — `adw::AboutDialog` has `add_legal_section`;
  `gtk::AboutDialog` falls back to a credit section. (Shipping a LICENSES file in
  the AppImage is the legal minimum if a dialog section is too much.)

### Help needed

- Snapcraft packaging
