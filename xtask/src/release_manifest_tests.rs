//! What the release publishes, and what it refuses to publish.
//!
//! The fixture is real: a throwaway key pair signed the payload below, and the
//! second signature comes from a different key. Only public halves are here.

use super::*;

/// The payload both signatures were made over.
const PAYLOAD: &[u8] = b"hello from the release\n";

/// The throwaway public key, base64 as it sits in `tauri.conf.json`.
const PUBKEY: &str = "dW50cnVzdGVkIGNvbW1lbnQ6IG1pbmlzaWduIHB1YmxpYyBrZXk6IDcxMDE5QTFEQ0UyRTkxQTkKUldTcGtTN09IWm9CY1hJZEh0Tkg3MERPM0hVR1UyaVR6V3g2VE9CaU5mUk9HR1RGb0UyNGRzaFEK";

/// Its signature over PAYLOAD, base64 as it is published.
const SIGNATURE: &str = "dW50cnVzdGVkIGNvbW1lbnQ6IHNpZ25hdHVyZSBmcm9tIHRhdXJpIHNlY3JldCBrZXkKUlVTcGtTN09IWm9CY1dHYytZcDZZSVJ1RzNVZjVOOTZqcTZWR0ZMZ0VCTHVibmQyWWdtU0hnV3U0Q2dJUXR1Z2FyOThvcUJQM1VLREE2Zkt3SHBPNVNPSzNSSCt1SUhNbEEwPQp0cnVzdGVkIGNvbW1lbnQ6IHRpbWVzdGFtcDoxNzg5NTI4ODQ1CWZpbGU6YXJ0aWZhY3QuYmluCmFlU1RMN29GQXMrOUh1VFI3Y1p6RHoxZW40Rmd6SkMzRGNpSTVGYW1SYStHMUVteXVncS9pZnY1MVJ4ZVVDR1I4My95RkNYdWtpRkxFS3BkWURIRUNRPT0K";

/// A signature over the same bytes, made by another key.
const ANOTHER_KEYS_SIGNATURE: &str = "dW50cnVzdGVkIGNvbW1lbnQ6IHNpZ25hdHVyZSBmcm9tIHRhdXJpIHNlY3JldCBrZXkKUlVSVzZLTFJybGF3dGFaeVBLM2E2SFRwZzVyckpWckl4TnFKajVZS1RldlBzUy85ZjJoSmptY0ZGV292d3hGL2VnV1hHdlA2dEc4dlBvTTE3MWdWMnk2VEl6U05ZR1dJR2djPQp0cnVzdGVkIGNvbW1lbnQ6IHRpbWVzdGFtcDoxNzg5NTI4ODcwCWZpbGU6b3RoZXIuYmluCnUvd3huYzVVaXM0UkVLUXB5UWRremZJaXFJL3ZSR0psMDRaZFZWVDREb2UySTRUeUtRUkR0UEFxWW5hdXZKK0xVc0dmdGNQS3A0bVJtaEdodjN1ZkF3PT0K";

#[test]
fn a_signature_from_the_key_the_app_carries_is_accepted() {
    assert_eq!(verified(PUBKEY, SIGNATURE, PAYLOAD), Ok(()));
}

/// The sabotage this command exists for: an artifact signed with a different
/// key is refused before anything is written.
#[test]
fn a_signature_from_another_key_is_refused() {
    let refused = verified(PUBKEY, ANOTHER_KEYS_SIGNATURE, PAYLOAD)
        .expect_err("another key's signature was accepted");
    assert!(refused.contains("different key"), "{refused}");

    // And so is a signature over other bytes.
    assert!(verified(PUBKEY, SIGNATURE, b"not what was signed").is_err());
}

#[test]
fn the_manifest_is_the_shape_the_updater_reads() {
    let written = manifest(
        "0.2.0",
        "what changed",
        "2026-09-16T00:00:00Z",
        &[(
            "linux-x86_64".to_owned(),
            "https://example.invalid/devpit_0.2.0_amd64.AppImage".to_owned(),
            SIGNATURE.to_owned(),
        )],
    );

    assert_eq!(written["version"], "0.2.0");
    assert_eq!(written["notes"], "what changed");
    assert_eq!(written["pub_date"], "2026-09-16T00:00:00Z");
    let platform = &written["platforms"]["linux-x86_64"];
    assert_eq!(
        platform["url"],
        "https://example.invalid/devpit_0.2.0_amd64.AppImage"
    );
    assert_eq!(platform["signature"], SIGNATURE);
}

/* One installer, two Macs: the app asks for `latest-app.json` by its bundle
type and finds itself in `platforms` by its target, so both have to be in
the one file. Sabotage: write a manifest per platform and the Intel Mac
downloads the Apple silicon build. */
#[test]
fn one_installer_carries_every_platform_that_built_it() {
    let written = manifest(
        "0.2.0",
        "",
        "2026-09-16T00:00:00Z",
        &[
            (
                "darwin-aarch64".to_owned(),
                "https://example.invalid/devpit_aarch64.app.tar.gz".to_owned(),
                SIGNATURE.to_owned(),
            ),
            (
                "darwin-x86_64".to_owned(),
                "https://example.invalid/devpit_x64.app.tar.gz".to_owned(),
                SIGNATURE.to_owned(),
            ),
        ],
    );

    let platforms = written["platforms"].as_object().expect("platforms");
    assert_eq!(platforms.len(), 2);
    assert_eq!(
        platforms["darwin-x86_64"]["url"],
        "https://example.invalid/devpit_x64.app.tar.gz"
    );
}

