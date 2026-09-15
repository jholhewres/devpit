//! What the agents on this machine spent, read from their own transcripts.
//!
//! Nothing is stored: the transcripts are the record, and a copy would be
//! wrong the moment a session wrote another line. Parsed files are kept in
//! memory by size and modification time, so reading the screen again only
//! reads what changed.

use std::collections::{BTreeMap, HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::{Mutex, OnceLock};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use devpit_agentcli::spend_prices::cost_of;
use devpit_agentcli::spend_scan::{day_of, records_in, transcripts, Record};
use devpit_core::Store;
use devpit_rpc::{
    Declared, ErrorCode, RpcError, SpendCard, SpendDay, SpendHistory, SpendInstallation, SpendRow,
    SpendSession, SpendShare, TokenCounts,
};

use crate::installations::Found;

const DAY: i64 = 86_400;

/// A day's cost, tokens, and cost and tokens by model.
type DayTally = (f64, TokenCounts, HashMap<String, (f64, f64)>);
const RECENT: usize = 20;

struct Parsed {
    size: u64,
    modified: SystemTime,
    records: Vec<Record>,
}

fn parsed() -> &'static Mutex<HashMap<PathBuf, Parsed>> {
    static PARSED: OnceLock<Mutex<HashMap<PathBuf, Parsed>>> = OnceLock::new();
    PARSED.get_or_init(Mutex::default)
}

/// A transcript's records, read again only when the file changed.
fn records_of(path: &Path) -> Vec<Record> {
    let Ok(meta) = std::fs::metadata(path) else {
        return Vec::new();
    };
    let (size, modified) = (meta.len(), meta.modified().unwrap_or(UNIX_EPOCH));
    if let Ok(cache) = parsed().lock() {
        if let Some(known) = cache
            .get(path)
            .filter(|known| known.size == size && known.modified == modified)
        {
            return known.records.clone();
        }
    }
    let records = std::fs::read_to_string(path)
        .map(|text| records_in(&text))
        .unwrap_or_default();
    if let Ok(mut cache) = parsed().lock() {
        cache.insert(
            path.to_owned(),
            Parsed {
                size,
                modified,
                records: records.clone(),
            },
        );
    }
    records
}

/// What an installation is called on the screen.
pub(crate) fn label_of(found: &Found) -> String {
    if found.profiles.is_empty() {
        found
            .directory
            .file_name()
            .map(|name| name.to_string_lossy().into_owned())
            .unwrap_or_else(|| found.directory.display().to_string())
    } else {
        found.profiles.join(", ")
    }
}

/// Whether Anthropic bills what an installation spends: not when a profile on
/// it points the CLI at another provider.
pub(crate) fn billed(found: &Found, declared: &[Declared]) -> bool {
    !declared
        .iter()
        .filter(|profile| found.profiles.contains(&profile.label))
        .flat_map(|profile| profile.env.iter())
        .any(|var| var.name == "ANTHROPIC_BASE_URL" && !var.value.contains("anthropic.com"))
}

#[derive(Default)]
struct Tally {
    cost: f64,
    tokens: TokenCounts,
    sessions: HashSet<String>,
    last: i64,
}

impl Tally {
    fn add(&mut self, record: &Record, cost: f64) {
        self.cost += cost;
        add_tokens(&mut self.tokens, record);
        self.sessions.insert(record.session_id.clone());
        self.last = self.last.max(record.at);
    }

    fn row(self, name: String) -> SpendRow {
        SpendRow {
            name,
            cost_usd: self.cost,
            tokens: self.tokens,
            sessions: self.sessions.len() as u32,
            last_active: self.last as f64,
        }
    }
}

fn add_tokens(tokens: &mut TokenCounts, record: &Record) {
    tokens.input += record.input as f64;
    tokens.output += record.output as f64;
    tokens.cache_read += record.cache_read as f64;
    tokens.cache_write += (record.cache_write_5m + record.cache_write_1h) as f64;
}

fn total(tokens: &TokenCounts) -> f64 {
    tokens.input + tokens.output + tokens.cache_read + tokens.cache_write
}

fn ranked(tallies: HashMap<String, Tally>) -> Vec<SpendRow> {
    let mut rows: Vec<SpendRow> = tallies
        .into_iter()
        .map(|(name, tally)| tally.row(name))
        .collect();
    rows.sort_by(|a, b| {
        b.cost_usd
            .total_cmp(&a.cost_usd)
            .then(total(&b.tokens).total_cmp(&total(&a.tokens)))
            .then(a.name.cmp(&b.name))
    });
    rows
}

