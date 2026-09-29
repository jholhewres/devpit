use super::fits;

#[test]
fn a_pattern_fits_within_one_name() {
    assert!(fits(".env*", ".env"));
    assert!(fits(".env*", ".env.local"));
    assert!(!fits(".env*", "x.env"));
    assert!(fits("*.local.json", "app.local.json"));
    assert!(!fits("*.local.json", "app.json"));
    assert!(fits("a*b*c", "a-b-c"));
    assert!(!fits("a*b*c", "a-c"));
}
