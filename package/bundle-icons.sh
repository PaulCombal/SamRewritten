#!/bin/sh
# Bundle only the Adwaita icons used by the app into assets/icons
SRC=/usr/share/icons/Adwaita
DST=assets/icons/Adwaita

if [ ! -d "$DST" ]; then
    mkdir -p "$DST/symbolic/actions" "$DST/symbolic/ui"
    cp "$SRC/index.theme" "$DST/"
    cp "$SRC/symbolic/actions/action-unavailable-symbolic.svg" "$DST/symbolic/actions/"
    cp "$SRC/symbolic/actions/document-edit-symbolic.svg" "$DST/symbolic/actions/"
    cp "$SRC/symbolic/actions/edit-find-symbolic.svg" "$DST/symbolic/actions/"
    cp "$SRC/symbolic/actions/go-previous-symbolic.svg" "$DST/symbolic/actions/"
    cp "$SRC/symbolic/actions/go-previous-symbolic-rtl.svg" "$DST/symbolic/actions/"
    cp "$SRC/symbolic/actions/go-up-symbolic.svg" "$DST/symbolic/actions/"
    cp "$SRC/symbolic/actions/media-playback-start-symbolic.svg" "$DST/symbolic/actions/"
    cp "$SRC/symbolic/actions/open-menu-symbolic.svg" "$DST/symbolic/actions/"
    cp "$SRC/symbolic/ui/window-new-symbolic.svg" "$DST/symbolic/ui/"
fi

exec "$@"
