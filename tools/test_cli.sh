#!/bin/sh
# Command-line regression tests for prefsuf.rs and certcheck.rs: exit
# statuses, the one-line verdict, and failures that must not crash.
#
#   sh tools/test_cli.sh        (run by tools/run_tests.sh as the "cli" step)
#
# Builds into $TMPDIR (default /tmp) like the programs themselves. Exits 0
# if every case passes, 1 otherwise.
set -u
here="$(cd "$(dirname "$0")/.." && pwd)"
work="$(mktemp -d "${TMPDIR:-/tmp}/prefsuf_cli.XXXXXX")" || exit 2
trap 'rm -rf "$work"' EXIT
export TMPDIR="$work"
cd "$here" || exit 2
passed=0
failed=0

# expect NAME WANTED_STATUS STDOUT_PREFIX COMMAND...   (prefix "" = not checked)
expect() {
    name="$1"; want="$2"; prefix="$3"; shift 3
    "$@" > "$work/out" 2> "$work/err"
    got=$?
    first="$(head -n 1 "$work/out")"
    if [ "$got" -ne "$want" ]; then
        echo "FAIL  $name: exit $got, expected $want"; sed 's/^/        /' "$work/err" | head -n 5
        failed=$((failed + 1))
    elif [ -n "$prefix" ] && [ "${first#"$prefix"}" = "$first" ]; then
        echo "FAIL  $name: first line '$first' does not start with '$prefix'"
        failed=$((failed + 1))
    elif grep -q 'panicked' "$work/err"; then
        echo "FAIL  $name: panicked"; head -n 3 "$work/err" | sed 's/^/        /'
        failed=$((failed + 1))
    else
        echo "ok    $name"
        passed=$((passed + 1))
    fi
}

bad="$(printf '\377')"   # one byte that is not valid UTF-8

# ---- the solver
expect "solver: one puzzle"                0 "The puzzle" sh prefsuf.rs 8 ab B5 Pab Sb
expect "solver: contradiction is exit 0"   0 "The puzzle" sh prefsuf.rs 7 ab B4 Pa Sb
expect "solver: --help"                    0 "How to use" sh prefsuf.rs --help
expect "solver: length not a number"       2 "" sh prefsuf.rs x ab
expect "solver: unknown rule"              2 "" sh prefsuf.rs 8 ab Q5
expect "solver: border longer than text"   2 "" sh prefsuf.rs 3 ab B4
expect "solver: non-UTF-8 argument"        2 "" sh prefsuf.rs 8 "$bad" B5
expect "solver: text too long for memory"  2 "" sh prefsuf.rs 18446744073709551615 ab
if [ -w /dev/full ]; then
    expect "solver: output cannot be written" 2 "" sh -c 'sh prefsuf.rs 8 ab B5 > /dev/full'
fi

# ---- the checker
sh prefsuf.rs --certificate 8 ab B5 Pab Sb > "$work/sat.txt" || { echo "FAIL  cannot make a certificate"; exit 1; }
sh prefsuf.rs --certificate 7 ab B4 Pa Sb > "$work/unsat.txt" || { echo "FAIL  cannot make a certificate"; exit 1; }
sed 's/^free 1$/free 0/' "$work/sat.txt" > "$work/forged.txt"
expect "checker: solvable, puzzle given"   0 "VALID: solvable" sh certcheck.rs "$work/sat.txt" 8 ab B5 Pab Sb
expect "checker: solvable, no puzzle"      0 "VALID: solvable" sh certcheck.rs "$work/sat.txt"
expect "checker: contradiction"            0 "VALID: contradiction" sh certcheck.rs "$work/unsat.txt" 7 ab B4 Pa Sb
expect "checker: standard input"           0 "VALID" sh -c 'sh certcheck.rs - < "$1"' sh "$work/unsat.txt"
expect "checker: forged free count"        1 "REJECTED" sh certcheck.rs "$work/forged.txt"
expect "checker: other puzzle"             1 "REJECTED" sh certcheck.rs "$work/sat.txt" 8 ab B5 Pab
expect "checker: missing file"             2 "" sh certcheck.rs "$work/none.txt"
expect "checker: no arguments"             2 "" sh certcheck.rs
expect "checker: LENGTH without LETTERS"   2 "" sh certcheck.rs "$work/sat.txt" 8
expect "checker: LENGTH not a number"      2 "" sh certcheck.rs "$work/sat.txt" x ab
expect "checker: unknown rule kind"        2 "" sh certcheck.rs "$work/sat.txt" 8 ab Q
expect "checker: bad puzzle, bad file"     2 "" sh certcheck.rs "$work/forged.txt" x ab
expect "checker: non-UTF-8 argument"       2 "" sh certcheck.rs "$work/sat.txt" 8 "$bad" B5
if [ -w /dev/full ]; then
    expect "checker: verdict cannot be written" 2 "" sh -c 'sh certcheck.rs "$1" > /dev/full' sh "$work/sat.txt"
fi
expect "checker: compiler cannot write"    2 "" env TMPDIR="$work/missing" sh certcheck.rs "$work/sat.txt"

echo
echo "cli: $passed passed, $failed failed"
[ "$failed" -eq 0 ]
