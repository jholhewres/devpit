# devpit shell startup — zsh
#
# Read as .zshenv from a ZDOTDIR of ours, which is the only startup file zsh
# reads before deciding where the rest live. So it hands ZDOTDIR back at once,
# sources their .zshenv, and defers everything else to a precmd that runs after
# their whole config — which may replace precmd_functions wholesale.

__devpit_usable_zdotdir() {
  [[ -n "${1:-}" ]] || return 1
  # Never ours, however we were pointed at it.
  [[ -f "$1/.devpit-shell-wrapper" ]] && return 1
  local file
  for file in .zshenv .zshrc .zprofile .zlogin; do
    [[ -r "$1/$file" ]] && return 0
  done
  # A directory with no zsh startup file in it is not a config root, and
  # keeping a stale one would stop zsh reading their real .zshenv forever.
  return 1
}
if __devpit_usable_zdotdir "${DEVPIT_ORIG_ZDOTDIR:-}"; then
  builtin export ZDOTDIR="$DEVPIT_ORIG_ZDOTDIR"
else
  builtin unset ZDOTDIR
fi
builtin unset DEVPIT_ORIG_ZDOTDIR
builtin unfunction __devpit_usable_zdotdir

# Consumed and destroyed here, so nothing this shell spawns inherits it.
builtin typeset -ga __devpit_features
__devpit_features=(${(s:,:)${DEVPIT_SHELL_FEATURES:-}})
builtin unset DEVPIT_SHELL_FEATURES
__devpit_wants() { (( ${__devpit_features[(Ie)$1]} )) }

# Inside tmux, every escape sequence tmux does not itself understand is eaten
# before it reaches the client — OSC 133 among them. tmux forwards a sequence
# wrapped in its own passthrough DCS, with each ESC inside doubled, and only
# when `allow-passthrough` is on. Outside tmux the plain form is what works.
__devpit_osc() {
  if [[ -n "${TMUX:-}" ]]; then
    builtin printf '\033Ptmux;\033\033]%s\007\033\\' "$1"
  else
    builtin printf '\033]%s\007' "$1"
  fi
}

__devpit_wants identity && __devpit_osc "777;devpit-shell-start:$$"

__devpit_precmd() {
  local code=$?
  if [[ -n "${__devpit_running:-}" ]]; then
    __devpit_osc "133;D;$code"
    builtin unset __devpit_running
  fi
  __devpit_osc '133;A'
}

__devpit_preexec() {
  __devpit_osc '133;C'
  # typeset -g: a plain assignment inside a function warns under
  # warn_create_global, above every command the person runs.
  builtin typeset -g __devpit_running=1
}

__devpit_init() {
  # emulate -L first: this body runs after their config, so without it we
  # inherit whatever options that config set. Under no_unset an unset
  # precmd_functions is fatal, and ksh_arrays breaks the 1-based lookup above.
  builtin emulate -L zsh
  (( $+__devpit_init_done )) && return 0
  builtin typeset -g __devpit_init_done=1
  builtin typeset -g precmd_functions preexec_functions

  if __devpit_wants marks; then
    # Substituted in place, not appended: this function is running from inside
    # precmd_functions right now, and appending would leave it there.
    precmd_functions=(${precmd_functions:/__devpit_init/__devpit_precmd})
    preexec_functions=(__devpit_preexec ${preexec_functions[@]})
  else
    precmd_functions=(${precmd_functions:#__devpit_init})
  fi

  if __devpit_wants ready; then
    # Chained, not replaced. Only a user-defined widget is callable as a plain
    # function; a builtin or completion form is left alone.
    if [[ "${widgets[zle-line-init]:-}" == "user:__devpit_ready_mark" ]]; then
      :
    elif (( ${+widgets[zle-line-init]} )) && [[ "${widgets[zle-line-init]}" == user:* ]]; then
      __devpit_prev_line_init="${widgets[zle-line-init]#user:}"
    else
      __devpit_prev_line_init=""
    fi
    __devpit_ready_mark() {
      __devpit_osc '777;devpit-shell-ready'
      # Called as a plain function so $WIDGET stays zle-line-init for
      # add-zle-hook-widget dispatchers.
      [[ -z "${__devpit_prev_line_init:-}" ]] || "${__devpit_prev_line_init}" "$@"
    }
    zle -N zle-line-init __devpit_ready_mark
  fi

  # Called by hand: we were appended during this prompt's own precmd sweep, so
  # the permanent hook has not run yet and the first prompt would lose its mark.
  __devpit_wants marks && __devpit_precmd

  builtin unset __devpit_features
  builtin unfunction __devpit_init __devpit_wants
}

{
  builtin typeset __devpit_their_zshenv="${ZDOTDIR-$HOME}/.zshenv"
  [[ ! -r "$__devpit_their_zshenv" ]] || builtin source -- "$__devpit_their_zshenv"
} always {
  builtin unset __devpit_their_zshenv
  builtin typeset -ag precmd_functions
  (( ${precmd_functions[(Ie)__devpit_init]} )) || precmd_functions+=(__devpit_init)
}
