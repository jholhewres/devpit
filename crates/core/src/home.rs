//! Where a project's own files live: `<root>/projects/<folder>/`.
//!
//! The one place that path is built — `cargo xtask check` refuses it anywhere
//! else. The folder is read from `project.folder`, never rebuilt from the name:
//! renaming a project must not move a folder a conversation is open in.

use std::ffi::OsStr;
use std::path::{Component, Path, PathBuf};

use crate::data_files;
use crate::store::{Store, StoreError};

const PROJECTS: &str = "projects";

/// What the bell calls a folder that could not be moved.
pub const FOLDER_NOTICE: &str = "folder";

#[derive(Debug, thiserror::Error)]
pub enum HomeError {
    #[error("not a project id")]
    Invalid,
    #[error("that project is not registered")]
    NotFound,
    #[error("that project's folder is not a name devpit gives")]
    Forbidden,
    #[error("that project's folder is not where devpit keeps it")]
    Elsewhere,
    #[error("not a capability id")]
    PluginId,
    #[error("that capability's folder is not where devpit keeps it")]
    PluginElsewhere,
    #[error("the capability's files could not be removed")]
    Removal(#[source] std::io::Error),
    #[error(transparent)]
    Store(#[from] StoreError),
}

/// The folder every project's own folder sits in.
pub fn projects_dir(root: &Path) -> PathBuf {
    root.join(PROJECTS)
}

/// One project's folder in the devpit workspace.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProjectHome {
    root: PathBuf,
    folder: String,
}

impl ProjectHome {
    /// The folder the store names for this project.
    ///
    /// A row no build has named yet keeps the folder it was made with, the id:
    /// that is where its files still are until the next start moves them.
    pub fn of(store: &Store, root: &Path, project_id: &str) -> Result<Self, HomeError> {
        if !plain_id(project_id) {
            return Err(HomeError::Invalid);
        }
        match store.project_folder(project_id)? {
            None => Err(HomeError::NotFound),
            Some(folder) => Ok(Self::named(root, project_id, folder)?.unmoved(project_id)),
        }
    }

    /// The folder under the id while the named one is not on disk.
    ///
    /// Naming commits before the move, so a failed move leaves the files under
    /// the id; a write to the named folder would then block every later move.
    fn unmoved(self, project_id: &str) -> Self {
        let named_absent = matches!(
            std::fs::symlink_metadata(self.dir()),
            Err(err) if err.kind() == std::io::ErrorKind::NotFound
        );
        // `symlink_metadata`: a link planted at the id is never followed.
        let under_id = std::fs::symlink_metadata(projects_dir(&self.root).join(project_id))
            .is_ok_and(|meta| meta.is_dir());
        if named_absent && under_id {
            Self {
                folder: project_id.to_owned(),
                ..self
            }
        } else {
            self
        }
    }

    /// A row's folder as read, for a caller holding a connection but no store.
    pub(crate) fn named(
        root: &Path,
        project_id: &str,
        folder: Option<String>,
    ) -> Result<Self, HomeError> {
        match folder {
            Some(folder) => Self::at(root, project_id, &folder),
            None if plain_id(project_id) => Ok(Self {
                root: root.to_path_buf(),
                folder: project_id.to_owned(),
            }),
            None => Err(HomeError::Invalid),
        }
    }

    /// A folder by name, refused unless it is one `folder_for` could give
    /// this id.
    ///
    /// The name comes out of a database that syncs, and it reaches
    /// `remove_dir_all`: a `..` in it is a delete of the workspace itself, and
    /// a folder of another id, or of none, is a delete of someone else's files.
    pub fn at(root: &Path, project_id: &str, folder: &str) -> Result<Self, HomeError> {
        if !folder_of_id(folder, project_id) {
            return Err(HomeError::Forbidden);
        }
        Ok(Self {
            root: root.to_path_buf(),
            folder: folder.to_owned(),
        })
    }

