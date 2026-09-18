//! The release manifest, written from artifacts that verify.
//!
//! `cargo xtask release-manifest <version>` reads the bundles the release
//! build left behind, checks each one's signature against the public key the
//! app itself carries, and only then writes the manifests the updater reads
//! and a `SHA256SUMS` beside them.
//!
//! The order is the point. A manifest published before the signatures are
//! checked is a manifest that can name a file no installed app will accept —
//! and the app is right to refuse it, which leaves everyone stuck on the
//! version they have with no way to be told why.

use std::path::{Path, PathBuf};

use base64::{engine::general_purpose::STANDARD, Engine};
use sha2::{Digest, Sha256};

/// One built artifact: what the updater downloads, and the signature beside it.
pub(crate) struct Artifact {
    /// `appimage`, `deb` or `app` — the installer the plugin detects and
    /// substitutes into the endpoint as `{{bundle_type}}`.
    pub kind: String,
    pub file: PathBuf,
    /// The `.sig` beside it, as published: base64 of the minisign file.
    pub signature: String,
    /// The key the updater looks itself up by: `linux-x86_64`,
    /// `darwin-aarch64`, `darwin-x86_64`.
    pub target: String,
}

/// The static manifest the updater reads for one installer.
///
/// One installer can be several platforms: a `.app.tar.gz` is built once for
/// Apple silicon and once for Intel, and both belong in `latest-app.json`
/// because the app asks for the file by its bundle type and finds itself in
/// `platforms` by its own target.
///
/// Pure, so a test can read what it wrote without a release to hand.
pub(crate) fn manifest(
    version: &str,
    notes: &str,
    date: &str,
    platforms: &[(String, String, String)],
) -> serde_json::Value {
    let mut said = serde_json::Map::new();
    for (target, url, signature) in platforms {
        said.insert(
            target.clone(),
            serde_json::json!({ "url": url, "signature": signature }),
        );
    }
    serde_json::json!({
        "version": version,
        "notes": notes,
        "pub_date": date,
        "platforms": serde_json::Value::Object(said),
    })
}

/// Whether a signature was made by the key the app carries.
///
/// Both sides arrive base64-encoded — the public key as it sits in
/// `tauri.conf.json`, the signature as it is published — and both are the
/// whole minisign file, comment line and all. Measured in US-009 against the
/// plugin's own `verify_signature`.
pub(crate) fn verified(pubkey_b64: &str, signature_b64: &str, bytes: &[u8]) -> Result<(), String> {
    let pub_text = decoded(pubkey_b64)?;
    let sig_text = decoded(signature_b64)?;
    let key = minisign_verify::PublicKey::decode(&pub_text).map_err(|err| err.to_string())?;
    let signature = minisign_verify::Signature::decode(&sig_text).map_err(|err| err.to_string())?;
    key.verify(bytes, &signature, false)
        .map_err(|err| err.to_string())
}

fn decoded(b64: &str) -> Result<String, String> {
    let raw = STANDARD.decode(b64.trim()).map_err(|err| err.to_string())?;
    String::from_utf8(raw).map_err(|err| err.to_string())
}

/// `sha256sum`'s own format, so the file can be checked with it.
pub(crate) fn checksums(files: &[(String, Vec<u8>)]) -> String {
    let mut said = String::new();
    for (name, bytes) in files {
        let digest = Sha256::digest(bytes);
        said.push_str(&format!("{digest:x}  {name}\n"));
    }
    said
}

/// The public key the app ships, out of its own config.
pub(crate) fn pubkey_of(root: &Path) -> Option<String> {
    let text = std::fs::read_to_string(root.join("apps/desktop/tauri.conf.json")).ok()?;
    let conf: serde_json::Value = serde_json::from_str(&text).ok()?;
    conf.pointer("/plugins/updater/pubkey")?
        .as_str()
        .map(ToOwned::to_owned)
}

/// The bundles a release build left, each with the signature beside it.
///
/// Three kinds, because three is what the updater can install in place: an
/// AppImage and a `.deb` on Linux, and on macOS the `.app.tar.gz` the plugin
/// unpacks over the installed app. The `.dmg` is a download for a person, not
/// an update, so it is published and never named in a manifest.
pub(crate) fn artifacts_in(bundle: &Path, target: &str) -> Vec<Artifact> {
    let mut found = Vec::new();
    for (kind, extension) in [
        ("appimage", ".AppImage"),
        ("deb", ".deb"),
        ("macos", ".app.tar.gz"),
    ] {
        let dir = bundle.join(kind);
        let Ok(entries) = std::fs::read_dir(&dir) else {
            continue;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            let name = path
                .file_name()
                .and_then(|name| name.to_str())
                .unwrap_or("");
            if !name.ends_with(extension) {
                continue;
            }
            let beside = PathBuf::from(format!("{}.sig", path.display()));
            let Ok(signature) = std::fs::read_to_string(&beside) else {
                continue;
            };
            found.push(Artifact {
                /* The folder is `macos` and the bundle type the app asks for
                is `app`: the endpoint is templated with what the updater
                calls it, not with where the bundler put it. */
                kind: if kind == "macos" {
                    "app".to_owned()
                } else {
                    kind.to_owned()
                },
                file: path,
                signature: signature.trim().to_owned(),
                target: target.to_owned(),
            });
        }
    }
    found
}

