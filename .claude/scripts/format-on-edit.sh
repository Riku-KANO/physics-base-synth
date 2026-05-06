#!/usr/bin/env bash
# PostToolUse hook: Edit / Write / MultiEdit 後に対象ファイルを自動 format。
# stdin に Claude Code から { tool_input: { file_path: "..." } } の JSON が来る。
# 失敗しても Claude をブロックしないため最後は exit 0。

set +e

PROJECT_ROOT="$(cd "$(dirname "$0")/../.." && pwd)"

# Node.js で stdin JSON から file_path を抽出（jq 非依存）
raw=$(node -e "let d='';process.stdin.on('data',c=>d+=c);process.stdin.on('end',()=>{try{process.stdout.write((JSON.parse(d).tool_input?.file_path)||'')}catch{}})")
[ -z "$raw" ] && exit 0

# Windows path (C:\...) -> forward slash に正規化
norm="${raw//\\//}"

case "$norm" in
	*/static/worklet/*) exit 0 ;;
	*.rs)
		cd "$PROJECT_ROOT" && cargo fmt -- "$raw" >/dev/null 2>&1
		;;
	*.ts | *.svelte | *.js | *.tsx)
		case "$norm" in
			*/web/*)
				cd "$PROJECT_ROOT" && pnpm --filter ./web exec prettier --write --log-level=warn "$raw" >/dev/null 2>&1
				;;
		esac
		;;
esac

exit 0
