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
    /// `appimage` or `deb` — the installer the plugin detects and substitutes
    /// into the endpoint as `{{bundle_type}}`.
    pub kind: String,
    pub file: PathBuf,
    /// The `.sig` beside it, as published: base64 of the minisign file.
    pub signature: String,
}

/// The static manifest the updater reads for one installer.
///
/// Pure, so a test can read what it wrote without a release to hand.
pub(crate) fn manifest(
    version: &str,
    notes: &str,
    date: &str,
    target: &str,
    url: &str,
    signature: &str,
) -> serde_json::Value {
    serde_json::json!({
        "version": version,
        "notes": notes,
        "pub_date": date,
        "platforms": {
            target: { "url": url, "signature": signature }
        }
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
pub(crate) fn artifacts_in(bundle: &Path) -> Vec<Artifact> {
    let mut found = Vec::new();
    for (kind, extension) in [("appimage", "AppImage"), ("deb", "deb")] {
        let dir = bundle.join(kind);
        let Ok(entries) = std::fs::read_dir(&dir) else {
            continue;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().and_then(|e| e.to_str()) != Some(extension) {
                continue;
            }
            let beside = PathBuf::from(format!("{}.sig", path.display()));
            let Ok(signature) = std::fs::read_to_string(&beside) else {
                continue;
            };
            found.push(Artifact {
                kind: kind.to_owned(),
                file: path,
                signature: signature.trim().to_owned(),
            });
        }
    }
    found
}

/// The target key the updater looks itself up by.
const TARGET: &str = "linux-x86_64";

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
    let bundle = root.join("target/release/bundle");
    let artifacts = artifacts_in(&bundle);
    if artifacts.is_empty() {
        return Err(format!(
            "no signed artifacts under {} — build with the release overlay first",
            bundle.display()
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
            name,
            bytes,
            artifact.signature.clone(),
        ));
    }

    let out = root.join("target/release/manifests");
    std::fs::create_dir_all(&out).map_err(|err| err.to_string())?;

    let mut written = Vec::new();
    for (kind, name, _, signature) in &checked {
        let path = out.join(format!("latest-{kind}.json"));
        let json = manifest(
            version,
            notes,
            date,
            TARGET,
            &url_for(version, name),
            signature,
        );
        std::fs::write(&path, format!("{json:#}\n")).map_err(|err| err.to_string())?;
        written.push(path);
    }

    let sums = out.join("SHA256SUMS");
    let listed: Vec<(String, Vec<u8>)> = checked
        .iter()
        .map(|(_, name, bytes, _)| (name.clone(), bytes.clone()))
        .collect();
    std::fs::write(&sums, checksums(&listed)).map_err(|err| err.to_string())?;
    written.push(sums);

    Ok(written)
}

#[cfg(test)]
#[path = "release_manifest_tests.rs"]
mod tests;
