//! Where the sealed blobs are kept: devpit.app's `/v1/sync`, as the account
//! this machine is signed in as. It sees names, versions and sizes, and
//! bytes it holds no key to.

use std::time::Duration;

use serde::Deserialize;

use crate::error::SyncError;

/// The most a sealed blob may weigh, here and on devpit.app.
pub const MOST_BLOB: usize = 1024 * 1024;
/// The most a listing is read to: 256 names and their versions are far less.
const MOST_LISTING: usize = 256 * 1024;
/// The header a blob's version comes back in.
const VERSION: &str = "x-sync-version";

/// A blob as the listing names it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Listed {
    pub name: String,
    pub version: u64,
}

/// What keeps the blobs: devpit.app, or a stand-in in a test.
#[allow(async_fn_in_trait)]
pub trait Blobs {
    async fn list(&self) -> Result<Vec<Listed>, SyncError>;
    /// The blob's version and bytes, or nothing when there is none.
    async fn get(&self, name: &str) -> Result<Option<(u64, Vec<u8>)>, SyncError>;
    /// Writes `bytes` over the version `base` (0: a blob that must not exist
    /// yet); the version kept, or [`SyncError::Stale`].
    async fn put(&self, name: &str, base: u64, bytes: Vec<u8>) -> Result<u64, SyncError>;
    /// Deletes every blob of the account.
    async fn clear(&self) -> Result<(), SyncError>;
}

/// devpit.app, as the signed-in account.
pub struct DevpitApp {
    client: reqwest::Client,
    origin: String,
    token: String,
}

impl std::fmt::Debug for DevpitApp {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // The session's token never reaches a log.
        write!(f, "DevpitApp {{ origin: {} }}", self.origin)
    }
}

#[derive(Deserialize)]
struct Listing {
    blobs: Vec<ListedBlob>,
}

#[derive(Deserialize)]
struct ListedBlob {
    name: String,
    version: u64,
}

#[derive(Deserialize)]
struct Stored {
    version: u64,
}

#[derive(Deserialize)]
struct Refusal {
    message: String,
}

impl DevpitApp {
    pub fn new(origin: &str, token: &str) -> Result<DevpitApp, SyncError> {
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(30))
            .user_agent(concat!("devpit/", env!("CARGO_PKG_VERSION")))
            .build()
            .map_err(|err| SyncError::Local(format!("no http client: {err}")))?;
        Ok(DevpitApp {
            client,
            origin: origin.trim_end_matches('/').to_owned(),
            token: token.to_owned(),
        })
    }

    fn url(&self, path: &str) -> String {
        format!("{}/v1/sync{path}", self.origin)
    }

    async fn send(&self, request: reqwest::RequestBuilder) -> Result<reqwest::Response, SyncError> {
        request
            .bearer_auth(&self.token)
            .send()
            .await
            .map_err(|err| SyncError::Unreachable(err.without_url().to_string()))
    }
}

/// A response's body, read only as far as `most`.
async fn body_within(mut response: reqwest::Response, most: usize) -> Result<Vec<u8>, SyncError> {
    let past = || SyncError::TooLarge(format!("devpit.app answered more than {most} bytes"));
    if response
        .content_length()
        .is_some_and(|size| size > most as u64)
    {
        return Err(past());
    }
    let mut body = Vec::new();
    while let Some(chunk) = response
        .chunk()
        .await
        .map_err(|err| SyncError::Unreachable(err.without_url().to_string()))?
    {
        if body.len() + chunk.len() > most {
            return Err(past());
        }
        body.extend_from_slice(&chunk);
    }
    Ok(body)
}

/// A refusal as the person is told it: what devpit.app said, by its status.
async fn refused(response: reqwest::Response) -> SyncError {
    let status = response.status();
    let said = body_within(response, 16 * 1024)
        .await
        .ok()
        .and_then(|body| serde_json::from_slice::<Refusal>(&body).ok())
        .map(|refusal| refusal.message.chars().take(300).collect::<String>())
        .unwrap_or_else(|| status.to_string());
    match status.as_u16() {
        401 => SyncError::SignedOut,
        409 => SyncError::Stale,
        413 => SyncError::TooLarge(said),
        _ => SyncError::Refused(said),
    }
}

impl Blobs for DevpitApp {
    async fn list(&self) -> Result<Vec<Listed>, SyncError> {
        let response = self.send(self.client.get(self.url(""))).await?;
        if !response.status().is_success() {
            return Err(refused(response).await);
        }
        let listing: Listing = serde_json::from_slice(&body_within(response, MOST_LISTING).await?)
            .map_err(|_| SyncError::Refused("devpit.app listed something unreadable".to_owned()))?;
        Ok(listing
            .blobs
            .into_iter()
            .map(|blob| Listed {
                name: blob.name,
                version: blob.version,
            })
            .collect())
    }

    async fn get(&self, name: &str) -> Result<Option<(u64, Vec<u8>)>, SyncError> {
        let response = self
            .send(self.client.get(self.url(&format!("/{name}"))))
            .await?;
        if response.status() == reqwest::StatusCode::NOT_FOUND {
            return Ok(None);
        }
        if !response.status().is_success() {
            return Err(refused(response).await);
        }
        let version = response
            .headers()
            .get(VERSION)
            .and_then(|value| value.to_str().ok())
            .and_then(|value| value.parse::<u64>().ok())
            .ok_or_else(|| {
                SyncError::Refused("devpit.app sent a blob without its version".to_owned())
            })?;
        Ok(Some((version, body_within(response, MOST_BLOB).await?)))
    }

    async fn put(&self, name: &str, base: u64, bytes: Vec<u8>) -> Result<u64, SyncError> {
        if bytes.len() > MOST_BLOB {
            return Err(too_large(bytes.len()));
        }
        let request = self
            .client
            .put(self.url(&format!("/{name}?version={base}")))
            .header(reqwest::header::CONTENT_TYPE, "application/octet-stream")
            .body(bytes);
        let response = self.send(request).await?;
        if !response.status().is_success() {
            return Err(refused(response).await);
        }
        let stored: Stored = serde_json::from_slice(&body_within(response, 16 * 1024).await?)
            .map_err(|_| SyncError::Refused("devpit.app answered a write unreadably".to_owned()))?;
        Ok(stored.version)
    }

    async fn clear(&self) -> Result<(), SyncError> {
        let response = self.send(self.client.delete(self.url(""))).await?;
        if !response.status().is_success() {
            return Err(refused(response).await);
        }
        Ok(())
    }
}

/// Said when a blob is past [`MOST_BLOB`], before it leaves this machine.
pub fn too_large(size: usize) -> SyncError {
    SyncError::TooLarge(format!(
        "what syncs is {} KiB sealed, and a blob is at most {} KiB",
        size.div_ceil(1024),
        MOST_BLOB / 1024
    ))
}
