use super::{refused, Pairing};

fn asked() -> Pairing {
    Pairing {
        code: "ABCD2345".to_owned(),
        name: "Pixel".to_owned(),
    }
}

/// A device pairing over the tailnet with the code on screen is still asked
/// about, by the name it gave, and paired only on a yes.
#[test]
fn a_tailnet_pairing_asks_the_person_after_the_code() {
    let mut asked_about = Vec::new();
    let no = refused(
        &asked(),
        |code| code == "ABCD2345",
        |name| {
            asked_about.push(name.to_owned());
            false
        },
    );
    assert!(no.is_some());
    assert_eq!(asked_about, ["Pixel"]);
    assert!(refused(&asked(), |_| true, |_| true).is_none());
}

#[test]
fn a_wrong_code_is_refused_before_anyone_is_asked() {
    let refusal = refused(&asked(), |_| false, |_| panic!("asked with a wrong code"));
    assert!(refusal.is_some_and(|why| String::from_utf8_lossy(why).contains("code")));
}
