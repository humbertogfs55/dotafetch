#!/usr/bin/env bash
# Downloads Liquipedia's "Dota 2 hero minimap icons" into reference/ for use
# as tracing reference when drawing the nerd-font icon pack - NOT for
# redistribution. reference/ is gitignored; these stay local.
#
# Uses Liquipedia's MediaWiki API (not HTML scraping): one categorymembers
# call lists every file in the category, then imageinfo calls (batched 50
# titles at a time, the API's per-request limit) resolve each to its direct
# image URL. Safe to re-run after new heroes ship - it just re-downloads.
set -euo pipefail

CATEGORY="Category:Dota_2_hero_minimap_icons"
OUT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)/reference/hero_minimap_icons"
API="https://liquipedia.net/commons/api.php"
UA="dotafetch-reference-icon-fetcher/0.1 (personal non-commercial project; https://github.com)"

command -v jq >/dev/null || { echo "fetch_reference_icons.sh: jq is required" >&2; exit 1; }

mkdir -p "$OUT_DIR"
tmp="$(mktemp -d)"
trap 'rm -rf "$tmp"' EXIT

echo "Listing $CATEGORY members..."
titles_file="$tmp/titles.txt"
: > "$titles_file"
cmcontinue=""
while :; do
    url="$API?action=query&list=categorymembers&cmtitle=$CATEGORY&cmlimit=500&format=json"
    [ -n "$cmcontinue" ] && url="$url&cmcontinue=$cmcontinue"
    resp="$(curl -sS --compressed -A "$UA" "$url")"
    echo "$resp" | jq -r '.query.categorymembers[].title' >> "$titles_file"
    cmcontinue="$(echo "$resp" | jq -r '.continue.cmcontinue // empty')"
    [ -z "$cmcontinue" ] && break
done
total=$(wc -l < "$titles_file")
echo "found $total icons"

echo "Resolving direct URLs..."
urls_file="$tmp/urls.tsv"
: > "$urls_file"
split -l 50 "$titles_file" "$tmp/chunk_"
for chunk in "$tmp"/chunk_*; do
    joined="$(paste -sd'|' "$chunk")"
    curl -sS --compressed -A "$UA" \
        --data-urlencode "action=query" \
        --data-urlencode "titles=$joined" \
        --data-urlencode "prop=imageinfo" \
        --data-urlencode "iiprop=url" \
        --data-urlencode "format=json" \
        "$API" \
        | jq -r '.query.pages[] | select(.imageinfo) | "\(.title)\t\(.imageinfo[0].url)"' \
        >> "$urls_file"
    sleep 1
done

echo "Downloading into $OUT_DIR..."
ok=0
fail=0
while IFS=$'\t' read -r title img_url; do
    fname="$(basename "$img_url")"
    if curl -sS -A "$UA" -o "$OUT_DIR/$fname" "$img_url" && [ -s "$OUT_DIR/$fname" ]; then
        ok=$((ok + 1))
    else
        fail=$((fail + 1))
        echo "  failed: $title" >&2
    fi
    sleep 0.3
done < "$urls_file"

echo "done: $ok downloaded, $fail failed"
