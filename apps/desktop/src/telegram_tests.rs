use super::{links, Kept};

fn waiting(code: &str, until: i64) -> Kept {
    Kept {
        token: "t".to_owned(),
        bot: "my_devpit_bot".to_owned(),
        chat_id: None,
        code: Some(code.to_owned()),
        code_until: until,
        titles: false,
    }
}

#[test]
fn the_code_devpit_showed_links_the_chat_while_it_holds() {
    let kept = waiting("A1B2C3", 100);
    assert!(links(&kept, "/start A1B2C3", 50));
    assert!(links(&kept, "a1b2c3", 50));
    assert!(!links(&kept, "/start A1B2C3", 100));
    assert!(!links(&kept, "/start ZZZZZZ", 50));
    assert!(!links(&kept, "hello", 50));
}

#[test]
fn a_linked_bot_takes_no_code_at_all() {
    let mut kept = waiting("A1B2C3", 100);
    kept.code = None;
    assert!(!links(&kept, "/start A1B2C3", 50));
}
