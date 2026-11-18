#!/bin/sh
# The packages whose tests CI runs on Windows and Linux, as `-p` arguments: those whose manifest
# says they are a platform (`[package.metadata.waft] platform = true`), `waft-embedder`, where the
# system's fonts meet the framework, and every example. The other crates switch on no target, so
# macOS's run of the whole workspace answers for them. Packages named as arguments are left out.
#   .github/platform-test-packages.sh [package-to-leave-out...]
left_out=$(printf '%s\n' "$@" | jq -R . | jq -s .)
cargo metadata --no-deps --format-version 1 | jq -r --argjson left_out "$left_out" '
  .packages[]
  | select(.metadata.waft.platform == true
      or .name == "waft-embedder"
      or (.manifest_path | test("[/\\\\]examples[/\\\\]")))
  | select(.name as $name | $left_out | index($name) | not)
  | "-p " + .name'
