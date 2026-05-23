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

### Help needed

- Snapcraft packaging
