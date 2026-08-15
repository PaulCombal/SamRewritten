#!/usr/bin/env bash
# Regenerate po/samrewritten.pot from the sources listed in POTFILES.in.
#
# The po/<lang>.po files belong to Weblate, which merges the new template in
# itself; merging here too would have both sides rewriting the same files, and
# every push would conflict. Pass --merge to do it locally anyway, once Weblate
# is locked (wlc lock).
#
# Keywords:
#   tr       -> runtime translation
#   tr_noop  -> deferred extraction (translated later via tr)
#   trn:1,2  -> plural form (singular, plural)
set -euo pipefail
cd "$(dirname "$0")/.."

merge=false
for arg in "$@"; do
  case "$arg" in
    --merge) merge=true ;;
    -h | --help)
      echo "usage: ${0##*/} [--merge]"
      exit 0
      ;;
    *)
      echo "${0##*/}: unknown argument '$arg'" >&2
      echo "usage: ${0##*/} [--merge]" >&2
      exit 2
      ;;
  esac
done

xgettext \
  --from-code=UTF-8 \
  --language=C \
  --keyword=tr \
  --keyword=tr_noop \
  --keyword=trn:1,2 \
  --add-comments=TRANSLATORS \
  --package-name=SamRewritten \
  --files-from=po/POTFILES.in \
  --output=po/samrewritten.pot

echo "Wrote po/samrewritten.pot"

if [ "$merge" = false ]; then
  echo "Left po/*.po untouched; Weblate merges them (pass --merge to do it here)."
  exit 0
fi

for po in po/*.po; do
  [ -e "$po" ] || continue
  echo "Merging into $po"
  msgmerge --update --backup=none "$po" po/samrewritten.pot
done