#[test]
fn the_checksums_are_the_format_sha256sum_reads() {
    let said = checksums(&[("devpit.deb".to_owned(), b"one".to_vec())]);
    // sha256 of "one", then two spaces, then the name.
    assert_eq!(
        said,
        "7692c3ad3540bb803c020b3aee66cd8887123234ea0c6e7143c0add73ff431ed  devpit.deb\n"
    );
}

#[test]
fn the_app_carries_a_public_key_to_verify_against() {
    assert!(pubkey_of(&crate::workspace_root()).is_some());
}

/// The target a local build claims. Written down as `linux-x86_64` until the
/// release grew an arm64 leg, and the failure would have been a manifest
/// quietly naming an architecture the machine is not.
#[test]
fn the_local_target_is_this_machine_and_not_a_constant() {
    let said = here();
    assert!(said.contains(std::env::consts::ARCH), "{said}");
    assert!(
        !said.contains("macos"),
        "darwin is the updater's word: {said}"
    );
    if std::env::consts::OS == "macos" {
        assert!(said.starts_with("darwin-"), "{said}");
    } else {
        assert!(said.starts_with(std::env::consts::OS), "{said}");
    }
}

/// One runner, two target keys. A universal macOS bundle is a single file
/// that both Macs download, and the updater looks itself up by target with no
/// idea the two keys point at the same place.
#[test]
fn a_leg_may_answer_for_more_than_one_target() {
    assert_eq!(
        targets_named("darwin-aarch64\ndarwin-x86_64\n"),
        ["darwin-aarch64", "darwin-x86_64"]
    );
    /* The shape every other leg writes, unchanged. */
    assert_eq!(targets_named("linux-x86_64\n"), ["linux-x86_64"]);
    /* Blank lines and stray spaces are the workflow's echo, not targets. */
    assert_eq!(targets_named("\n  linux-aarch64  \n\n"), ["linux-aarch64"]);
    assert!(targets_named("   \n\n").is_empty());
}

/// And the manifest that comes out of it: two platforms, one download.
#[test]
fn two_platforms_may_name_the_same_download() {
    let one = (
        "devpit.app.tar.gz".to_owned(),
        "https://example.invalid/devpit.app.tar.gz".to_owned(),
        "signature".to_owned(),
    );
    let platforms = [
        ("darwin-aarch64".to_owned(), one.1.clone(), one.2.clone()),
        ("darwin-x86_64".to_owned(), one.1.clone(), one.2.clone()),
    ];
    let said = manifest("0.2.0", "", "2026-01-01T00:00:00Z", &platforms);
    let found = said["platforms"]
        .as_object()
        .expect("platforms is an object");
    assert_eq!(found.len(), 2, "an Intel Mac has to find itself in here");
    assert_eq!(
        found["darwin-aarch64"]["url"],
        found["darwin-x86_64"]["url"]
    );
}

/// `SHA256SUMS` is for a first download, and on a Mac that is the `.dmg`.
///
/// It was built from the updater's list, which leaves the dmg out on purpose
/// and names the universal tarball once per Mac. So it shipped without the dmg
/// and with the tarball twice — and `sha256sum -c --ignore-missing`, the
/// README's own line, checked nothing for anybody on a Mac.
#[test]
fn the_checksums_name_the_dmg_and_nothing_twice() {
    let tarball = ("devpit.app.tar.gz".to_owned(), b"app".to_vec());
    let updater = vec![
        ("devpit_0.1.7_amd64.deb".to_owned(), b"deb".to_vec()),
        tarball.clone(),
        /* The universal build, answering for the second Mac. */
        tarball.clone(),
    ];
    let dmgs = vec![("devpit_0.1.7_universal.dmg".to_owned(), b"dmg".to_vec())];

    let listed = for_a_person(&updater, dmgs);
    let names: Vec<&str> = listed.iter().map(|(name, _)| name.as_str()).collect();
    assert_eq!(
        names,
        [
            "devpit_0.1.7_amd64.deb",
            "devpit.app.tar.gz",
            "devpit_0.1.7_universal.dmg"
        ]
    );
}

/// The dmg is read from the folder the bundler writes it to, and only a dmg.
#[test]
fn the_dmgs_are_found_where_the_bundler_leaves_them() {
    let bundle = tempfile::tempdir().expect("a bundle");
    let dmg = bundle.path().join("dmg");
    std::fs::create_dir_all(&dmg).expect("the dmg folder");
    std::fs::write(dmg.join("devpit_0.1.7_universal.dmg"), b"image").expect("a dmg");
    std::fs::write(dmg.join("bundle_dmg.sh"), b"#!/bin/sh").expect("the bundler's script");

    let found = dmgs_in(bundle.path());
    assert_eq!(
        found.len(),
        1,
        "{:?}",
        found.iter().map(|(n, _)| n).collect::<Vec<_>>()
    );
    assert_eq!(found[0].0, "devpit_0.1.7_universal.dmg");
    assert_eq!(found[0].1, b"image");

    /* A Linux runner has no dmg folder at all, and that is nothing, not an error. */
    assert!(dmgs_in(&bundle.path().join("nowhere")).is_empty());
}
