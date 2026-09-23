# devpit shell startup — fish
#
# Read through --init-command, which fish runs after the person's own config:
# everything here is added to what they have, never in place of it.

# The request is consumed here and destroyed, before anything the shell later
# spawns can see or inherit it.
set -g __devpit_features (string split , -- "$DEVPIT_SHELL_FEATURES")
set -e DEVPIT_SHELL_FEATURES

# Inside tmux, every escape sequence tmux does not itself understand is eaten
# before it reaches the client — OSC 133 among them. tmux forwards a sequence
# wrapped in its own passthrough DCS, with each ESC inside doubled, and only
# when `allow-passthrough` is on. Outside tmux the plain form is what works.
function __devpit_osc
    if set -q TMUX
        printf '\033Ptmux;\033\033]%s\007\033\\' $argv[1]
    else
        printf '\033]%s\007' $argv[1]
    end
end

# Where `devpit-agent` lives. tmux will not carry a PATH of ours.
if set -q DEVPIT_BIN; and test -n "$DEVPIT_BIN"; and not contains -- $DEVPIT_BIN $PATH
    set -gx PATH $DEVPIT_BIN $PATH
end
set -e DEVPIT_BIN

# Agents typed by hand get what one devpit starts gets: its hooks and its
# tools. Only where the person has no function of that name, and not twice.
set -g __devpit_claude_settings "$DEVPIT_CLAUDE_SETTINGS"
set -g __devpit_claude_mcp "$DEVPIT_CLAUDE_MCP"
set -g __devpit_codex_exe "$DEVPIT_CODEX_EXE"
set -e DEVPIT_CLAUDE_SETTINGS DEVPIT_CLAUDE_MCP DEVPIT_CODEX_EXE
if test -n "$__devpit_claude_settings$__devpit_claude_mcp"; and not functions -q claude
    function claude
        set -l with
        if test -n "$__devpit_claude_settings"; and not string match -q -- '*--settings*' "$argv"
            set -a with "--settings=$__devpit_claude_settings"
        end
        if test -n "$__devpit_claude_mcp"; and not string match -q -- '*--mcp-config*' "$argv"
            set -a with "--mcp-config=$__devpit_claude_mcp"
        end
        command claude $with $argv
    end
end
if test -n "$__devpit_codex_exe"; and not functions -q codex
    function codex
        if string match -q -- '*mcp_servers.devpit*' "$argv"
            command codex $argv
        else
            command codex -c "mcp_servers.devpit.command=\"$__devpit_codex_exe\"" \
                -c 'mcp_servers.devpit.args=["mcp"]' \
                -c 'mcp_servers.devpit.env={DEVPIT_AGENT_ID="codex"}' $argv
        end
    end
end

# fish 4 marks its own prompts (OSC 133 A/B/C/D, and OSC 7) unless told not
# to. Two sets of marks would be two of every block, so where fish speaks for
# itself only the line it ran is added — which fish's marks carry as a URL
# and ours as the text the block's header shows.
set -g __devpit_own_marks 1
if status test-feature mark-prompt 2>/dev/null
    set -e __devpit_own_marks
end

if contains -- marks $__devpit_features; and not set -q __devpit_own_marks
    function __devpit_preexec --on-event fish_preexec
        set -l line (string join \n -- $argv | string replace -a \a '' | string replace -a \e '' | string collect)
        __devpit_osc "777;devpit-cmd;"(string sub -l 2000 -- "$line")
    end
else if contains -- marks $__devpit_features
    # Where the shell is, and a new prompt, before it is drawn. `%` and spaces
    # are the two a path carries that a URI cannot.
    function __devpit_prompt --on-event fish_prompt
        set -l here (string replace -a '%' '%25' -- $PWD)
        __devpit_osc "7;file://"(prompt_hostname)(string replace -a ' ' '%20' -- $here)
        __devpit_osc '133;A'
    end

    # The line as typed, then the start of its output. The terminator bytes
    # are taken out — they would end the sequence early — and a pasted script
    # is cut to a header's length.
    function __devpit_preexec --on-event fish_preexec
        set -l line (string join \n -- $argv | string replace -a \a '' | string replace -a \e '' | string collect)
        __devpit_osc "777;devpit-cmd;"(string sub -l 2000 -- "$line")
        __devpit_osc '133;C'
        set -g __devpit_running 1
    end

    # What it exited with. Read first: anything before it would overwrite it.
    function __devpit_postexec --on-event fish_postexec
        set -l code $status
        if set -q __devpit_running
            __devpit_osc "133;D;$code"
            set -e __devpit_running
        end
    end
end
# A shell typed here, and a login on another machine, keep their commands as
# blocks: they start through the same startup files as this one. Only a bare
# interactive `bash` or `zsh`, and only where the person has no function of
# that name. `DEVPIT_SSH_WRAP=0` leaves ssh alone.
if contains -- marks $__devpit_features
    set -g __devpit_root (string replace -r -- '/fish/init\.fish$' '' (status filename))
    if not functions -q bash; and test -r "$__devpit_root/bash/rcfile"
        function bash --wraps bash
            if test (count $argv) -eq 0; and isatty stdin; and isatty stdout
                env DEVPIT_SHELL_FEATURES=marks bash --rcfile "$__devpit_root/bash/rcfile"
            else
                command bash $argv
            end
        end
    end
    if not functions -q zsh; and test -r "$__devpit_root/zsh/.zshenv"
        function zsh --wraps zsh
            if test (count $argv) -eq 0; and isatty stdin; and isatty stdout
                env DEVPIT_ORIG_ZDOTDIR="$ZDOTDIR" ZDOTDIR="$__devpit_root/zsh" DEVPIT_SHELL_FEATURES=marks zsh
            else
                command zsh $argv
            end
        end
    end
    if not functions -q ssh; and test -r "$__devpit_root/ssh/login.sh"
        function ssh --wraps ssh
            if test "$DEVPIT_SSH_WRAP" != 0; and isatty stdin; and isatty stdout
                command sh "$__devpit_root/ssh/login.sh" "$__devpit_root" $argv
            else
                command ssh $argv
            end
        end
    end
end
set -e __devpit_features __devpit_own_marks