#[derive(Default)]
struct SessionTally {
    tally: Tally,
    turns: u32,
    cwd: String,
    by_model: HashMap<String, u64>,
    installation: usize,
}

/// The history, from records already read. `now` and `days` fix the range;
/// `project_of` names a working directory, `card_of` finds a session's card.
pub(crate) fn summarize(
    read: Vec<(usize, Record)>,
    installations: Vec<SpendInstallation>,
    now: i64,
    days: u32,
    project_of: impl Fn(&str) -> String,
    card_of: impl Fn(&str) -> Option<SpendCard>,
) -> SpendHistory {
    let since = now - i64::from(days) * DAY;
    let mut seen = HashSet::new();
    let mut history = SpendHistory {
        days,
        installations,
        cost_usd: 0.0,
        tokens: TokenCounts::default(),
        sessions: 0,
        turns: 0,
        active_days: 0,
        cache_reuse: 0.0,
        daily: Vec::new(),
        models: Vec::new(),
        projects: Vec::new(),
        recent: Vec::new(),
        estimated: false,
        unpriced_tokens: 0.0,
        files: 0,
        records: 0,
        scan_ms: 0.0,
    };
    let mut daily: BTreeMap<i64, DayTally> = BTreeMap::new();
    let mut models: HashMap<String, Tally> = HashMap::new();
    let mut projects: HashMap<String, Tally> = HashMap::new();
    let mut sessions: HashMap<String, SessionTally> = HashMap::new();

    for (installation, record) in read {
        if record.at < since || record.at > now + DAY || !seen.insert(record.key.clone()) {
            continue;
        }
        let cost = cost_of(&record);
        if cost.is_none() {
            history.unpriced_tokens += record.tokens() as f64;
        }
        if cost.is_some()
            && !history
                .installations
                .get(installation)
                .is_none_or(|one| one.billed)
        {
            history.estimated = true;
        }
        let cost = cost.unwrap_or(0.0);
        history.turns += 1;
        history.cost_usd += cost;
        add_tokens(&mut history.tokens, &record);

        let day = daily.entry(record.at.div_euclid(DAY)).or_default();
        day.0 += cost;
        add_tokens(&mut day.1, &record);
        let share = day.2.entry(record.model.clone()).or_default();
        share.0 += cost;
        share.1 += record.tokens() as f64;

        models
            .entry(record.model.clone())
            .or_default()
            .add(&record, cost);
        projects
            .entry(project_of(&record.cwd))
            .or_default()
            .add(&record, cost);
        let session = sessions.entry(record.session_id.clone()).or_default();
        session.tally.add(&record, cost);
        session.turns += 1;
        session.installation = installation;
        if session.cwd.is_empty() {
            session.cwd = record.cwd.clone();
        }
        *session.by_model.entry(record.model.clone()).or_default() += record.tokens();
    }

    history.active_days = daily.len() as u32;
    let input = history.tokens.input + history.tokens.cache_read + history.tokens.cache_write;
    history.cache_reuse = if input > 0.0 {
        history.tokens.cache_read / input
    } else {
        0.0
    };
    history.daily = (since.div_euclid(DAY) + 1..=now.div_euclid(DAY))
        .map(|day| {
            let (cost, tokens, shares) = daily.remove(&day).unwrap_or_default();
            let mut by_model: Vec<SpendShare> = shares
                .into_iter()
                .map(|(name, (cost_usd, tokens))| SpendShare {
                    name,
                    cost_usd,
                    tokens,
                })
                .collect();
            by_model.sort_by(|a, b| {
                b.cost_usd
                    .total_cmp(&a.cost_usd)
                    .then(b.tokens.total_cmp(&a.tokens))
            });
            SpendDay {
                day: day_of(day * DAY),
                cost_usd: cost,
                tokens,
                by_model,
            }
        })
        .collect();
    history.models = ranked(models);
    history.projects = ranked(projects);
    history.sessions = sessions.len() as u32;
    let mut recent: Vec<(String, SessionTally)> = sessions.into_iter().collect();
    recent.sort_by(|a, b| b.1.tally.last.cmp(&a.1.tally.last).then(a.0.cmp(&b.0)));
    history.recent = recent
        .into_iter()
        .take(RECENT)
        .map(|(session_id, session)| SpendSession {
            card: card_of(&session_id),
            project: project_of(&session.cwd),
            model: session
                .by_model
                .iter()
                .max_by(|a, b| a.1.cmp(b.1).then(b.0.cmp(a.0)))
                .map(|(model, _)| model.clone())
                .unwrap_or_default(),
            turns: session.turns,
            installation: history
                .installations
                .get(session.installation)
                .map(|one| one.label.clone())
                .unwrap_or_default(),
            cost_usd: session.tally.cost,
            last_active: session.tally.last as f64,
            tokens: session.tally.tokens,
            session_id,
        })
        .collect();
    history
}

