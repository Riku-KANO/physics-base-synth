#!/usr/bin/env bash
# Stop hook: ターン終了時に lint check。失敗しても Claude をブロックせず、
# systemMessage で警告を表示するのみ（exit 0 で常に終了）。

set +e

PROJECT_ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
cd "$PROJECT_ROOT" || exit 0

failures=()

if ! cargo clippy --workspace --all-targets -- -D warnings >/tmp/physics-synth-clippy.log 2>&1; then
	failures+=("cargo clippy")
fi

if ! pnpm --filter ./web lint >/tmp/physics-synth-pnpm-lint.log 2>&1; then
	failures+=("pnpm --filter ./web lint")
fi

if [ ${#failures[@]} -gt 0 ]; then
	IFS=', '
	msg="Lint check failed: ${failures[*]}. See /tmp/physics-synth-clippy.log and /tmp/physics-synth-pnpm-lint.log."
	node -e "process.stdout.write(JSON.stringify({systemMessage: process.argv[1]}))" "$msg"
fi

exit 0
