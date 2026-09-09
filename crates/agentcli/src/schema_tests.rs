use super::*;

const SCHEMA: &str = r#"{"type":"object",
        "properties":{"verdict":{"type":"string"},"findings":{"type":"array"}},
        "required":["verdict","findings"]}"#;

#[test]
fn an_answer_with_the_declared_fields_passes() {
    assert_eq!(
        validates(r#"{"verdict":"approved","findings":[]}"#, SCHEMA),
        Ok(())
    );
}

/// The failure this exists to catch: prose where the card expected fields.
#[test]
fn prose_where_json_was_asked_for_is_named_as_such() {
    let error = validates("Looks good to me!", SCHEMA).expect_err("should refuse");
    assert!(error.contains("not JSON"), "{error}");
}

#[test]
fn a_missing_required_field_is_named() {
    let error = validates(r#"{"verdict":"approved"}"#, SCHEMA).expect_err("should refuse");
    assert!(error.contains("findings"), "{error}");
}

#[test]
fn a_field_of_the_wrong_type_is_named() {
    let error =
        validates(r#"{"verdict":"ok","findings":"none"}"#, SCHEMA).expect_err("should refuse");
    assert!(
        error.contains("findings") && error.contains("array"),
        "{error}"
    );
}

/// A step without a schema is a step that wanted prose.
#[test]
fn a_schema_without_required_fields_accepts_anything() {
    assert_eq!(
        validates(r#"{"anything":1}"#, r#"{"type":"object"}"#),
        Ok(())
    );
}
