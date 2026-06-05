#!/bin/sh
SRC=/usr/share/icons/Adwaita
DST=assets/icons/Adwaita

mkdir -p "$DST/symbolic/actions" "$DST/symbolic/ui" "$DST/symbolic/categories" "$DST/symbolic/status"

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

Directories=symbolic/actions,symbolic/ui,symbolic/categories,symbolic/status

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

[symbolic/categories]
Context=Categories
Size=16
MinSize=8
MaxSize=512
Type=Scalable

[symbolic/status]
Context=Status
Size=16
MinSize=8
MaxSize=512
Type=Scalable
EOF

cp -f "$SRC/symbolic/actions/action-unavailable-symbolic.svg" \
      "$SRC/symbolic/actions/document-edit-symbolic.svg" \
      "$SRC/symbolic/actions/edit-find-symbolic.svg" \
      "$SRC/symbolic/actions/go-previous-symbolic.svg" \
      "$SRC/symbolic/actions/go-previous-symbolic-rtl.svg" \
      "$SRC/symbolic/actions/go-up-symbolic.svg" \
      "$SRC/symbolic/actions/list-add-symbolic.svg" \
      "$SRC/symbolic/actions/media-playback-start-symbolic.svg" \
      "$SRC/symbolic/actions/object-select-symbolic.svg" \
      "$SRC/symbolic/actions/open-menu-symbolic.svg" \
      "$SRC/symbolic/actions/system-search-symbolic.svg" \
      "$DST/symbolic/actions/"

cp -f "$SRC/symbolic/ui/pan-down-symbolic.svg" \
      "$SRC/symbolic/ui/window-new-symbolic.svg" \
      "$SRC/symbolic/ui/window-close-symbolic.svg" \
      "$DST/symbolic/ui/"

cp -f "$SRC/symbolic/categories/emoji-recent-symbolic.svg" "$DST/symbolic/categories/"

cp -f "$SRC/symbolic/status/avatar-default-symbolic.svg" \
      "$SRC/symbolic/status/dialog-error-symbolic.svg" \
      "$DST/symbolic/status/"

exec "$@"