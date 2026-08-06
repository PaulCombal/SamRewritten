### Low priority

- default completion sort -> where in the scrolling are we?
- Make the library-wide bulk actions undoable. The action journal records that
  "Unlock all in selection", "Reset all in selection" and an import happened,
  and to which apps, but not what they changed: the per-item before-state only
  ever exists inside the app-server child, and nothing carries it back. So the
  very misclick worth undoing — a bulk unlock across a whole selection — still
  cannot be. Fix: have the child call `collect_app_export` before it mutates and
  return that snapshot alongside its result, then record it as the operation's
  before-image. The undo path already knows what to do with one — it hands
  `AppExport`s to `ImportApps`.
- Record the achievements Steam grants by itself. Storing one unlock makes Steam
  re-evaluate every stat-driven achievement in that game, so a single click can
  unlock several — SamRewritten asked for one, Steam did nine. The journal only
  knows what was asked for, so those extras are neither listed nor undoable, and
  they would re-grant themselves anyway while the stats behind them stand.
  Catching them means re-reading the app's achievements after a store and
  recording the difference; worth doing for the record even though the undo
  cannot help.
- Undo a single stat edit. Deliberately left out: Steam refuses a decrease on an
  increment-only stat (`progress_io.rs:classify_stat` spells out which), so the
  button would fail often enough to be worse than not offering it. The per-app
  reset's undo does restore stats, because `apply_app_export` puts both halves
  back in one pass.

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
