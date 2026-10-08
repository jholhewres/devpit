use super::{aws_profile_in, declared, missing_in, refused, Missing, Need};

#[test]
fn needs_txt_names_what_it_needs_and_nothing_else_runs() {
    let text = "# sign-ins\naws asc\ngcloud\ngh # for PRs\naws bad;rm -rf ~\nkubectl\n";
    assert_eq!(
        declared(text),
        [Need::Aws("asc".to_owned()), Need::Gcloud, Need::Gh]
    );
}

#[test]
fn an_env_file_names_the_aws_profile() {
    assert_eq!(
        aws_profile_in("A=1\nexport AWS_PROFILE=\"ascbot\"\n"),
        Some("ascbot".to_owned())
    );
    assert_eq!(aws_profile_in("AWS_PROFILE=$(whoami)\n"), None);
    assert_eq!(aws_profile_in("AWS_REGION=us-east-1\n"), None);
}

#[test]
fn an_expired_aws_profile_says_the_command_with_its_name() {
    let error = "Exit code 255\nError when retrieving token from sso: Token has expired and refresh failed\n$ aws s3 ls --profile asc";
    assert_eq!(
        missing_in("Bash", error),
        Some(Missing {
            what: "the AWS profile asc is signed out or expired".to_owned(),
            command: "aws sso login --profile asc".to_owned(),
        })
    );
    let unnamed = missing_in(
        "Bash",
        "An error occurred (ExpiredToken) when calling GetObject",
    )
    .expect("aws");
    assert_eq!(unnamed.command, "aws sso login");
}

#[test]
fn gcloud_gh_and_an_mcp_server_are_told_apart() {
    let gcloud = missing_in(
        "Bash",
        "ERROR: (gcloud.run.deploy) There was a problem refreshing your current auth tokens",
    )
    .expect("gcloud");
    assert_eq!(gcloud.command, "gcloud auth login");
    let gh = missing_in(
        "Bash",
        "To get started with GitHub CLI, please run:  gh auth login",
    )
    .expect("gh");
    assert_eq!(gh.command, "gh auth login");
    let mcp = missing_in("mcp__github__create_issue", "HTTP 401 Unauthorized").expect("mcp");
    assert_eq!(mcp.what, "the MCP server github needs signing in again");
}

#[test]
fn an_ordinary_failure_is_not_a_missing_sign_in() {
    assert_eq!(
        missing_in("Bash", "error[E0425]: cannot find value `x`"),
        None
    );
    // A 401 from a Bash curl is the server's answer, not a sign-in of ours.
    assert_eq!(
        missing_in("Bash", "curl: (22) The requested URL returned error: 401"),
        None
    );
}

#[test]
fn a_refused_start_says_each_fix() {
    let said = refused(&[Need::Aws("asc".to_owned()).missing(), Need::Gh.missing()]);
    assert!(said.starts_with("not started: the AWS profile asc"));
    assert!(said
        .contains("`aws sso login --profile asc`; the GitHub CLI is signed out — `gh auth login`"));
    assert!(said.contains("anyway: true"));
}
