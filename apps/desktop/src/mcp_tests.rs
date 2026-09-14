use super::*;

fn server(name: &str, scope: &str) -> Server {
    Server {
        name: name.to_owned(),
        scope: scope.to_owned(),
        reached_by: format!("{scope}-command"),
    }
}

/// The project's file is read first, so the project's spelling of a shared
/// name is the one that survives — which is the order the CLI resolves in.
#[test]
fn the_first_file_to_name_a_server_keeps_it() {
    let mut servers = Vec::new();
    let mut sources = Vec::new();
    take(
        vec![server("reports", "project")],
        Path::new("/p/.mcp.json"),
        &mut servers,
        &mut sources,
    );
    take(
        vec![server("reports", "user"), server("other", "user")],
        Path::new("/home/me/.claude.json"),
        &mut servers,
        &mut sources,
    );

    assert_eq!(servers.len(), 2);
    assert_eq!(servers[0].scope, "project");
    assert_eq!(servers[0].reached_by, "project-command");
    assert_eq!(servers[1].name, "other");
}

/// A file naming nothing is not a source. Listing it would tell the reader
/// their servers came from a file that has none in it.
#[test]
fn a_file_that_named_nothing_is_not_listed_as_a_source() {
    let mut servers = Vec::new();
    let mut sources = Vec::new();
    take(
        Vec::new(),
        Path::new("/p/.mcp.json"),
        &mut servers,
        &mut sources,
    );
    assert!(sources.is_empty());
    assert!(servers.is_empty());
}

/// One file is read twice — once for this project's section and once for the
/// user-wide one — and naming it twice would read as two installations.
#[test]
fn one_file_read_for_two_scopes_is_named_once() {
    let mut servers = Vec::new();
    let mut sources = Vec::new();
    let settings = Path::new("/home/me/.claude.json");
    take(
        vec![server("reports", "project")],
        settings,
        &mut servers,
        &mut sources,
    );
    take(
        vec![server("everywhere", "user")],
        settings,
        &mut servers,
        &mut sources,
    );
    sources.dedup();
    assert_eq!(sources, vec!["/home/me/.claude.json".to_owned()]);
}
