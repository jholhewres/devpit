# devpit shell startup — ssh
#
# Run through `sh` by the `ssh` function the startup files define, with the
# folder those files live in and then ssh's own arguments. Inside this script
# `ssh` is the real one: a shell function is not inherited by a new process.
#
# For an interactive login, and only that, the remote shell is started with
# the same startup file as a local one, so the commands run on the other side
# are blocks too. Anything else — a remote command, a tunnel, a subsystem, a
# forced TTY setting, a missing tool here — is plain ssh, untouched.

root=$1
shift

plain() {
  exec ssh "$@"
}

# Whether these arguments open a login: a destination and nothing after it.
# ssh reads options after the destination too, so they are parsed on both
# sides of it. The option letters are OpenSSH's; one that asks for no shell
# — `-N`, `-T`, `-W`, `-s`, `-O`, `-G`, `-V`, `-f`, `-n` — ends the question.
logs_in() {
  host=''
  while :; do
    OPTIND=1
    while getopts ':1246AaCfGgKkMNnqsTtVvXxYyB:b:c:D:E:e:F:I:i:J:L:l:m:O:o:P:p:Q:R:S:W:w:' opt; do
      case $opt in
        f | G | N | n | O | Q | s | T | V | W | : | \?) return 1 ;;
      esac
    done
    ended=''
    [ "$OPTIND" -gt 1 ] && eval "ended=\${$((OPTIND - 1))}"
    shift $((OPTIND - 1))
    if [ "$ended" = -- ]; then
      [ -z "$host" ] && [ $# -eq 1 ] && return 0
      [ -n "$host" ] && [ $# -eq 0 ]
      return
    fi
    [ $# -gt 0 ] || break
    # A second word is a command for the remote side.
    [ -z "$host" ] || return 1
    host=$1
    shift
  done
  [ -n "$host" ]
}

logs_in "$@" || plain "$@"

# What the configuration adds: a RemoteCommand, a TTY refused, no session.
config=$(ssh -G "$@" 2>/dev/null) || plain "$@"
if printf '%s\n' "$config" | grep -Eiq '^(remotecommand |requesttty (no|false)$|sessiontype (none|subsystem)$|forkafterauthentication yes$|stdinnull yes$)'; then
  plain "$@"
fi

bash_rc=$root/bash/rcfile
zsh_env=$root/zsh/.zshenv
[ -r "$bash_rc" ] && [ -r "$zsh_env" ] || plain "$@"

# The script the remote side runs: this side's two startup files, written to a
# folder of its own, and the remote login shell started through the one that
# fits it. Each file removes that folder first — an open file outlives its
# name — so nothing of devpit is left on the other machine.
boot() {
  # Inside tmux here, the marks must be wrapped for this tmux to pass them on,
  # whatever the remote side knows about tmux.
  if [ -n "${TMUX:-}" ]; then
    printf 'passthrough=1\n'
  else
    printf 'passthrough=\n'
  fi
  cat <<'__devpit_static'
mkdir -p "$d/zsh" || return
first="command rm -rf -- '$d'"
printf '%s\n' "$first" > "$d/rcfile"
printf '%s\n' "$first" > "$d/zsh/.zshenv"
cat >> "$d/rcfile" <<'__devpit_end'
__devpit_static
  cat "$bash_rc"
  printf '\n__devpit_end\n'
  cat <<'__devpit_static'
cat >> "$d/zsh/.zshenv" <<'__devpit_end'
__devpit_static
  cat "$zsh_env"
  printf '\n__devpit_end\n'
  cat <<'__devpit_end'
export DEVPIT_SHELL_FEATURES=marks
[ -z "$passthrough" ] || export DEVPIT_PASSTHROUGH=1
case ${SHELL##*/} in
  bash) exec "$SHELL" --rcfile "$d/rcfile" ;;
  zsh)
    [ -z "${ZDOTDIR:-}" ] || export DEVPIT_ORIG_ZDOTDIR="$ZDOTDIR"
    export ZDOTDIR="$d/zsh"
    exec "$SHELL" -l
    ;;
esac
unset DEVPIT_SHELL_FEATURES DEVPIT_PASSTHROUGH
__devpit_end
}

payload=$(boot | gzip -c | base64 | tr -d '\n') || plain "$@"
[ -n "$payload" ] || plain "$@"

# The one line the remote login shell is handed. Whatever that shell is — sh,
# bash, zsh, fish, csh — it reads `exec sh -c '…'` the same way, which is why
# what is inside the quotes has no quote, backslash, `!` or newline of its
# own. A remote side missing a tool, or a shell with no startup file here,
# lands in its login shell as plain ssh would have.
remote="exec sh -c 'd=\$(mktemp -d 2>/dev/null)||exec \"\${SHELL:-sh}\" -l;echo $payload|(base64 -d 2>/dev/null||base64 -D 2>/dev/null)|gzip -dc >\"\$d/boot\" 2>/dev/null&&. \"\$d/boot\";rm -rf \"\$d\";exec \"\${SHELL:-sh}\" -l'"

exec ssh -t "$@" "$remote"
