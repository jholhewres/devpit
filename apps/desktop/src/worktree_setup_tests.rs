use super::kept;

#[test]
fn what_the_screen_sends_is_kept_without_blanks_or_paths_that_climb_out() {
    let prime = kept(devpit_rpc::WorktreeSetup {
        copy: vec![".env*".to_owned(), " ".to_owned(), "../secrets".to_owned()],
        link: vec!["/etc/passwd".to_owned()],
        run: vec!["pnpm install".to_owned(), String::new()],
        share: vec![devpit_rpc::EnvVar {
            name: "CARGO_TARGET_DIR".to_owned(),
            value: "/tmp/t".to_owned(),
        }],
    });
    assert_eq!(prime.copy, [".env*"]);
    assert!(prime.link.is_empty());
    assert_eq!(prime.run, ["pnpm install"]);
    assert_eq!(
        prime.share.get("CARGO_TARGET_DIR").map(String::as_str),
        Some("/tmp/t")
    );
}
