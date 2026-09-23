use devpit_rpc::{Declared, EnvVar};

use super::*;

fn known(base: &str) -> bool {
    base == "claude"
}

fn profile() -> Declared {
    Declared {
        id: "01J".to_owned(),
        label: "GLM".to_owned(),
        base: "claude".to_owned(),
        command: String::new(),
        args: Vec::new(),
        models: Vec::new(),
        env: Vec::new(),
    }
}

fn var(name: &str, value: &str) -> EnvVar {
    EnvVar {
        name: name.to_owned(),
        value: value.to_owned(),
    }
}

#[test]
fn the_shape_measured_in_a_real_zshrc_is_allowed() {
    // `glm` as it was actually written: seven variables, the agent's own
    // program, and one flag. If this is refused the feature does not exist.
    let mut glm = profile();
    glm.env = vec![
        var("ANTHROPIC_BASE_URL", "https://api.z.ai/api/anthropic"),
        var("ANTHROPIC_MODEL", "glm-5.3"),
        var("CLAUDE_CONFIG_DIR", "/home/someone/.claude-glm"),
    ];
    glm.args = vec![
        "--permission-mode".to_owned(),
        "bypassPermissions".to_owned(),
    ];
    assert_eq!(allowed(&glm, known), Ok(()));
}

#[test]
fn a_profile_needs_a_name() {
    let mut unnamed = profile();
    unnamed.label = "   ".to_owned();
    assert_eq!(allowed(&unnamed, known), Err(Refused::Unnamed));
}

#[test]
fn a_base_this_build_does_not_know_is_refused() {
    let mut odd = profile();
    odd.base = "hal9000".to_owned();
    assert_eq!(
        allowed(&odd, known),
        Err(Refused::UnknownBase("hal9000".to_owned()))
    );
}

#[test]
fn an_empty_command_means_the_base_agents_own() {
    // The common case. `glm` runs `claude`; only `clawz` named a program.
    assert_eq!(allowed(&profile(), known), Ok(()));
}

#[test]
fn a_command_line_in_the_program_field_is_refused() {
    let mut sneaky = profile();
    sneaky.command = "claude --permission-mode bypassPermissions".to_owned();
    assert!(matches!(
        allowed(&sneaky, known),
        Err(Refused::NotAProgram(_))
    ));
}

#[test]
fn an_absolute_path_is_a_fine_program() {
    // `clawz` pointed at a debug build in a source tree.
    let mut local = profile();
    local.command = "/home/someone/src/claw/target/debug/claw".to_owned();
    assert_eq!(allowed(&local, known), Ok(()));
}

#[test]
fn shell_punctuation_in_an_argument_is_refused() {
    for bad in ["; rm -rf /", "$(whoami)", "`id`", "a|b", "x>y"] {
        let mut hostile = profile();
        hostile.args = vec![bad.to_owned()];
        assert!(
            matches!(allowed(&hostile, known), Err(Refused::NotAnArgument(_))),
            "{bad} was allowed through"
        );
    }
}

#[test]
fn a_variable_name_follows_the_shells_own_rule() {
    for good in ["FOO", "_foo", "ANTHROPIC_BASE_URL", "a1"] {
        assert!(is_a_variable_name(good), "{good} should be a name");
    }
    for bad in ["1FOO", "foo-bar", "foo bar", "", "FOO=", "FOO.BAR"] {
        assert!(!is_a_variable_name(bad), "{bad} should not be a name");
    }
}

#[test]
fn a_bad_variable_name_is_refused_by_name() {
    let mut wrong = profile();
    wrong.env = vec![var("1FOO", "x")];
    assert_eq!(
        allowed(&wrong, known),
        Err(Refused::NotAVariableName("1FOO".to_owned()))
    );
}

#[test]
fn a_value_may_be_anything_at_all() {
    // The opposite rule from the program and the arguments, on purpose: a
    // value is data. A token, a URL, a path with a space, a semicolon.
    let mut anything = profile();
    anything.env = vec![
        var("TOKEN", "a;b|c$(d)`e`"),
        var("HOME_ISH", "/home/some one/with space"),
        var("EMPTY", ""),
    ];
    assert_eq!(allowed(&anything, known), Ok(()));
}

