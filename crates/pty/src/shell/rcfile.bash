# devpit shell startup — bash
#
# Read through --rcfile, so it replaces the interactive startup files and must
# source them itself.

# The request is consumed here and destroyed, before anything the shell later
# spawns — including another devpit started from this very terminal — can see
# or inherit it.
__devpit_features=",${DEVPIT_SHELL_FEATURES:-},"
builtin unset DEVPIT_SHELL_FEATURES
__devpit_wants() { [[ "$__devpit_features" == *",$1,"* ]]; }

# Inside tmux, every escape sequence tmux does not itself understand is eaten
# before it reaches the client — OSC 133 among them. tmux forwards a sequence
# wrapped in its own passthrough DCS, with each ESC inside doubled, and only
# when `allow-passthrough` is on. Outside tmux the plain form is what works.
__devpit_osc() {
  if [[ -n "${TMUX:-}" ]]; then
    printf '\033Ptmux;\033\033]%s\007\033\\' "$1"
  else
    printf '\033]%s\007' "$1"
  fi
}

__devpit_wants identity && __devpit_osc "777;devpit-shell-start:$$"
__devpit_ready=''
__devpit_wants ready && __devpit_ready=1
__devpit_marks=''
__devpit_wants marks && __devpit_marks=1
unset __devpit_features
unset -f __devpit_wants

# Their own startup files, in the order bash would have read them.
[[ -f /etc/profile ]] && source /etc/profile
if [[ -f "$HOME/.bash_profile" ]]; then source "$HOME/.bash_profile"
elif [[ -f "$HOME/.bash_login" ]]; then source "$HOME/.bash_login"
elif [[ -f "$HOME/.profile" ]]; then source "$HOME/.profile"
fi
[[ -f "$HOME/.bashrc" ]] && source "$HOME/.bashrc"

# Where `devpit-agent` lives, after their config, which may set PATH outright;
# tmux will not carry a PATH of ours (`-e PATH=` is ignored).
if [[ -n "${DEVPIT_BIN:-}" && ":$PATH:" != *":$DEVPIT_BIN:"* ]]; then
  export PATH="$DEVPIT_BIN:$PATH"
fi
builtin unset DEVPIT_BIN

# Without bracketed paste, older readline reads each newline of a pasted
# multiline command as Enter and breaks it into PS2 continuations.
[[ $- == *i* ]] && bind 'set enable-bracketed-paste on' 2>/dev/null

if [[ -n "$__devpit_marks" ]]; then
  # Prompt boundary, and the exit code of whatever just finished. The code is
  # read first: anything before it would overwrite $?.
  __devpit_precmd() {
    local code=$?
    __devpit_in_prompt=1
    if [[ -n "${__devpit_running:-}" ]]; then
      __devpit_osc "133;D;$code"
      unset __devpit_running
    fi
    __devpit_osc '133;A'
    return "$code"
  }

  # Command start. The guards are the whole difficulty: DEBUG fires for every
  # simple command, including the ones our own prompt hooks run, and a `C`
  # emitted for those would mark a command nobody typed.
  __devpit_preexec() {
    [[ -z "${__devpit_starting:-}" ]] || return 0
    [[ -z "${__devpit_in_prompt:-}" ]] || return 0
    case "${FUNCNAME[1]:-}" in __devpit_*|__bp_*) return 0 ;; esac
    case "$BASH_COMMAND" in __devpit_*) return 0 ;; esac
    # Their trap, if a framework installed one, still runs.
    [[ -z "${__devpit_their_trap:-}" ]] || eval "$__devpit_their_trap" || true
    # A chained trap can fire twice for one command; only the first emits.
    [[ -z "${__devpit_running:-}" ]] || return 0
    __devpit_osc '133;C'
    __devpit_running=1
  }

  # Run last each prompt: adopt whatever trap a framework installed during the
  # prompt, then take DEBUG back. Without this, starship's trap replaces ours
  # at the first prompt and no command start is ever reported again.
  __devpit_epilogue() {
    unset __devpit_in_prompt
    local spec
    spec="$(trap -p DEBUG)"
    if [[ -n "$spec" && "$spec" != "trap -- '__devpit_preexec' DEBUG" ]]; then
      spec="${spec#trap -- }"
      spec="${spec% DEBUG}"
      eval "__devpit_their_trap=$spec"
    fi
    trap '__devpit_preexec' DEBUG
    if [[ -n "$__devpit_ready" ]]; then
      PS1="${PS1-}""\[$(__devpit_osc '777;devpit-shell-ready')\]"
      __devpit_ready=''
    fi
  }

  # Prepended and appended around whatever they already had. bash 5.1 gained
  # the array form; before it PROMPT_COMMAND is one string, and ours are joined
  # onto it with separators rather than replacing it.
  if (( BASH_VERSINFO[0] > 5 || (BASH_VERSINFO[0] == 5 && BASH_VERSINFO[1] >= 1) )); then
    PROMPT_COMMAND=(__devpit_precmd "${PROMPT_COMMAND[@]+"${PROMPT_COMMAND[@]}"}" __devpit_epilogue)
  else
    PROMPT_COMMAND="__devpit_precmd${PROMPT_COMMAND:+; $PROMPT_COMMAND}; __devpit_epilogue"
  fi

  # Armed only now: bash treats the commands in this very file as foreground
  # commands, and an armed trap would emit a C for each of them.
  __devpit_starting=1
  trap '__devpit_preexec' DEBUG
  unset __devpit_starting
elif [[ -n "$__devpit_ready" ]]; then
  PS1="${PS1-}""\[$(__devpit_osc '777;devpit-shell-ready')\]"
  __devpit_ready=''
fi
