#!/usr/bin/env bash
# kr — KernRift fat binary launcher
#
# Termux on Android 14+ denies raw execve of files in /data/data/<app>/.
# Our runner binary (kr-bin) does a raw syscall and gets EACCES when it
# tries to launch the extracted slice. To work around this, kr-bin still
# extracts and chmod's the slice to ./kr-exec, then exits 120. We catch
# that here and re-exec ./kr-exec via the user's shell, where the Termux
# libc LD_PRELOAD wrapper makes exec succeed.
#
# Other platforms (Linux/macOS) raise no such restriction; the runner
# exec's directly and we never reach the post-call return.

set -u

KRBIN="${KRBIN:-${0%/*}/kr-bin}"
[ -x "$KRBIN" ] || KRBIN=$(command -v kr-bin)
if [ -z "${KRBIN:-}" ] || [ ! -x "$KRBIN" ]; then
    echo "kr: kr-bin not found alongside this wrapper" >&2
    exit 1
fi

"$KRBIN" "$@"
status=$?

# Termux fallback: 120 = "extracted ./kr-exec, exec was denied"
if [ "$status" -eq 120 ] && [ -x ./kr-exec ]; then
    shift  # drop the .krbo path; remaining args go to the slice
    exec ./kr-exec "$@"
fi

exit $status