#[test]
fn setting_the_same_variable_twice_is_refused() {
    // Only the last would count, so the list would be lying about itself.
    let mut twice = profile();
    twice.env = vec![var("FOO", "one"), var("FOO", "two")];
    assert_eq!(
        allowed(&twice, known),
        Err(Refused::SetTwice("FOO".to_owned()))
    );
}

#[test]
fn a_quoted_value_is_one_word_to_a_shell() {
    assert_eq!(quoted("plain"), "'plain'");
    assert_eq!(quoted("with space"), "'with space'");
    assert_eq!(quoted("a;b"), "'a;b'");
}

#[test]
fn a_quote_inside_a_value_closes_escapes_and_reopens() {
    // The one case naive quoting gets wrong, and the one that ends a string
    // early and leaves the rest of it as shell.
    assert_eq!(quoted("it's"), r"'it'\''s'");
    assert_eq!(quoted("'; rm -rf /"), r"''\''; rm -rf /'");
}

#[test]
fn a_quoted_value_survives_a_real_shell() {
    // The escape is only as good as what a shell does with it, so a shell is
    // what checks it.
    for value in [
        "plain",
        "with space",
        "it's",
        "'; echo owned",
        "$HOME",
        "a`b`c",
    ] {
        let script = format!("printf %s {}", quoted(value));
        let out = std::process::Command::new("sh")
            .args(["-c", &script])
            .output()
            .expect("sh");
        assert_eq!(
            String::from_utf8_lossy(&out.stdout),
            value,
            "the shell read {value:?} as something else"
        );
    }
}

#[test]
fn one_word_is_the_same_rule_the_open_in_apps_use() {
    assert!(is_one_word("code"));
    assert!(is_one_word("/usr/bin/code"));
    assert!(!is_one_word("code --wait"));
    assert!(!is_one_word(""));
}

#[test]
fn no_refusal_ever_quotes_a_value() {
    // Names are safe to say back — a variable name, a flag, a program. A
    // value is where the token is, and a refusal is a string that ends up on
    // a screen and in whatever the screen is copied into.
    let secret = "sk-do-not-say-this-out-loud";
    let mut carrying = profile();
    carrying.env = vec![var("1BAD", secret), var("ANTHROPIC_AUTH_TOKEN", secret)];
    let why = allowed(&carrying, known).expect_err("it should refuse");
    assert!(
        !why.to_string().contains(secret),
        "the refusal said the value out loud: {why}"
    );

    let mut twice = profile();
    twice.env = vec![
        var("ANTHROPIC_AUTH_TOKEN", secret),
        var("ANTHROPIC_AUTH_TOKEN", secret),
    ];
    let why = allowed(&twice, known).expect_err("it should refuse");
    assert!(!why.to_string().contains(secret), "{why}");
}

#[test]
fn a_model_may_carry_the_wide_context_suffix_and_nothing_a_shell_reads() {
    for good in ["opus", "glm-5.3[1m]", "claude-opus-5-5", "org/model:tag@v1"] {
        assert!(is_a_model(good), "{good} should be a model");
    }
    for bad in ["", "[1m]", "glm 5", "a[1m]b", "$(id)", "a;b", "glm[2m]"] {
        assert!(!is_a_model(bad), "{bad} should not be a model");
    }
    let mut odd = profile();
    odd.models = vec!["glm-5.3".to_owned(), "glm 5".to_owned()];
    assert_eq!(
        allowed(&odd, known),
        Err(Refused::NotAModel("glm 5".to_owned()))
    );
}

#[test]
fn a_home_at_the_front_of_a_value_is_made_absolute() {
    // Pasted from a `.zshrc`, where the shell expanded it; nothing here will.
    let mut claudin = profile();
    claudin.env = vec![
        var("CLAUDE_CONFIG_DIR", "~/.claude-claudin"),
        var("ONE", "$HOME/.claude-glm"),
        var("TWO", "${HOME}"),
        var("TOKEN", "a~/b"),
    ];
    let done = at_home(claudin, "/home/someone/");
    let values: Vec<&str> = done.env.iter().map(|one| one.value.as_str()).collect();
    assert_eq!(
        values,
        [
            "/home/someone/.claude-claudin",
            "/home/someone/.claude-glm",
            "/home/someone/",
            "a~/b"
        ]
    );
}