/// The target of a build made here, for when there is no `collected/` tree to
/// read one out of.
///
/// Derived rather than written down: `linux-x86_64` as a constant was right
/// until the release grew an arm64 leg, and the way it would have been wrong
/// is a local manifest quietly claiming somebody else's architecture.
pub(crate) fn here() -> String {
    let os = match std::env::consts::OS {
        "macos" => "darwin",
        other => other,
    };
    format!("{os}-{}", std::env::consts::ARCH)
}

/// Every folder of bundles to read, each with the target it was built for.
///
/// A release that builds on three runners collects them under
/// `target/release/collected/<name>/`, each holding the `bundle/` tree one
/// runner made and a `target.txt` saying whose it is. With no such folder this
/// is the ordinary local build: one tree, this machine's target.
fn bundles_in(root: &Path) -> Vec<(PathBuf, Vec<String>)> {
    let collected = root.join("target/release/collected");
    let mut found = Vec::new();
    if let Ok(entries) = std::fs::read_dir(&collected) {
        for entry in entries.flatten() {
            let dir = entry.path();
            if !dir.is_dir() {
                continue;
            }
            let targets = std::fs::read_to_string(dir.join("target.txt"))
                .map(|said| targets_named(&said))
                .ok()
                .filter(|named: &Vec<String>| !named.is_empty())
                .unwrap_or_else(|| vec![here()]);
            found.push((dir.join("bundle"), targets));
        }
    }
    if found.is_empty() {
        found.push((root.join("target/release/bundle"), vec![here()]));
    }
    found
}

/// The targets one runner's bundles answer for, one per line.
///
/// More than one because a universal macOS bundle is a single file that an
/// Intel Mac and an Apple silicon Mac must both find. The updater resolves by
/// target key and has no idea the two keys point at the same download, so the
/// manifest has to say it twice.
pub(crate) fn targets_named(said: &str) -> Vec<String> {
    said.lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .map(ToOwned::to_owned)
        .collect()
}

/// Where a published artifact lives, once the tag exists.
fn url_for(version: &str, name: &str) -> String {
    format!("https://github.com/jholhewres/devpit/releases/download/v{version}/{name}")
}

/// Writes one manifest per installer, and a `SHA256SUMS` beside them.
///
/// Nothing is written until every artifact has verified: a half-written
/// release is one the workflow would happily publish.
pub fn run(root: &Path, version: &str, notes: &str, date: &str) -> Result<Vec<PathBuf>, String> {
    let pubkey = pubkey_of(root).ok_or("the app carries no public key to verify against")?;
    let dirs = bundles_in(root);
    let artifacts: Vec<Artifact> = dirs
        .iter()
        .flat_map(|(bundle, targets)| {
            targets
                .iter()
                .flat_map(|target| artifacts_in(bundle, target))
                .collect::<Vec<_>>()
        })
        .collect();
    if artifacts.is_empty() {
        return Err(format!(
            "no signed artifacts under {} — build with the release overlay first",
            dirs.iter()
                .map(|(dir, _)| dir.display().to_string())
                .collect::<Vec<_>>()
                .join(", ")
        ));
    }

    let mut checked = Vec::new();
    for artifact in &artifacts {
        let name = artifact
            .file
            .file_name()
            .and_then(|name| name.to_str())
            .ok_or("an artifact with no name")?
            .to_owned();
        let bytes = std::fs::read(&artifact.file).map_err(|err| format!("{name}: {err}"))?;
        verified(&pubkey, &artifact.signature, &bytes)
            .map_err(|why| format!("{name} does not verify against the app's key: {why}"))?;
        checked.push((
            artifact.kind.clone(),
            artifact.target.clone(),
            name,
            bytes,
            artifact.signature.clone(),
        ));
    }

    let out = root.join("target/release/manifests");
    std::fs::create_dir_all(&out).map_err(|err| err.to_string())?;

    /* One manifest per installer, every platform that built it inside — so
    `latest-app.json` names both Macs and an app on either finds itself. */
    let mut kinds: Vec<String> = Vec::new();
    for (kind, ..) in &checked {
        if !kinds.contains(kind) {
            kinds.push(kind.clone());
        }
    }

    let mut written = Vec::new();
    for kind in &kinds {
        let platforms: Vec<(String, String, String)> = checked
            .iter()
            .filter(|(one, ..)| one == kind)
            .map(|(_, target, name, _, signature)| {
                (target.clone(), url_for(version, name), signature.clone())
            })
            .collect();
        let path = out.join(format!("latest-{kind}.json"));
        let json = manifest(version, notes, date, &platforms);
        std::fs::write(&path, format!("{json:#}\n")).map_err(|err| err.to_string())?;
        written.push(path);
    }

    let sums = out.join("SHA256SUMS");
    let listed: Vec<(String, Vec<u8>)> = checked
        .iter()
        .map(|(_, _, name, bytes, _)| (name.clone(), bytes.clone()))
        .collect();
    std::fs::write(&sums, checksums(&listed)).map_err(|err| err.to_string())?;
    written.push(sums);

    Ok(written)
}

#[cfg(test)]
#[path = "release_manifest_tests.rs"]
mod tests;
