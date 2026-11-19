#!/bin/sh
# The packages whose tests CI runs on Windows and Linux, as `-p` arguments: those whose manifest
# says they are a platform (`[package.metadata.waft] platform = true`), `waft-embedder`, where the
# system's fonts meet the framework, and every example. The other crates switch on no target, so
# macOS's run of the whole workspace answers for them. Packages named as arguments are left out.
#   .github/platform-test-packages.sh [package-to-leave-out...]
left_out=$(printf '%s\n' "$@" | jq -R . | jq -s .)
packages=$(cargo metadata --no-deps --format-version 1 | jq -r --argjson left_out "$left_out" '
  .packages[]
  | select(.metadata.waft.platform == true
      or .name == "waft-embedder"
      or (.manifest_path | test("[/\\\\]examples[/\\\\]")))
  | select(.name as $name | $left_out | index($name) | not)
  | "-p " + .name' |
  tr -d '\r')  # jq on Windows ends its lines with CRLF, which cargo reads as part of a name
# `cargo test` with no `-p` tests the whole workspace, so finding nothing is a failure.
[ -n "$packages" ] || { echo "no package to test: is jq installed?" >&2; exit 1; }
echo "$packages"
