use super::*;

/// The row under a skill's name.
///
/// Measured on the machine this was written on: 244 `SKILL.md` files, every
/// one of them carrying a `description:` in its frontmatter, and the panel
/// showing the body's opening sentence instead — often two hundred characters
/// of instructions in a row one line tall.
mod the_description {
    use super::*;

    const REAL: &str = "---\n\
        name: ai-slop-cleaner\n\
        description: Clean AI-generated code slop with a regression-safe workflow\n\
        level: 3\n\
        ---\n\
        \n\
        # AI Slop Cleaner\n\
        \n\
        Use this skill to clean AI-generated code slop without drifting scope.\n";

    #[test]
    fn comes_from_the_frontmatter_not_the_body() {
        assert_eq!(
            description_of(REAL),
            "Clean AI-generated code slop with a regression-safe workflow"
        );
    }

    #[test]
    fn a_block_scalar_gives_its_first_line() {
        // 14 of the 244 write it this way. Reading the `|` itself would put a
        // pipe character under every one of their names.
        let text = "---\ndescription: |\n  Query the usage statistics.\n  Triggered by a command.\nname: x\n---\n\nbody\n";
        assert_eq!(description_of(text), "Query the usage statistics.");
    }

    #[test]
    fn quotes_around_the_value_are_not_part_of_it() {
        let text = "---\ndescription: \"Submit case feedback\"\n---\n\nbody\n";
        assert_eq!(description_of(text), "Submit case feedback");
    }

    #[test]
    fn a_skill_with_no_frontmatter_falls_back_to_its_prose() {
        let text = "# tdd\n\nWrite the test first.\n";
        assert_eq!(description_of(text), "Write the test first.");
    }

    #[test]
    fn frontmatter_without_a_description_falls_back_too() {
        let text = "---\nname: x\n---\n\n# x\n\nWhat it does.\n";
        assert_eq!(description_of(text), "What it does.");
    }

    #[test]
    fn the_heading_is_not_the_description() {
        // The heading repeats the name, and a row reading `tdd — # tdd` says
        // nothing twice.
        assert_eq!(description_of("# tdd\n\nReal prose.\n"), "Real prose.");
    }
}

#[test]
fn the_body_comes_back_without_its_frontmatter() {
    let text = "---\nname: x\ndescription: y\n---\n\n# Heading\n\nProse.\n";
    assert_eq!(
        split_frontmatter(text).1.trim_start(),
        "# Heading\n\nProse.\n"
    );
}

#[test]
fn a_list_right_under_the_frontmatter_keeps_its_markers() {
    let text = "---\nname: x\n---\n- first\n- second\n";
    assert_eq!(split_frontmatter(text).1, "- first\n- second\n");
}

#[test]
fn an_empty_block_is_not_described_by_the_next_key() {
    let text = "---\ndescription: |\nname: x\n---\n\nWhat it does.\n";
    assert_eq!(description_of(text), "What it does.");
}

#[test]
fn a_file_that_only_looks_like_frontmatter_keeps_all_of_itself() {
    // An opening `---` with no closing one is a horizontal rule, not a
    // frontmatter block, and eating the file from there loses the whole skill.
    let text = "---\njust prose after a rule\n";
    assert_eq!(split_frontmatter(text).1, text);
}

#[test]
fn a_name_that_is_really_a_path_is_refused() {
    // The lookup walks trusted directories, so a name is never resolved
    // against one — but a name carrying separators is a path pretending, and
    // saying so beats letting it miss quietly.
    for name in ["../../etc/passwd", "a/b", ".ssh"] {
        let refused = skills_read(name.to_owned()).expect_err("a refusal");
        assert_eq!(refused.code, devpit_rpc::ErrorCode::Forbidden);
    }
}

#[test]
fn a_skill_this_machine_does_not_have_is_not_found() {
    let missing = skills_read("no-such-skill-anywhere".to_owned()).expect_err("not found");
    assert_eq!(missing.code, devpit_rpc::ErrorCode::NotFound);
}
