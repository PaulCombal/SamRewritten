#!/usr/bin/env bash
# Regenerate po/samrewritten.pot from the sources listed in POTFILES.in, then
# merge the new template into every existing po/<lang>.po.
#
# Keywords:
#   tr       -> runtime translation
#   tr_noop  -> deferred extraction (translated later via tr)
#   trn:1,2  -> plural form (singular, plural)
set -euo pipefail
cd "$(dirname "$0")/.."

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

for po in po/*.po; do
  [ -e "$po" ] || continue
  echo "Merging into $po"
  msgmerge --update --backup=none "$po" po/samrewritten.pot
done
