Strict-confinement snap packaging works via a copy-on-startup workaround for
the `file_mmap` restriction on `personal-files` paths.

## The problem

The Snap store will not accept SamRewritten in classic confinement (the
relevant forum thread is https://forum.snapcraft.io/t/samrewritten/47964).
In strict confinement, the `personal-files` interface grants read on Steam's
`steamclient.so` but AppArmor denies `file_mmap` on personal-files locations,
so `dlopen()` cannot load it directly from
`$HOME/snap/steam/common/.local/share/Steam/linux64/steamclient.so`.

## The workaround

On every launch, `package/sam-launcher.sh` copies `steamclient.so` from the
Steam snap's data area into our own `$SNAP_USER_COMMON`. dlopen and mmap are
both permitted there, so the library loads normally. `steamclient.so`
multi-process IPC with running Steam continues to work even when loaded from
a non-canonical path; verified end-to-end with `list-achievements`, which
exercises both the orchestrator and the per-app-server child loading the
library concurrently.

`SAM_STEAMCLIENT_PATH` is set to the copy (so the loader uses it) and
`SAM_STEAM_INSTALL_ROOT` is set to the canonical Steam path (so the locator
still finds the achievement-schema files exposed through `personal-files`).
The install-root gate in `loaded_install_is_running` resolves the canonical
root via `SteamLocator::get_local_steam_install_root_folders` rather than
deriving it from the steamclient path, which keeps the check correct when
the dlopen target is a copy.

## Personal-files paths

Four paths are granted read access — every read site the code reaches:

- `linux64/steamclient.so` — for the startup copy.
- `appcache/stats` — `UserGameStatsSchema_<id>.bin` (achievement definitions)
  and `UserGameStats_<account>_<id>.bin` (user stat values).
- `appcache/librarycache` — locally cached banner images used by the GUI
  variants when rendering the app list.
- `userdata` — `userdata/<account>/config/localconfig.vdf` is parsed for
  per-app playtime. `personal-files` paths don't support globs, so we grant
  the parent directory.

## Apps shipped

The snap ships three executables, each invoked through the same launcher:

- `samrewritten` — default GTK GUI (`snap run samrewritten`).
- `samrewritten-adw` — Adwaita-styled GUI (`snap run samrewritten.samrewritten-adw`).
- `samrewritten-cli` — command-line interface (`snap run samrewritten.samrewritten-cli`).

## Caveats

Only the Steam snap is supported as the Steam install. Adding Flatpak or
distro-package Steam would require additional `personal-files` entries and
likely store-review pushback. The snap target currently only ships the
snap-Steam path.
