#!/bin/sh
SRC=/usr/share/icons/Adwaita
DST=assets/icons/Adwaita

if [ ! -d "$DST" ]; then
    mkdir -p "$DST/symbolic/actions" "$DST/symbolic/ui"

    cat <<EOF > "$DST/index.theme"
[Icon Theme]
Name=Adwaita
Comment=The Only One trimmed for SamRewritten
Example=folder
Inherits=AdwaitaLegacy,hicolor
Hidden=true

DisplayDepth=32
LinkOverlay=link_overlay
LockOverlay=lock_overlay
ZipOverlay=zip_overlay
DesktopDefault=48
DesktopSizes=16,22,32,48,64,72,96,128
ToolbarDefault=22
ToolbarSizes=16,22,32,48
MainToolbarDefault=22
MainToolbarSizes=16,22,32,48
SmallDefault=16
SmallSizes=16
PanelDefault=32
PanelSizes=16,22,32,48,64,72,96,128

Directories=symbolic/actions,symbolic/ui

[symbolic/actions]
Context=Actions
Size=16
MinSize=8
MaxSize=512
Type=Scalable

[symbolic/ui]
Context=UI
Size=16
MinSize=8
MaxSize=512
Type=Scalable
EOF

    cp "$SRC/symbolic/actions/action-unavailable-symbolic.svg" \
       "$SRC/symbolic/actions/document-edit-symbolic.svg" \
       "$SRC/symbolic/actions/edit-find-symbolic.svg" \
       "$SRC/symbolic/actions/go-previous-symbolic.svg" \
       "$SRC/symbolic/actions/go-previous-symbolic-rtl.svg" \
       "$SRC/symbolic/actions/go-up-symbolic.svg" \
       "$SRC/symbolic/actions/media-playback-start-symbolic.svg" \
       "$SRC/symbolic/actions/open-menu-symbolic.svg" \
       "$DST/symbolic/actions/"

    cp "$SRC/symbolic/ui/window-new-symbolic.svg" "$DST/symbolic/ui/"
fi

exec "$@"