# devpit shell startup — PowerShell
#
# Run through -EncodedCommand, after the person's own profile: everything here
# is added to what they have. Their prompt still draws the prompt; ours only
# puts the marks around it. Written for Windows PowerShell 5.1 as well as 7,
# which has neither the backtick escape for ESC nor ??: [char]27, if/else.

# The PATH Windows keeps for this user now, as a new Windows Terminal tab gets
# it: the server this shell runs under may have been started before a tool was
# installed, and `claude` would be missing here while found everywhere else.
$__devpit_fresh = @(
    [Environment]::GetEnvironmentVariable('Path', 'Machine'),
    [Environment]::GetEnvironmentVariable('Path', 'User'),
    $env:PATH
) -join ';' -split ';' | Where-Object { $_ } | Select-Object -Unique
$env:PATH = $__devpit_fresh -join ';'
Remove-Variable __devpit_fresh

# Where `devpit-agent` lives, after their profile, which may set PATH outright.
if ($env:DEVPIT_BIN -and -not (($env:PATH -split ';') -contains $env:DEVPIT_BIN)) {
    $env:PATH = "$env:DEVPIT_BIN;$env:PATH"
}

# Agents typed by hand get what one devpit starts gets: its hooks and its
# tools. Only where the person has no function of that name, and not twice.
$global:__devpit_claude_settings = $env:DEVPIT_CLAUDE_SETTINGS
$global:__devpit_claude_mcp = $env:DEVPIT_CLAUDE_MCP
foreach ($name in 'DEVPIT_BIN', 'DEVPIT_CLAUDE_SETTINGS', 'DEVPIT_CLAUDE_MCP', 'DEVPIT_CODEX_EXE') {
    Remove-Item "Env:$name" -ErrorAction SilentlyContinue
}
if (($global:__devpit_claude_settings -or $global:__devpit_claude_mcp) -and -not (Test-Path Function:\claude)) {
    function global:claude {
        $with = @()
        if ($global:__devpit_claude_settings -and -not ("$args" -like '*--settings*')) {
            $with += "--settings=$global:__devpit_claude_settings"
        }
        if ($global:__devpit_claude_mcp -and -not ("$args" -like '*--mcp-config*')) {
            $with += "--mcp-config=$global:__devpit_claude_mcp"
        }
        $found = Get-Command claude -CommandType Application, ExternalScript -ErrorAction SilentlyContinue |
            Select-Object -First 1
        & $found @with @args
    }
}

function global:__devpit_osc([string]$body) {
    "$([char]27)]$body$([char]7)"
}

# A new prompt, the exit of whatever just finished, and where the shell is.
# `%` and spaces are the two a path carries that a URI cannot.
$global:__devpit_running = $false
$global:__devpit_their_prompt = $function:prompt
function global:prompt {
    # Read first: anything run before this would overwrite both.
    $ok = $global:?
    $code = $global:LASTEXITCODE
    $out = ''
    if ($global:__devpit_running) {
        $status = if ($ok) { 0 } elseif ($code) { $code } else { 1 }
        $out += __devpit_osc "133;D;$status"
        $global:__devpit_running = $false
    }
    $here = $executionContext.SessionState.Path.CurrentLocation
    if ($here.Provider.Name -eq 'FileSystem') {
        $path = $here.ProviderPath
        # A long-path prefix is not part of the folder's name.
        if ($path.StartsWith('\\?\')) { $path = $path.Substring(4) }
        $path = $path.Replace('\', '/').Replace('%', '%25').Replace(' ', '%20')
        $out += __devpit_osc "7;file://$env:COMPUTERNAME/$($path.TrimStart('/'))"
    }
    $out += __devpit_osc '133;A'
    # Theirs sees the exit code it would have seen without us.
    $global:LASTEXITCODE = $code
    if ($global:__devpit_their_prompt) {
        $out += (& $global:__devpit_their_prompt) -join ''
    } else {
        $out += "PS $here> "
    }
    $out + (__devpit_osc '133;B')
}

# The line being run and where its output begins, as PSReadLine hands it over.
# Without PSReadLine there is no moment to say it, and only the prompts are
# marked. Control characters are taken out, and the length is capped.
if (Test-Path Function:\PSConsoleHostReadLine) {
    $global:__devpit_their_readline = $function:PSConsoleHostReadLine
    function global:PSConsoleHostReadLine {
        $line = & $global:__devpit_their_readline
        if ($line -and $line.Trim()) {
            # Every control character, not only the terminator bytes: a key
            # sent to clear the line arrives in it, and draws as a box.
            $shown = $line -replace '[\x00-\x08\x0B-\x1F\x7F]', ''
            if ($shown.Length -gt 2000) { $shown = $shown.Substring(0, 2000) }
            [Console]::Write((__devpit_osc "777;devpit-cmd;$shown"))
            [Console]::Write((__devpit_osc '133;C'))
            $global:__devpit_running = $true
        }
        $line
    }
}