    pub fn dir(&self) -> PathBuf {
        projects_dir(&self.root).join(&self.folder)
    }

    /// The folder relative to the workspace root, `/` separated.
    pub fn relative(&self) -> String {
        format!("{PROJECTS}/{}", self.folder)
    }

    /// The folder's own name: one segment under `projects_dir`.
    pub fn folder(&self) -> &str {
        &self.folder
    }

    /// The folder as it is on disk, for a delete; `None` when nothing is there.
    ///
    /// Refused unless it resolves to itself, directly under `projects/`: what
    /// gets deleted is this folder, never where a link there points — another
    /// project's folder, or the workspace.
    pub fn wipeable(&self) -> Result<Option<PathBuf>, HomeError> {
        let real = match self.dir().canonicalize() {
            Ok(real) => real,
            Err(err) if err.kind() == std::io::ErrorKind::NotFound => return Ok(None),
            Err(_) => return Err(HomeError::Elsewhere),
        };
        let projects = projects_dir(&self.root)
            .canonicalize()
            .map_err(|_| HomeError::Elsewhere)?;
        let own = real.parent() == Some(projects.as_path())
            && real.file_name() == Some(OsStr::new(&self.folder));
        if !own {
            return Err(HomeError::Elsewhere);
        }
        Ok(real.is_dir().then_some(real))
    }

    /// Conversations: a transcript and a head per conversation.
    pub fn sessions(&self) -> PathBuf {
        self.dir().join("sessions")
    }

    /// What a fresh worktree of this project is prepared with.
    pub fn prime(&self) -> PathBuf {
        self.dir().join("prime.json")
    }

    /// Pictures pasted into the composer.
    pub fn pasted(&self) -> PathBuf {
        self.dir().join("pasted")
    }

    /// Each terminal's finished commands, kept across restarts.
    pub fn blocks(&self) -> PathBuf {
        self.dir().join("blocks")
    }

    /// A plugin's own files, relative to `dir()`, `/` separated.
    ///
    /// Relative so the caller resolves it against a folder it trusts, which is
    /// what catches a `data` symlinked out of it.
    pub fn plugin_data(&self, plugin_id: &str) -> Result<String, HomeError> {
        if !plain_plugin_id(plugin_id) {
            return Err(HomeError::PluginId);
        }
        Ok(format!("data/{plugin_id}"))
    }

