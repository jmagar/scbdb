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

# Count effective LOC (non-blank, non-comment) for a source file.
# If end_line > 0, only count up to that line (inclusive).
count_effective_loc() {
    local f="$1"
    local end_line="${2:-0}"
    awk -v end_line="$end_line" '
        BEGIN { count=0; in_block=0 }
        end_line > 0 && NR > end_line { exit }
        {
            line=$0
            sub(/^[[:space:]]+/, "", line)

            # Skip blank lines.
            if (line == "") next

            # Skip block-comment body lines until closing marker.
            if (in_block) {
                if (line ~ /\*\//) {
                    sub(/^.*\*\//, "", line)
                    sub(/^[[:space:]]+/, "", line)
                    in_block=0
                    if (line == "") next
                } else {
                    next
                }
            }

            # Skip line comments.
            if (line ~ /^\/\//) next

            # Handle block comment starts.
            if (line ~ /^\/\*/) {
                if (line ~ /\*\//) {
                    sub(/^\/\*.*\*\//, "", line)
                    sub(/^[[:space:]]+/, "", line)
                    if (line == "") next
                } else {
                    in_block=1
                    next
                }
            }

            count++
        }
        END { print count }
    ' "$f"
}

# Count production LOC in a .rs file:
# - Excludes inline test module lines (from top-level #[cfg(test)] onward)
# - Excludes comments and blanks
rs_production_lines() {
    local f="$1"
    local test_line
    local end_line=0
    test_line=$(grep -n '^#\[cfg(test)\]' "$f" | head -1 | cut -d: -f1 || true)
    if [[ -n "$test_line" ]]; then
        end_line=$(( test_line - 1 ))
    fi
    count_effective_loc "$f" "$end_line"
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
            lines=$(count_effective_loc "$file")
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
