#!/usr/bin/env bash
# Remove old releases and unused graphs from the wety-deploy R2 bucket. The deploy
# workflow runs it with --apply after each successful server deploy; without --apply
# it only lists what it would remove. See deploy/README.md.
#
#   deploy/prune-r2.sh [--apply]
#
# Keeps the newest KEEP_RELEASES releases (by upload time), the graphs they name, the
# graph deploy/graph.json names, and any graph uploaded in the last GRAPH_GRACE_DAYS
# days (published for a change not merged yet). The host keeps its own copies of the
# releases it can roll back to. Needs rclone with a remote named by WETY_R2_REMOTE
# (default wety-deploy; r2 in CI) and jq.
set -euo pipefail

repo=$(cd "$(dirname "$0")/.." && pwd)
remote=${WETY_R2_REMOTE:-wety-deploy}
bucket=${WETY_R2_BUCKET:-wety-deploy}
keep_releases=${KEEP_RELEASES:-5}
grace_days=${GRAPH_GRACE_DAYS:-30}
apply=false
[ "${1:-}" = --apply ] && apply=true
base=$remote:$bucket

remove() {
    if $apply; then
        echo "removing $1"
        rclone purge "$base/$1"
    else
        echo "would remove $1"
    fi
}

# Releases, newest first. A release.json is uploaded last, so it marks a complete one.
releases=$(rclone lsjson -R --files-only --use-server-modtime --include '/*/release.json' "$base/releases/" |
    jq -r 'sort_by(.ModTime) | reverse | .[].Path | split("/")[0]')
kept=$(head -n "$keep_releases" <<<"$releases")

keep_graphs=" $(jq -er '.sha256' "$repo/deploy/graph.json") "
for commit in $kept; do
    keep_graphs+="$(rclone cat "$base/releases/$commit/release.json" | jq -er '.graph.sha256') "
done

for commit in $(tail -n +"$((keep_releases + 1))" <<<"$releases"); do
    remove "releases/$commit"
done
# Prefixes without a release.json are uploads that did not finish.
for dir in $(rclone lsf --dirs-only "$base/releases/" | tr -d /); do
    grep -qx "$dir" <<<"$releases" || remove "releases/$dir"
done

# Graphs uploaded more than grace_days ago (server time, not the file's own mtime).
for sha in $(rclone lsf -R --files-only --use-server-modtime --min-age "${grace_days}d" "$base/graphs/" |
    cut -d/ -f1 | sort -u); do
    case $keep_graphs in *" $sha "*) continue ;; esac
    remove "graphs/$sha"
done
