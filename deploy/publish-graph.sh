#!/usr/bin/env bash
# Upload a graph artifact to R2 and select it for deployment by rewriting
# deploy/graph.json. Commit that file; merging it into main deploys the graph.
#
#   deploy/publish-graph.sh [artifact-dir]
#
# The default is the artifact selected by data/products/etymology_graph/CURRENT.json.
# Needs rclone with a remote named by WETY_R2_REMOTE (default r2) and jq.
set -euo pipefail

repo=$(cd "$(dirname "$0")/.." && pwd)
remote=${WETY_R2_REMOTE:-r2}
bucket=${WETY_R2_BUCKET:-wety}
pointer=$repo/data/products/etymology_graph/CURRENT.json

if [ $# -gt 0 ]; then
    artifact=$(cd "$1" && pwd)
else
    artifact=$repo/$(jq -er '.artifact' "$pointer")
fi
manifest=$artifact/manifest.json
[ -f "$manifest" ] || {
    echo "no manifest.json in $artifact" >&2
    exit 1
}
# A graph artifact holds one file, its entrypoint.
[ "$(jq '.files | length' "$manifest")" = 1 ] || {
    echo "expected exactly one file in $manifest" >&2
    exit 1
}
file=$(jq -er '.files | keys[0]' "$manifest")
sha=$(jq -er --arg f "$file" '.files[$f].sha256' "$manifest")
bytes=$(jq -er --arg f "$file" '.files[$f].bytes' "$manifest")
legacy=$(jq '.provenance.status == "legacy_adoption"' "$manifest")
object=graphs/$sha/$file

echo "checking $artifact/files/$file against its manifest"
[ "$(shasum -a 256 "$artifact/files/$file" | cut -d' ' -f1)" = "$sha" ] || {
    echo "checksum mismatch: $artifact/files/$file" >&2
    exit 1
}

if [ "$(rclone size --json "$remote:$bucket/$object" 2>/dev/null | jq '.bytes')" = "$bytes" ]; then
    echo "$object already in R2"
else
    echo "uploading $object ($bytes bytes)"
    rclone copyto --progress "$artifact/files/$file" "$remote:$bucket/$object"
fi

jq -n --arg artifact_id "$(jq -er '.artifact_id' "$manifest")" --arg object "$object" \
    --arg file "$file" --arg sha "$sha" --argjson bytes "$bytes" --argjson legacy "$legacy" \
    '{schema_version: 1, artifact_id: $artifact_id, object: $object, file: $file,
      sha256: $sha, bytes: $bytes, allow_legacy: $legacy}' >"$repo/deploy/graph.json"
echo "wrote deploy/graph.json; commit it to deploy this graph"
