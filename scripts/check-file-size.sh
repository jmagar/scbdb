#!/usr/bin/env bash
# Prevent monolithic files from being committed.
# Checks every staged .rs / .ts / .tsx file against per-type line limits.
# Test files are exempt — they grow with coverage and that's fine.
set -euo pipefail

# ── Thresholds (lines) ────────────────────────────────────────────────────────
MAX_RS=300   # Rust source modules
MAX_TS=300   # TypeScript source files

is_test_file() {
    local f="$1"
    # Rust: tests/ directory or *_test.rs
    [[ "$f" =~ (/tests?/|_test\.rs$) ]] && return 0
    # TypeScript: *.test.ts(x), *.spec.ts(x), __tests__/
    [[ "$f" =~ (\.(test|spec)\.(ts|tsx)$|/__tests__/) ]] && return 0
    return 1
}

# ── Check staged files ────────────────────────────────────────────────────────
violations=()

while IFS= read -r file; do
    [[ -f "$file" ]] || continue
    is_test_file "$file" && continue

    case "$file" in
        *.rs)  limit=$MAX_RS ;;
        *.ts|*.tsx) limit=$MAX_TS ;;
        *) continue ;;
    esac

    lines=$(wc -l < "$file")
    if (( lines > limit )); then
        violations+=("  ${file}: ${lines} lines (limit: ${limit})")
    fi
done < <(git diff --cached --name-only --diff-filter=ACM)

# ── Report ────────────────────────────────────────────────────────────────────
if (( ${#violations[@]} > 0 )); then
    echo "" >&2
    echo "Monolithic file(s) detected — split into smaller modules:" >&2
    printf '%s\n' "${violations[@]}" >&2
    echo "" >&2
    echo "Limits: .rs=${MAX_RS} lines, .ts(x)=${MAX_TS} lines (test files exempt)" >&2
    echo "" >&2
    exit 1
fi
