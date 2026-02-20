#!/usr/bin/env bash
# Prevent monolithic files from being committed.
# Checks every staged .rs / .ts / .tsx file against per-type line limits.
# Test files are exempt — they grow with coverage and that's fine.
# For Rust files with inline #[cfg(test)] modules, only production lines
# (before the first top-level #[cfg(test)] marker) count toward the limit.
set -euo pipefail

# ── Thresholds (lines) ────────────────────────────────────────────────────────
MAX_RS=300   # Rust source modules (production lines only; inline tests excluded)
MAX_TS=300   # TypeScript source files

is_test_file() {
    local f="$1"
    # Rust: tests/ directory, *_test.rs, or tests.rs module files
    [[ "$f" =~ (/tests?/|_test\.rs$|/tests\.rs$) ]] && return 0
    # TypeScript: *.test.ts(x), *.spec.ts(x), __tests__/
    [[ "$f" =~ (\.(test|spec)\.(ts|tsx)$|/__tests__/) ]] && return 0
    return 1
}

# Count production lines in a .rs file: lines before the first top-level
# #[cfg(test)] annotation. If no such marker exists, count all lines.
rs_production_lines() {
    local f="$1"
    local test_line
    test_line=$(grep -n '^#\[cfg(test)\]' "$f" | head -1 | cut -d: -f1 || true)
    if [[ -n "$test_line" ]]; then
        echo $(( test_line - 1 ))
    else
        wc -l < "$f"
    fi
}

# ── Check staged files ────────────────────────────────────────────────────────
violations=()

while IFS= read -r file; do
    [[ -f "$file" ]] || continue
    is_test_file "$file" && continue

    case "$file" in
        *.rs)
            lines=$(rs_production_lines "$file")
            limit=$MAX_RS
            ;;
        *.ts|*.tsx)
            lines=$(wc -l < "$file")
            limit=$MAX_TS
            ;;
        *) continue ;;
    esac

    if (( lines > limit )); then
        violations+=("  ${file}: ${lines} lines (limit: ${limit})")
    fi
done < <(git diff --cached --name-only --diff-filter=ACM)

# ── Report ────────────────────────────────────────────────────────────────────
if (( ${#violations[@]} > 0 )); then
    echo "" >&2
    echo "Monolithic file(s) detected — split into focused modules:" >&2
    printf '%s\n' "${violations[@]}" >&2
    echo "" >&2
    echo "Limits: .rs=${MAX_RS} production lines (inline tests excluded), .ts(x)=${MAX_TS} lines (test files exempt)" >&2
    echo "" >&2
    exit 1
fi
