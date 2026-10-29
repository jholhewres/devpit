//! A value that must be one word to anything that reads it as a command line.

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
