# Packaging SamRewritten as a Flatpak

Short version: SamRewritten itself is unlikely to ship as a Flathub Flatpak in
the foreseeable future. The reason is the same PID-namespace problem that
`src/utils/steam_ns.rs` solves for the *other* direction (host SamRewritten
talking to Flatpak Steam) — except the workaround is not available to a
sandboxed application.

## Why it doesn't work

Steam's IPC tracks each connected client's liveness by PID, observed from
Steam's own PID namespace. When the client lives in a different PID namespace
(e.g. a Flatpak), the PID Steam sees is meaningless to it — connections get
reaped mid-call, app-servers look like they never exit (Steam keeps reporting
the user as in-game), and the broken-pipe symptom you see in #10 follows.

On the host we work around this by joining Steam's user+PID namespace before
forking app-servers (see `src/utils/steam_ns.rs`). That works because the host
has unrestricted access to `setns(2)` / `pidfd_open(2)` and the target user
namespace is owned by our uid.

A Flatpak'd SamRewritten cannot do the same trick from inside its sandbox:

1. **Seccomp.** Flatpak's default filter blocks `setns` and `unshare` of
   non-user namespaces. Unblocking them requires `--allow=devel`, which
   Flathub does not accept for end-user apps.
2. **`/proc` visibility.** `detect_flatpak_steam` walks the host `/proc` to
   find the Steam process. Inside a Flatpak the visible `/proc` is the
   sandbox's, and there is no supported permission to expose the host `/proc`.
3. **Cross-sandbox namespace entry.** Even with the syscalls unblocked,
   joining a sibling Flatpak's user namespace from inside your own is fragile
   and not guaranteed to be permitted by the kernel.
4. **Steam install paths.** `SteamLocator` reads host paths
   (`~/.steam/...`, `~/.var/app/com.valvesoftware.Steam/...`). Reaching those
   from inside a sandbox needs broad filesystem permissions that Flathub
   reviewers reject.

Removing any one of these is hard; removing all four is what a clean Flathub
submission would require.

## What has been considered

- **`flatpak-spawn --host` shim** — keep the GTK frontend inside the sandbox
  and run the orchestrator on the host. Sidesteps the namespace problem, but
  needs `--talk-name=org.freedesktop.Flatpak` (also a Flathub red flag) and
  still requires the orchestrator binary to exist on the host, which defeats
  the "install via Flathub" benefit.
- **Manual `.flatpak` bundle outside Flathub** with whichever permissions are
  actually needed (`--allow=devel`, host `/proc` mount, etc.). Technically
  feasible but loses the discoverability that motivates the request.
