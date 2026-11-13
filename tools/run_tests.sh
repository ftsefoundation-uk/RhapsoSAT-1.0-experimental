#!/bin/sh
# Run the whole test suite so that it keeps going if the terminal disconnects.
#
#   tools/run_tests.sh start    start a detached run (refuses if one is running)
#   tools/run_tests.sh status   is it running? if finished: result and exit status
#   tools/run_tests.sh log      print the log of the latest run (add -f to follow)
#   tools/run_tests.sh wait     block until the current run finishes; exit with its status
#   tools/run_tests.sh stop     stop a running run
#   tools/run_tests.sh run      run in the foreground (what the detached job executes)
#
# Detaching uses tmux if it is installed (session "prefsuf-tests"), otherwise
# setsid + nohup. Either survives closing the terminal or losing the
# connection. Neither survives a reboot or the process being killed; after
# that, "status" reports the run as interrupted and "start" begins a new one.
#
# Everything is written under test-logs/ (ignored by git):
#   test-logs/run-<time>.log   full output       test-logs/latest.log -> newest log
#   test-logs/status           one line: RUNNING / PASSED / FAILED / STOPPED, with times
#   test-logs/exit_status      exit status of the last finished run (0 = all passed)
#   test-logs/lock/            exists while a run is active (holds its process id)
set -u
here="$(cd "$(dirname "$0")/.." && pwd)"
logs="$here/test-logs"
session="prefsuf-tests"
mkdir -p "$logs"

running_pid() {
    [ -f "$logs/lock/pid" ] || return 1
    pid="$(cat "$logs/lock/pid" 2>/dev/null)"
    [ -n "$pid" ] && kill -0 "$pid" 2>/dev/null && echo "$pid"
}

case "${1:-}" in
run)
    if ! mkdir "$logs/lock" 2>/dev/null; then
        if running_pid >/dev/null; then
            echo "a test run is already active (pid $(running_pid)); see: tools/run_tests.sh status" >&2
            exit 3
        fi
        echo "removing the lock of an interrupted run" >&2
        rm -rf "$logs/lock" && mkdir "$logs/lock" || exit 3
    fi
    echo $$ > "$logs/lock/pid"
    log="$logs/run-$(date +%Y%m%d-%H%M%S).log"
    ln -sfn "$(basename "$log")" "$logs/latest.log"
    rm -f "$logs/exit_status"
    started="$(date '+%Y-%m-%d %H:%M:%S')"
    echo "RUNNING since $started (pid $$, log $(basename "$log"))" > "$logs/status"
    trap 'echo "STOPPED at $(date "+%Y-%m-%d %H:%M:%S") (started $started)" > "$logs/status"; echo 130 > "$logs/exit_status"; rm -rf "$logs/lock"; exit 130' INT TERM HUP
    export TMPDIR="$logs/build"
    mkdir -p "$TMPDIR"
    cd "$here" || exit 2
    overall=0
    step() {
        name="$1"; shift
        echo "=== $name: $* ($(date '+%H:%M:%S'))"
        "$@"
        code=$?
        echo "=== $name: exit $code"
        [ $code -eq 0 ] || { overall=$code; failed="${failed:-} $name"; }
    }
    {
        echo "test run started $started on $(uname -n), $(rustc --version 2>&1)"
        echo "commit $(git rev-parse --short HEAD 2>/dev/null) on $(git branch --show-current 2>/dev/null); uncommitted files: $(git status --porcelain 2>/dev/null | wc -l)"
        step self-tests   sh prefsuf.rs
        step stress       sh hard.rs 1
        step adversarial  sh adversarial.rs check 1
        step proof-lab    sh proof_lab.rs check 1
        step theory-lab   sh theory_lab.rs check
        step grid-lab     sh grid_lab.rs check
        step reduce-lab   sh reduce_lab.rs check 1
        step cert-lab     sh cert_lab.rs check 1
        echo "=== finished $(date '+%Y-%m-%d %H:%M:%S'): overall exit $overall${failed:+ (failed:$failed)}"
    } > "$log" 2>&1
    # the self-tests print the demo after their own verdict; keep only the verdict lines handy
    if [ "$overall" -eq 0 ]; then word=PASSED; else word="FAILED (${failed# })"; fi
    echo "$word at $(date '+%Y-%m-%d %H:%M:%S') (started $started, log $(basename "$log"))" > "$logs/status"
    echo "$overall" > "$logs/exit_status"
    rm -rf "$logs/lock"
    exit "$overall"
    ;;
start)
    if running_pid >/dev/null; then
        echo "a test run is already active (pid $(running_pid)). Not starting another."
        echo "check it with: tools/run_tests.sh status"
        exit 3
    fi
    if command -v tmux >/dev/null 2>&1; then
        tmux kill-session -t "$session" 2>/dev/null
        tmux new-session -d -s "$session" "sh '$here/tools/run_tests.sh' run" || exit 2
        how="tmux session '$session' (attach with: tmux attach -t $session)"
    else
        setsid nohup sh "$here/tools/run_tests.sh" run >/dev/null 2>&1 &
        how="a detached nohup job"
    fi
    sleep 1
    echo "started in $how"
    echo "check with: tools/run_tests.sh status   |   tools/run_tests.sh log -f"
    ;;
status)
    if pid="$(running_pid)"; then
        cat "$logs/status"
        echo "progress:"; grep '^=== ' "$logs/latest.log" 2>/dev/null | sed 's/^/  /'
        exit 0
    fi
    if [ -d "$logs/lock" ]; then
        echo "INTERRUPTED: a run was started but its process is gone (reboot or kill)."
        echo "last recorded state: $(cat "$logs/status" 2>/dev/null)"
        echo "start again with: tools/run_tests.sh start"
        exit 4
    fi
    if [ -f "$logs/status" ]; then
        cat "$logs/status"
        echo "exit status: $(cat "$logs/exit_status" 2>/dev/null || echo unknown)"
        grep -E '^=== .*: exit |passed, .* failed|^failures:|^total failures:|theory checks failed|^seed [0-9]+:' "$logs/latest.log" 2>/dev/null | sed 's/^/  /'
        exit "$(cat "$logs/exit_status" 2>/dev/null || echo 1)"
    fi
    echo "no test run has been started yet (tools/run_tests.sh start)"
    exit 5
    ;;
log)
    [ -e "$logs/latest.log" ] || { echo "no log yet"; exit 5; }
    if [ "${2:-}" = "-f" ]; then tail -n 40 -f "$logs/latest.log"; else cat "$logs/latest.log"; fi
    ;;
wait)
    while running_pid >/dev/null; do sleep 2; done
    exec sh "$0" status
    ;;
stop)
    if pid="$(running_pid)"; then
        kill "$pid" && echo "stopped run (pid $pid)"
        tmux kill-session -t "$session" 2>/dev/null
    else
        echo "no run is active"
    fi
    ;;
*)
    sed -n '2,20p' "$0" | sed 's/^# \{0,1\}//'
    exit 2
    ;;
esac
