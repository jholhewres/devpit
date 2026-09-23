//! What a declared profile is allowed to say.
//!
//! A profile is environment, program and arguments, and the three are checked
//! by different rules because they are different kinds of thing.
//!
//! The program and the arguments are a program's own vocabulary: `claude`,
//! `--permission-mode`, `bypassPermissions`. Nothing there ever needs a space
//! or a semicolon, and one of the three places a profile is used types a line
//! into a terminal — so anything a shell would read as punctuation is refused
//! outright rather than escaped. Refusing is a rule somebody can hold in their
//! head; escaping is a rule that has to be right everywhere forever.
//!
//! An environment **value** is the opposite: it is data. A token, a URL, a
//! home directory with a space in it. It is allowed to be anything and is
//! quoted where quoting is needed. Its *name* is not data — it is a variable
//! name, and the shell's own rule for those is the rule here.

use devpit_rpc::Declared;

/// Why a profile was not saved.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Refused {
    /// A profile with no name is a row nobody can point at.
    Unnamed,
    /// The base agent is not one this build knows.
    UnknownBase(String),
    /// A program name that is really a command line.
    NotAProgram(String),
    /// An argument carrying shell punctuation or a space.
    NotAnArgument(String),
    /// `1FOO`, `foo-bar`, or anything else no shell would accept.
    NotAVariableName(String),
    /// The same variable set twice, where only the last would have counted.
    SetTwice(String),
    /// A model name with a space or shell punctuation in it.
    NotAModel(String),
}

impl std::fmt::Display for Refused {
    fn fmt(&self, out: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Unnamed => write!(out, "give the profile a name"),
            Self::UnknownBase(base) => write!(out, "{base} is not an agent this build knows"),
            Self::NotAProgram(word) => {
                write!(
                    out,
                    "{word} is a command line, not a program — put the flags in Arguments"
                )
            }
            Self::NotAnArgument(word) => {
                write!(out, "{word} cannot be an argument: no spaces, and nothing a shell would read as punctuation")
            }
            Self::NotAVariableName(name) => {
                write!(out, "{name} is not a variable name — letters, digits and underscore, not starting with a digit")
            }
            Self::SetTwice(name) => write!(out, "{name} is set twice"),
            Self::NotAModel(model) => {
                write!(out, "{model} is not a model name — letters, digits and . - _ : / @, with [1m] allowed at the end")
            }
        }
    }
}

/// Whether a string is one word a shell would not interpret.
///
/// The same rule the "Open in" apps use for their program, and the same
/// reasoning: anything a shell treats as punctuation is refused, and so is
/// whitespace, because a field that holds one thing holds one thing.
pub fn is_one_word(word: &str) -> bool {
    let word = word.trim();
    !word.is_empty()
        && word.len() <= 512
        && !word.chars().any(|letter| {
            letter.is_whitespace()
                || letter.is_control()
                || ";|&$`<>()[]{}*?!#'\"\\\n".contains(letter)
        })
}

/// Whether a name is one a shell would accept for a variable.
pub fn is_a_variable_name(name: &str) -> bool {
    let mut letters = name.chars();
    letters
        .next()
        .is_some_and(|first| first.is_ascii_alphabetic() || first == '_')
        && letters.all(|letter| letter.is_ascii_alphanumeric() || letter == '_')
}

/// Whether a string is a model the CLI could be passed.
///
/// Stricter than an argument, because it is a name and not a flag: the `[1m]`
/// suffix is the one place brackets belong, and only at the end.
pub fn is_a_model(model: &str) -> bool {
    let bare = model.strip_suffix("[1m]").unwrap_or(model);
    !bare.is_empty()
        && model.len() <= 128
        && bare
            .chars()
            .all(|letter| letter.is_ascii_alphanumeric() || "._-:/@".contains(letter))
}

/// Whether this profile can be saved, and why not when it cannot.
pub fn allowed(declared: &Declared, knows_base: impl Fn(&str) -> bool) -> Result<(), Refused> {
    if declared.label.trim().is_empty() {
        return Err(Refused::Unnamed);
    }
    if !knows_base(&declared.base) {
        return Err(Refused::UnknownBase(declared.base.clone()));
    }
    // Empty is the base agent's own program, which is the common case and the
    // reason this is not simply required.
    if !declared.command.is_empty() && !is_one_word(&declared.command) {
        return Err(Refused::NotAProgram(declared.command.clone()));
    }
    for arg in &declared.args {
        if !is_one_word(arg) {
            return Err(Refused::NotAnArgument(arg.clone()));
        }
    }
    let mut seen: Vec<&str> = Vec::new();
    for var in &declared.env {
        if !is_a_variable_name(&var.name) {
            return Err(Refused::NotAVariableName(var.name.clone()));
        }
        if seen.contains(&var.name.as_str()) {
            return Err(Refused::SetTwice(var.name.clone()));
        }
        seen.push(&var.name);
    }
    if let Some(model) = declared.models.iter().find(|one| !is_a_model(one)) {
        return Err(Refused::NotAModel(model.clone()));
    }
    Ok(())
}

/// The profile with `~` and `$HOME` at the front of a value made absolute.
///
/// A value is handed to a spawned process as it is, and single-quoted where it
/// is typed, so no shell ever expands it — `~/.claude-claudin` pasted from a
/// `.zshrc` would otherwise name a folder called `~`.
pub fn at_home(mut declared: Declared, home: &str) -> Declared {
    for var in &mut declared.env {
        let rest = ["~/", "$HOME/", "${HOME}/"]
            .iter()
            .find_map(|lead| var.value.strip_prefix(lead));
        if let Some(rest) = rest {
            var.value = format!("{}/{rest}", home.trim_end_matches('/'));
        } else if ["~", "$HOME", "${HOME}"].contains(&var.value.as_str()) {
            var.value = home.to_owned();
        }
    }
    declared
}

/// A value as one shell word.
///
/// Single quotes, because inside them a shell interprets nothing at all — and
/// a `'` in the value closes, escapes and reopens. The terminal is the one
/// place a profile becomes text; the other two spawn a process and hand the
/// value over untouched.
pub fn quoted(value: &str) -> String {
    format!("'{}'", value.replace('\'', r"'\''"))
}

#[cfg(test)]
#[path = "declaring_tests.rs"]
mod tests;
