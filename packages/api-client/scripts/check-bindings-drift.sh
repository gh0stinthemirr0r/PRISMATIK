#!/usr/bin/env sh
set -eu

generated_dir="packages/api-client/src/generated"
marker="$generated_dir/.gitkeep"

if [ ! -e "$marker" ]; then
  echo "Missing generated bindings marker: $marker" >&2
  exit 1
fi

if [ -n "$(git status --porcelain -- "$generated_dir")" ]; then
  echo "Generated API bindings differ from the committed tree." >&2
  echo "Regenerate them with tauri-specta and commit the result." >&2
  exit 1
fi

echo "Generated API bindings are clean."