    /// A plugin's folder as it is on disk, for a delete; `None` when nothing
    /// is there.
    ///
    /// The project folder must pass `wipeable`, and neither `data` nor the
    /// plugin's folder may be a link: a link is refused, never followed.
    pub fn plugin_data_wipeable(&self, plugin_id: &str) -> Result<Option<PathBuf>, HomeError> {
        let relative = self.plugin_data(plugin_id)?;
        let Some(project) = self.wipeable()? else {
            return Ok(None);
        };
        let mut at = project.clone();
        for segment in relative.split('/') {
            at.push(segment);
            match std::fs::symlink_metadata(&at) {
                Err(err) if err.kind() == std::io::ErrorKind::NotFound => return Ok(None),
                Err(err) => return Err(HomeError::Removal(err)),
                Ok(meta) if meta.is_dir() => {}
                Ok(_) => return Err(HomeError::PluginElsewhere),
            }
        }
        // Resolved once more, so what is deleted is exactly what was checked.
        match at.canonicalize() {
            Ok(real) if real == at => Ok(Some(real)),
            _ => Err(HomeError::PluginElsewhere),
        }
    }
}

/// Uninstalls a plugin from a project and, when `delete_data`, deletes its
/// folder; answers how many regular files went.
///
/// The folder goes before the row changes, so a refused or failed delete
/// leaves the plugin installed rather than half removed. Pins to its files stay.
pub fn uninstall_plugin(
    store: &Store,
    root: &Path,
    project_id: &str,
    plugin_id: &str,
    delete_data: bool,
) -> Result<u32, HomeError> {
    let home = ProjectHome::of(store, root, project_id)?;
    home.plugin_data(plugin_id)?;
    let removed = match delete_data
        .then(|| home.plugin_data_wipeable(plugin_id))
        .transpose()?
    {
        Some(Some(dir)) => data_files::remove_dir_counting(&dir).map_err(HomeError::Removal)?,
        _ => 0,
    };
    store.uninstall_plugin(project_id, plugin_id)?;
    Ok(removed)
}

/// `^[a-z][a-z0-9-]{1,31}$`: the ids a plugin manifest may carry, and so the
/// only names a plugin's data folder can have.
pub fn plain_plugin_id(id: &str) -> bool {
    let mut letters = id.chars();
    letters
        .next()
        .is_some_and(|first| first.is_ascii_lowercase())
        && (2..=32).contains(&id.len())
        && letters
            .all(|letter| letter.is_ascii_lowercase() || letter.is_ascii_digit() || letter == '-')
}

/// Past any name a person gives a drawing, and inside the 255 bytes every
/// filesystem devpit runs on allows a name.
const LONGEST_DATA_NAME: usize = 200;

/// Names Windows opens as a device, whatever follows the first dot.
const DEVICE_NAMES: [&str; 22] = [
    "con", "prn", "aux", "nul", "com1", "com2", "com3", "com4", "com5", "com6", "com7", "com8",
    "com9", "lpt1", "lpt2", "lpt3", "lpt4", "lpt5", "lpt6", "lpt7", "lpt8", "lpt9",
];

/// A file name a plugin may keep in its data folder: one segment on every
/// platform, so never absolute, no `..`, and not hidden — a leading dot is how
/// an unfinished write is named. Nor a name Windows reads as a device or
/// trims, nor one a listing could draw as a different name.
pub fn plain_data_name(name: &str) -> bool {
    let stem = name.split('.').next().unwrap_or_default();
    let stem = stem.trim_end_matches(' ');
    !name.is_empty()
        && name.len() <= LONGEST_DATA_NAME
        && !name.starts_with('.')
        && !name.ends_with(['.', ' '])
        && !name.contains("..")
        && !name
            .chars()
            .any(|letter| letter.is_control() || refused_in_a_name(letter))
        && !DEVICE_NAMES
            .iter()
            .any(|device| device.eq_ignore_ascii_case(stem))
}

/// Separators, what Windows refuses in a name, and the bidi controls that
/// draw a name in an order other than the one it is spelled in.
fn refused_in_a_name(letter: char) -> bool {
    matches!(
        letter,
        '/' | '\\'
            | ':'
            | '<'
            | '>'
            | '"'
            | '|'
            | '?'
            | '*'
            | '\u{202A}'..='\u{202E}'
            | '\u{2066}'..='\u{2069}'
    )
}

/// One path segment, from the alphabet ids are made of.
fn plain_id(id: &str) -> bool {
    !id.is_empty()
        && id.len() <= 64
        && id
            .chars()
            .all(|letter| letter.is_ascii_alphanumeric() || letter == '_' || letter == '-')
}

/// The longest folder name `plain_folder` accepts.
const LONGEST_FOLDER: usize = 80;

/// The fewest characters of the id a folder's suffix carries.
const SHORTEST_SUFFIX: usize = 6;

fn plain_folder(folder: &str) -> bool {
    !folder.is_empty()
        && folder.len() <= LONGEST_FOLDER
        && folder
            .chars()
            .all(|letter| letter.is_ascii_lowercase() || letter.is_ascii_digit() || letter == '-')
}

/// Whether `folder` is one `folder_for` could have given this id: a slug, then
/// a tail of the id's ULID at least six long, or the id's hash.
fn folder_of_id(folder: &str, project_id: &str) -> bool {
    plain_folder(folder)
        && folder.rsplit_once('-').is_some_and(|(slug, suffix)| {
            !slug.is_empty()
                && (suffix == hashed(project_id)
                    || (suffix.len() >= SHORTEST_SUFFIX && ulid_of(project_id).ends_with(suffix)))
        })
}

/// What follows an id's last `_`, as lowercase ASCII letters and digits.
fn ulid_of(project_id: &str) -> String {
    project_id
        .rsplit('_')
        .next()
        .unwrap_or(project_id)
        .chars()
        .filter(char::is_ascii_alphanumeric)
        .map(|letter| letter.to_ascii_lowercase())
        .collect()
}

/// Twelve hex characters of the id's SHA-256: a suffix still the same on
/// every machine when the id's ULID cannot give one.
fn hashed(project_id: &str) -> String {
    use sha2::{Digest, Sha256};

    Sha256::digest(project_id.as_bytes())
        .iter()
        .take(6)
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

/// The longest a slug gets, so a folder stays readable in a file manager.
const LONGEST_SLUG: usize = 40;

/// The folder a project gets: its name as a slug, then the end of its id.
///
/// The suffix comes from the id, not the machine, so every machine names the
/// folder the same. Six characters, or eight when six is `taken`, then as much
/// of the id as fits; the id's hash when none is free or the id is too short.
pub fn folder_for(name: &str, project_id: &str, taken: impl Fn(&str) -> bool) -> String {
    let slug = slug_of(name);
    let ulid = ulid_of(project_id);
    let hash = hashed(project_id);
    // Never past `plain_folder`'s limit, or the folder given is one it refuses.
    let room = LONGEST_FOLDER - slug.len() - 1;

    let mut suffixes = Vec::new();
    if ulid.len() >= SHORTEST_SUFFIX {
        for width in [SHORTEST_SUFFIX, 8, ulid.len()] {
            let width = width.min(ulid.len()).min(room);
            suffixes.push(&ulid[ulid.len() - width..]);
        }
    }
    // The hash last: an id sharing its tail with another still gets a folder
    // nothing else has, so the unique index never refuses the row.
    suffixes.push(&hash);

    let mut folder = String::new();
    for suffix in suffixes {
        folder = format!("{slug}-{suffix}");
        if !taken(&folder) {
            break;
        }
    }
    folder
}

fn slug_of(name: &str) -> String {
    let mut slug = String::new();
    for letter in name.chars().flat_map(char::to_lowercase) {
        if letter.is_ascii_alphanumeric() {
            slug.push(letter);
        } else if let Some(plain) = transliterated(letter) {
            slug.push_str(plain);
        } else if !slug.is_empty() && !slug.ends_with('-') {
            slug.push('-');
        }
    }
    slug.truncate(LONGEST_SLUG);
    let slug = slug.trim_end_matches('-');
    if slug.is_empty() {
        "project".to_owned()
    } else {
        slug.to_owned()
    }
}

/// The accented Latin letters a project name is likely to carry.
///
/// A table rather than a crate: anything outside it becomes a hyphen, which
/// still names a folder, and the id suffix keeps it unique.
fn transliterated(letter: char) -> Option<&'static str> {
    Some(match letter {
        'à' | 'á' | 'â' | 'ã' | 'ä' | 'å' | 'ā' | 'ă' | 'ą' => "a",
        'æ' => "ae",
        'ç' | 'ć' | 'č' => "c",
        'ď' | 'đ' => "d",
        'è' | 'é' | 'ê' | 'ë' | 'ē' | 'ę' | 'ě' => "e",
        'ì' | 'í' | 'î' | 'ï' | 'ī' => "i",
        'ł' => "l",
        'ñ' | 'ń' | 'ň' => "n",
        'ò' | 'ó' | 'ô' | 'õ' | 'ö' | 'ø' | 'ō' | 'ő' => "o",
        'œ' => "oe",
        'ř' => "r",
        'ś' | 'š' | 'ş' => "s",
        'ß' => "ss",
        'ť' | 'ţ' => "t",
        'ù' | 'ú' | 'û' | 'ü' | 'ū' | 'ů' | 'ű' => "u",
        'ý' | 'ÿ' => "y",
        'ź' | 'ż' | 'ž' => "z",
        _ => return None,
    })
}

/// Names every project's folder and moves the ones still under their id.
///
/// Run once at launch, from the app — never from `Store::open`, so a test
/// opening a store cannot move folders. Two transactions, both `IMMEDIATE`
/// for the reason `migrations::run` gives: a second copy starting at the same
/// moment waits, then finds the work done. Naming commits first so a failed
/// move can never un-name a folder that already moved.
pub fn settle(store: &Store, root: &Path) -> Result<(), StoreError> {
    immediate(store, |store| store.backfill_folders().map(|_| ()))?;
    immediate(store, |store| move_folders(store, root))
}

fn immediate(
    store: &Store,
    work: impl FnOnce(&Store) -> Result<(), StoreError>,
) -> Result<(), StoreError> {
    store.conn().execute_batch("BEGIN IMMEDIATE")?;
    match work(store) {
        Ok(()) => Ok(store.conn().execute_batch("COMMIT")?),
        Err(err) => {
            let _ = store.conn().execute_batch("ROLLBACK");
            Err(err)
        }
    }
}

fn move_folders(store: &Store, root: &Path) -> Result<(), StoreError> {
    // Pins were written canonical; the root may reach them through a symlink.
    let real = projects_dir(root).canonicalize().ok();
    for (id, folder) in store.project_folders()? {
        let (true, Ok(home)) = (plain_id(&id), ProjectHome::at(root, &id, &folder)) else {
            continue;
        };
        let old = projects_dir(root).join(&id);
        let new = home.dir();
        if old.is_dir() {
            // Relative: the notice is drawn on screen, which has no use for
            // where the home directory is.
            let refusal = if new.exists() {
                Some(format!(
                    "{} already exists, so nothing was moved.",
                    home.relative()
                ))
            } else {
                std::fs::rename(&old, &new).err().map(|err| err.to_string())
            };
            if let Some(reason) = refusal {
                tell_once(store, &id, &format!("{PROJECTS}/{id}: {reason}"))?;
                continue;
            }
        }
        store.move_pins(&old.display().to_string(), &new.display().to_string())?;
        if let Some(real) = &real {
            let (from, to) = (real.join(&id), real.join(&folder));
            store.move_pins(&from.display().to_string(), &to.display().to_string())?;
        }
    }
    Ok(())
}

/// Rings the bell about a folder that stayed put, unless it already is.
fn tell_once(store: &Store, id: &str, detail: &str) -> Result<(), StoreError> {
    let told = store
        .notices(crate::limits::NOTICES_KEPT)?
        .into_iter()
        .any(|row| {
            row.read_at.is_none()
                && row.kind == FOLDER_NOTICE
                && row.project_id.as_deref() == Some(id)
        });
    if !told {
        let title = "A project's files stayed in the folder named by its id";
        store.add_notice(Some(id), FOLDER_NOTICE, title, Some(detail), None)?;
    }
    Ok(())
}

/// A path written under a project's id, found in its named folder.
///
/// Transcripts keep the absolute path of a pasted picture, and a line already
/// written is never rewritten. `None` when the path exists as written, or
/// nothing is at the new place either.
pub fn moved(store: &Store, root: &Path, path: &Path) -> Option<PathBuf> {
    if path.exists() {
        return None;
    }
    let rest = path.strip_prefix(projects_dir(root)).ok()?;
    // Plain names only: a `..` would have this probe paths it never returns.
    if !rest
        .components()
        .all(|part| matches!(part, Component::Normal(_)))
    {
        return None;
    }
    let mut parts = rest.components();
    let id = parts.next()?.as_os_str().to_str()?;
    let found = ProjectHome::of(store, root, id)
        .ok()?
        .dir()
        .join(parts.as_path());
    found.exists().then_some(found)
}

/// What the devpit home holds that another user has no business reading: the
/// store and its journals (profile environments live there, tokens and all),
/// the port the hooks post to, the secret they carry, the commands the agent
/// CLI runs, and the account token.
const PRIVATE: &[&str] = &[
    "state.db",
    "state.db-wal",
    "state.db-shm",
    "hook-endpoint",
    "hook-auth",
    "hooks.json",
    "account-token",
];

/// Makes the devpit home private, and says what it could not do.
///
/// The directory becomes owner-only and so does every file in [`PRIVATE`] that
/// is there. An install made before this existed is tightened on the next
/// start, which is the only moment the app knows about every one of them.
///
/// **Best effort, and deliberately so.** A home on a filesystem with no modes,
/// or one owned by somebody else, is a reason to say so and carry on — an app
/// that refuses to open because it could not change a permission has turned a
/// hardening into an outage. The caller logs what comes back.
///
/// Nothing here follows a link: `set_permissions` does, so a link planted in
/// the home would have devpit change the mode of whatever it points at.
#[cfg(unix)]
pub fn harden(root: &Path) -> Vec<(PathBuf, std::io::Error)> {
    let mut refused = Vec::new();
    if let Err(err) = owner_only(root, 0o700) {
        refused.push((root.to_path_buf(), err));
    }
    for name in PRIVATE {
        let path = root.join(name);
        match owner_only(&path, 0o600) {
            Ok(()) => {}
            Err(err) if err.kind() == std::io::ErrorKind::NotFound => {}
            Err(err) => refused.push((path, err)),
        }
    }
    refused
}

#[cfg(not(unix))]
pub fn harden(_root: &Path) -> Vec<(PathBuf, std::io::Error)> {
    Vec::new()
}

/// Creates the state root owner-only, when this is the start that creates it.
///
/// Before the store opens: SQLite makes `state.db` with the umask's mode, and
/// in a folder nobody else can enter that mode reaches nobody. Left to
/// `harden`, a first start ran its whole session with the store readable by
/// every user of the machine.
pub fn make_private_root(root: &Path) -> std::io::Result<()> {
    if root.exists() {
        return Ok(());
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::DirBuilderExt;
        std::fs::DirBuilder::new()
            .recursive(true)
            .mode(0o700)
            .create(root)
    }
    #[cfg(not(unix))]
    {
        std::fs::create_dir_all(root)
    }
}

/// One path, owner-only, refusing a link rather than following it.
#[cfg(unix)]
fn owner_only(path: &Path, mode: u32) -> std::io::Result<()> {
    use std::os::unix::fs::PermissionsExt;

    let found = std::fs::symlink_metadata(path)?;
    if found.file_type().is_symlink() {
        return Err(std::io::Error::other(
            "a link, and what it points at is not devpit's to tighten",
        ));
    }
    std::fs::set_permissions(path, std::fs::Permissions::from_mode(mode))
}

/// Writes a file only its owner can read, private from the moment it exists.
///
/// The mode is given to `open`, not set afterwards: a token written first and
/// tightened second is world-readable for as long as that takes. The second
/// call is for the file that was already there, which keeps the mode it had.
pub fn write_private(path: &Path, bytes: &[u8]) -> std::io::Result<()> {
    #[cfg(unix)]
    {
        use std::io::Write;
        use std::os::unix::fs::OpenOptionsExt;

        let mut file = std::fs::OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .mode(0o600)
            .open(path)?;
        file.write_all(bytes)?;
        owner_only(path, 0o600)
    }
    #[cfg(not(unix))]
    {
        std::fs::write(path, bytes)
    }
}

#[cfg(test)]
#[path = "home_tests.rs"]
mod tests;
