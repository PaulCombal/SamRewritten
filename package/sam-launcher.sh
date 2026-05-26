#!/bin/bash
# Snap command-chain launcher for SamRewritten.
#
# No personal-files access: the user grants their Steam folder via the XDG
# FileChooser portal (GTK_USE_PORTAL). That folder lives on a fuse.portal mount
# that refuses mmap(PROT_EXEC), so the GUI mirrors steamclient.so into
# $SNAP_USER_COMMON and we load it from there.

set -eu

export GTK_USE_PORTAL=1
export SAM_STEAMCLIENT_PATH="$SNAP_USER_COMMON/steamclient.so"

# The GUI persists the portal-granted root here; export it so the CLI (no picker)
# can reuse the grant once the GUI has run. The GUI re-pins it itself at runtime.
if [ -f "$SNAP_USER_COMMON/steam_root.txt" ]; then
    SAM_STEAM_INSTALL_ROOT="$(cat "$SNAP_USER_COMMON/steam_root.txt")"
    export SAM_STEAM_INSTALL_ROOT
fi

exec "$@"