/// The card a session belongs to: its run, its background link or its pane.
fn card_of_session(store: &Store, session_id: &str) -> Option<SpendCard> {
    let card_id = match store.session_holder(session_id).ok().flatten() {
        Some((card, _)) => card,
        None => {
            let leaf = store.pane_of_session(session_id).ok().flatten()?;
            crate::card_route::card_of_leaf(store, &leaf)?.card_id
        }
    };
    let title = store.card(&card_id).ok().flatten()?.title;
    Some(SpendCard { id: card_id, title })
}

fn history(
    project_id: Option<String>,
    installation: Option<String>,
    days: u32,
) -> Result<SpendHistory, RpcError> {
    let started = Instant::now();
    let store = crate::projects::store()?;
    let found = crate::installations::found()?;
    let declared = crate::agent_profiles::declared(&store);
    let installations: Vec<SpendInstallation> = found
        .iter()
        .map(|one| SpendInstallation {
            directory: one.directory.display().to_string(),
            label: label_of(one),
            billed: billed(one, &declared),
        })
        .collect();
    let chosen: Vec<usize> = match installation.as_deref() {
        None => (0..found.len()).collect(),
        Some(wanted) => vec![found
            .iter()
            .position(|one| one.directory == Path::new(wanted))
            .ok_or_else(|| {
                RpcError::new(
                    ErrorCode::Forbidden,
                    "that is not an installation any profile runs against",
                )
            })?],
    };

    // A project's work happens in its folder and in its cards' checkouts.
    let scope: Option<Vec<String>> = match &project_id {
        None => None,
        Some(id) => {
            let (_, root) = crate::projects::locate(&store, id)?;
            let checkouts = Store::root()
                .map_err(|err| RpcError::internal(err.to_string()))?
                .join("worktrees")
                .join(id);
            Some(vec![
                root.display().to_string(),
                checkouts.display().to_string(),
            ])
        }
    };
    let roots: Vec<(String, String)> = store
        .projects()?
        .into_iter()
        .map(|row| (row.root_path, row.name))
        .collect();

    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|since| since.as_secs() as i64)
        .unwrap_or_default();
    let since = UNIX_EPOCH + Duration::from_secs((now - i64::from(days) * DAY).max(0) as u64);
    let mut files = 0;
    let mut read = Vec::new();
    for index in chosen {
        for path in transcripts(&found[index].directory, since) {
            files += 1;
            read.extend(
                records_of(&path)
                    .into_iter()
                    .filter(|record| {
                        scope.as_ref().is_none_or(|folders| {
                            folders.iter().any(|folder| within(&record.cwd, folder))
                        })
                    })
                    .map(|record| (index, record)),
            );
        }
    }
    let records = read.len() as u32;
    let project_of = |cwd: &str| {
        roots
            .iter()
            .filter(|(root, _)| within(cwd, root))
            .max_by_key(|(root, _)| root.len())
            .map(|(_, name)| name.clone())
            .unwrap_or_else(|| cwd.to_owned())
    };
    let mut history = summarize(read, installations, now, days, project_of, |session| {
        card_of_session(&store, session)
    });
    history.files = files;
    history.records = records;
    history.scan_ms = started.elapsed().as_secs_f64() * 1_000.0;
    Ok(history)
}

/// Whether `path` is `folder` or inside it — not a sibling that shares a prefix.
fn within(path: &str, folder: &str) -> bool {
    path == folder
        || path
            .strip_prefix(folder)
            .is_some_and(|rest| rest.starts_with('/'))
}

/// `spend.history` — what the agents spent over the last `days`, from their
/// transcripts, for one installation or all and one project or all.
#[tauri::command]
#[specta::specta]
pub async fn spend_history(
    project_id: Option<String>,
    installation: Option<String>,
    days: u32,
) -> Result<SpendHistory, RpcError> {
    if !(1..=366).contains(&days) {
        return Err(RpcError::new(
            ErrorCode::Invalid,
            "a range is 1 to 366 days",
        ));
    }
    if project_id
        .as_deref()
        .is_some_and(|id| !crate::adopting::plain(id))
    {
        return Err(RpcError::new(ErrorCode::Forbidden, "that is not a project"));
    }
    tauri::async_runtime::spawn_blocking(move || history(project_id, installation, days))
        .await
        .map_err(|err| RpcError::internal(err.to_string()))?
}

#[cfg(test)]
#[path = "spend_history_tests.rs"]
mod tests;
