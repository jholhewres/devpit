use super::*;

fn minimal_manifest(id: &str) -> PluginManifest {
    PluginManifest {
        id: id.to_owned(),
        name: "Test Plugin".to_owned(),
        version: "0.1.0".to_owned(),
        description: "A manifest built only for a test.".to_owned(),
        surfaces: vec![Surface::CardPin],
        data: DataSpec {
            extensions: vec![".test".to_owned()],
            max_bytes: 1024.0,
        },
        permissions: vec![],
    }
}

fn shipped(id: &str) -> PluginManifest {
    catalogue()
        .into_iter()
        .find(|one| one.id == id)
        .unwrap_or_else(|| panic!("{id} is shipped"))
}

/// Every Capability this build ships, and the shape of the first of them.
#[test]
fn the_catalogue_ships_the_capabilities_the_window_mounts() {
    let ids: Vec<String> = catalogue().into_iter().map(|one| one.id).collect();
    assert_eq!(ids, vec!["excalidraw", "notes"]);

    let excalidraw = shipped("excalidraw");
    assert_eq!(excalidraw.data.extensions, vec![".excalidraw".to_owned()]);
    assert_eq!(excalidraw.data.max_bytes, MAX_DATA_BYTES);
    assert_eq!(
        excalidraw.surfaces,
        vec![Surface::Pane { many: true }, Surface::CardPin]
    );
    assert_eq!(excalidraw.permissions, vec![Permission::DataOwn]);

    // The window names these extensions too; a rename here without one there
    // opens a pane no file ever reaches.
    assert_eq!(shipped("notes").data.extensions, vec![".md".to_owned()]);
}

#[test]
fn id_outside_the_pattern_is_refused() {
    // Uppercase is outside `[a-z][a-z0-9-]{1,31}`.
    let manifest = minimal_manifest("Excalidraw");
    assert_eq!(
        validate(&manifest),
        Err(PluginError::InvalidId {
            id: "Excalidraw".to_owned()
        })
    );
}

#[test]
fn extension_without_a_leading_dot_is_refused() {
    let mut manifest = minimal_manifest("drawing");
    manifest.data.extensions = vec!["excalidraw".to_owned()];
    assert_eq!(
        validate(&manifest),
        Err(PluginError::InvalidExtension {
            plugin: "drawing".to_owned(),
            extension: "excalidraw".to_owned(),
        })
    );
}

#[test]
fn extension_containing_a_slash_is_refused() {
    let mut manifest = minimal_manifest("drawing");
    manifest.data.extensions = vec!["./nested/.excalidraw".to_owned()];
    assert_eq!(
        validate(&manifest),
        Err(PluginError::InvalidExtension {
            plugin: "drawing".to_owned(),
            extension: "./nested/.excalidraw".to_owned(),
        })
    );
}

#[test]
fn max_bytes_above_the_ceiling_is_refused() {
    let mut manifest = minimal_manifest("drawing");
    manifest.data.max_bytes = MAX_DATA_BYTES + 1.0;
    assert_eq!(
        validate(&manifest),
        Err(PluginError::InvalidMaxBytes {
            plugin: "drawing".to_owned(),
            max_bytes: MAX_DATA_BYTES + 1.0,
        })
    );
}

/// NaN fails every comparison and infinity overflows the ceiling arithmetic,
/// so neither may pass for a byte count. `matches!`: NaN is never equal to NaN.
#[test]
fn max_bytes_that_is_no_whole_byte_count_is_refused() {
    for max_bytes in [
        f64::NAN,
        f64::INFINITY,
        f64::NEG_INFINITY,
        -1.0,
        0.0,
        0.5,
        1024.5,
    ] {
        let mut manifest = minimal_manifest("drawing");
        manifest.data.max_bytes = max_bytes;
        assert!(
            matches!(
                validate(&manifest),
                Err(PluginError::InvalidMaxBytes { .. })
            ),
            "{max_bytes} was accepted"
        );
    }
}

#[test]
fn extension_that_is_a_bare_dot_or_carries_what_a_name_may_not_is_refused() {
    for extension in [".", ".\\x", ".\0", ".a b", "..x"] {
        let mut manifest = minimal_manifest("drawing");
        manifest.data.extensions = vec![extension.to_owned()];
        assert!(
            matches!(
                validate(&manifest),
                Err(PluginError::InvalidExtension { .. })
            ),
            "{extension:?} was accepted"
        );
    }
    let mut manifest = minimal_manifest("drawing");
    manifest.data.extensions = vec![".excalidraw".to_owned(), ".a-b_1".to_owned()];
    assert_eq!(validate(&manifest), Ok(()));
}

#[test]
fn duplicate_id_in_the_catalogue_is_refused() {
    let manifests = vec![minimal_manifest("drawing"), minimal_manifest("drawing")];
    assert_eq!(
        validate_catalogue(&manifests),
        Err(PluginError::DuplicateId {
            id: "drawing".to_owned()
        })
    );
}

#[test]
fn every_manifest_in_the_catalogue_passes_validation() {
    assert_eq!(validate_catalogue(&catalogue()), Ok(()));
}

/// The old `drawing` rows were written into this plugin's folder; a plugin
/// renamed away from it would leave them where nothing reads.
#[test]
fn drawings_moved_into_a_plugin_the_catalogue_ships() {
    let shipped = catalogue()
        .into_iter()
        .find(|one| one.id == devpit_core::store::DRAWINGS_PLUGIN)
        .expect("the drawings' plugin is shipped");
    assert!(shipped
        .data
        .extensions
        .iter()
        .any(|extension| extension == devpit_core::store::DRAWING_EXTENSION));
}
