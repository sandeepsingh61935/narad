#!/bin/sh
# narad-commit-check
# One closed slice per commit. Conventional subject. Multi-area dumps need
# a body trailer: Bulk-commit: approved
set -eu

usage() {
    echo "usage: check-commit.sh [--msg-file FILE] [--files-from FILE] [--selftest]" >&2
    exit 2
}

MSG_FILE=""
FILES_FROM=""
SELFTEST=0

while [ $# -gt 0 ]; do
    case "$1" in
        --msg-file) MSG_FILE="${2:-}"; shift 2 ;;
        --files-from) FILES_FROM="${2:-}"; shift 2 ;;
        --selftest) SELFTEST=1; shift ;;
        -h|--help) usage ;;
        *) usage ;;
    esac
done

# Map a path to a commit unit. meta/docs/workspace are non-product.
unit_of() {
    p=$1
    case "$p" in
        crates/*)
            crate=${p#crates/}
            crate=${crate%%/*}
            echo "crate:$crate"
            ;;
        dashboard/*|dashboard) echo dashboard ;;
        extension/*|extension) echo extension ;;
        packaging/*|packaging) echo packaging ;;
        spikes/README.md|spikes) echo meta ;;
        spikes/*)
            spike=${p#spikes/}
            spike=${spike%%/*}
            echo "spike:$spike"
            ;;
        docs/*) echo docs ;;
        scripts/*) echo meta ;;
        .cursor/*|.pi/*|openspec/*) echo meta ;;
        AGENTS.md|CLAUDE.md|.gitignore|.gitattributes|PRODUCT.md) echo meta ;;
        Cargo.toml|Cargo.lock) echo workspace ;;
        *)
            root=${p%%/*}
            if [ "$root" = "$p" ]; then
                echo meta
            else
                echo "other:$root"
            fi
            ;;
    esac
}

product_units_from_list() {
    list=$1
    products=""
    has_crate=0
    while IFS= read -r f || [ -n "$f" ]; do
        [ -z "$f" ] && continue
        u=$(unit_of "$f")
        case "$u" in
            meta|docs) ;;
            workspace) ;;
            crate:*)
                has_crate=1
                case " $products " in
                    *" $u "*) ;;
                    *) products="$products $u" ;;
                esac
                ;;
            *)
                case " $products " in
                    *" $u "*) ;;
                    *) products="$products $u" ;;
                esac
                ;;
        esac
    done <<EOF
$list
EOF
    # shellcheck disable=SC2086
    set -- $products
    echo $#
    echo "$products"
}

subject_ok() {
    subj=$1
    echo "$subj" | grep -Eq '^(feat|fix|docs|chore|test|refactor|perf|ci|build|revert)(\([a-z0-9/_-]+\))?(!)?: [^ ].+'
}

bulk_approved() {
    msg=$1
    echo "$msg" | grep -Eq '(^|[^[:alnum:]-])Bulk-commit: approved([[:space:]]|$)'
}

check() {
    msg=$1
    files=$2
    subj=$(printf '%s\n' "$msg" | head -n 1)
    # strip comment lines git may leave
    subj=$(printf '%s' "$subj" | sed 's/^#.*//')

    if [ -z "$subj" ]; then
        echo "narad-commit-check: empty subject" >&2
        return 1
    fi
    if ! subject_ok "$subj"; then
        echo "narad-commit-check: subject must be conventional:" >&2
        echo "  type(scope)?: summary" >&2
        echo "  types: feat fix docs chore test refactor perf ci build revert" >&2
        echo "  got: $subj" >&2
        return 1
    fi

    counted=$(product_units_from_list "$files")
    n=$(printf '%s\n' "$counted" | head -n 1)
    units=$(printf '%s\n' "$counted" | tail -n +2)

    if [ "$n" -gt 1 ]; then
        if bulk_approved "$msg" || [ "${NARAD_ALLOW_BULK_COMMIT:-}" = "1" ]; then
            echo "narad-commit-check: bulk dump allowed ($n product areas:$units)" >&2
            return 0
        fi
        echo "narad-commit-check: $n product areas in one commit:$units" >&2
        echo "  One closed slice per commit (one crate | dashboard | extension | packaging | one spike," >&2
        echo "  plus docs/meta). Split the commit, or add a body trailer:" >&2
        echo "    Bulk-commit: approved" >&2
        return 1
    fi
    return 0
}

if [ "$SELFTEST" -eq 1 ]; then
    fail=0
    subject_ok "feat(naradd): hold-to-talk hotkey" || fail=1
    subject_ok "chore: wire thin agent contract" || fail=1
    subject_ok "WIP stuff" && fail=1
    n=$(product_units_from_list "crates/naradd/src/main.rs
crates/naradd/src/hotkeys.rs
docs/REQUIREMENTS.md
AGENTS.md" | head -n 1)
    [ "$n" -eq 1 ] || fail=1
    n=$(product_units_from_list "crates/naradd/src/main.rs
dashboard/src/routes/+page.svelte
extension/extension.js
spikes/hotkeys/src/main.rs" | head -n 1)
    [ "$n" -eq 4 ] || fail=1
    check "feat: Phase 0/1 MVP — everything
" "crates/naradd/src/main.rs
dashboard/package.json
extension/extension.js" && fail=1
    check "feat: Phase 0/1 MVP

Bulk-commit: approved
" "crates/naradd/src/main.rs
dashboard/package.json" || fail=1
    if [ "$fail" -ne 0 ]; then
        echo "narad-commit-check: selftest FAILED" >&2
        exit 1
    fi
    echo "narad-commit-check: selftest ok"
    exit 0
fi

if [ -z "$MSG_FILE" ]; then
    echo "narad-commit-check: --msg-file required (or --selftest)" >&2
    exit 2
fi

msg=$(cat "$MSG_FILE")
if [ -n "$FILES_FROM" ]; then
    files=$(cat "$FILES_FROM")
else
    files=$(git diff --cached --name-only --diff-filter=ACMR)
fi

check "$msg" "$files"
