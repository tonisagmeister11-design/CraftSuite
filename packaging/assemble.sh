#!/usr/bin/env bash
# Assembles the CraftSuite folder from already-built binaries:
#
#   CraftSuite/
#     CraftSuite(.exe)               the start window
#     FilmCraft/filmcraft(.exe)      FilmCraft, unchanged, with its command-line tool
#     EffectCraft/effectcraft(.exe)  EffectCraft, unchanged, with its command-line tool
#     LIESMICH.txt, licences
#
# Usage: packaging/assemble.sh <out-dir> <launcher-bin> <filmcraft-bin-dir> <effectcraft-bin-dir> [exe-suffix]
# Run from the repository root (needs vendor/ checked out for the licence files).
set -euo pipefail

out=$1 launcher=$2 film_bin=$3 effect_bin=$4 ext=${5:-}
root=$(cd "$(dirname "$0")/.." && pwd)

rm -rf "$out"
mkdir -p "$out/FilmCraft" "$out/EffectCraft"

cp "$launcher" "$out/CraftSuite$ext"
cp "$film_bin/filmcraft$ext" "$film_bin/filmcraft-cli$ext" "$out/FilmCraft/"
cp "$effect_bin/effectcraft$ext" "$effect_bin/effectcraft-cli$ext" "$out/EffectCraft/"

# Each app's licences, notices and the licences of the fonts built into it.
for app in filmcraft:FilmCraft effectcraft:EffectCraft; do
  src="$root/vendor/${app%%:*}" dst="$out/${app##*:}"
  for f in LICENSE-MIT LICENSE-APACHE NOTICE ATTRIBUTION.md README.md; do
    if [[ -f "$src/$f" ]]; then cp "$src/$f" "$dst/"; fi
  done
  for lic in "$src"/assets/fonts/OFL-*.txt; do
    if [[ -f "$lic" ]]; then cp "$lic" "$dst/"; fi
  done
done
# Licences of the craft-fonts fonts built into FilmCraft (CRAFT_FONTS_DIR), as OFL-<family>.txt.
if [[ -n "${CRAFT_FONTS_DIR:-}" ]]; then
  for lic in "$CRAFT_FONTS_DIR"/fonts/*/OFL.txt; do
    if [[ -f "$lic" ]]; then cp "$lic" "$out/FilmCraft/OFL-$(basename "$(dirname "$lic")").txt"; fi
  done
fi
cp "$root/LICENSE-MIT" "$root/LICENSE-APACHE" "$out/"
cp "$root/packaging/LIESMICH.txt" "$out/"
echo "assembled $out"
