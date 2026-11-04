use std::io::{BufRead, BufReader, Write};
use std::os::unix::fs::PermissionsExt;

use super::*;

#[test]
fn each_of_the_four_release_targets_has_its_archive() {
    assert_eq!(platform("linux", "x86_64").as_deref(), Some("linux_amd64"));
    assert_eq!(platform("linux", "aarch64").as_deref(), Some("linux_arm64"));
    assert_eq!(platform("macos", "x86_64").as_deref(), Some("darwin_amd64"));
    assert_eq!(
        platform("macos", "aarch64").as_deref(),
        Some("darwin_arm64")
    );
    assert_eq!(platform("windows", "x86_64"), None);
    assert_eq!(
        archive("0.2.1", "linux_amd64"),
        "devflow_0.2.1_linux_amd64.tar.gz"
    );
}

/// The v0.2.1 release's own checksums.txt.
const SUMS: &str = "\
21726ba703e90a260f6ec972863c7f1fbb384a81b78ee767db1a778d897c7a51  devflow_0.2.1_darwin_amd64.tar.gz
f47a1e9036f7e895a3e92ee3631a1e52c1a317710873a329709605b32cb3cd16  devflow_0.2.1_darwin_arm64.tar.gz
e9c602b93d8ce49875cb21c066dd8da643a452118aaf12a3f19fb99ad7679029  devflow_0.2.1_linux_amd64.tar.gz
b50f19abcc47c803f05f9f08169be3f458c9b8bbcd4a9b6e55119917ff775e3a *devflow_0.2.1_linux_arm64.tar.gz
";

#[test]
fn the_checksum_is_the_one_listed_for_that_archive() {
    assert_eq!(
        sum_for(SUMS, "devflow_0.2.1_linux_amd64.tar.gz").as_deref(),
        Some("e9c602b93d8ce49875cb21c066dd8da643a452118aaf12a3f19fb99ad7679029")
    );
    assert_eq!(
        sum_for(SUMS, "devflow_0.2.1_linux_arm64.tar.gz").as_deref(),
        Some("b50f19abcc47c803f05f9f08169be3f458c9b8bbcd4a9b6e55119917ff775e3a")
    );
    assert_eq!(sum_for(SUMS, "devflow_0.2.1_linux.tar.gz"), None);
}

/// A release folder served over HTTP on this machine, one connection at a time.
fn serve(files: Vec<(String, Vec<u8>)>) -> String {
    let listener = std::net::TcpListener::bind("127.0.0.1:0").expect("bind");
    let at = listener.local_addr().expect("addr");
    std::thread::spawn(move || {
        for stream in listener.incoming() {
            let Ok(mut stream) = stream else { return };
            let mut first = String::new();
            let mut reader = BufReader::new(stream.try_clone().expect("clone"));
            reader.read_line(&mut first).expect("line");
            loop {
                let mut header = String::new();
                if reader.read_line(&mut header).unwrap_or(0) <= 2 {
                    break;
                }
            }
            let path = first.split_whitespace().nth(1).unwrap_or("");
            let found = files.iter().find(|(name, _)| path.ends_with(name.as_str()));
            let (status, body) = match found {
                Some((_, body)) => ("200 OK", body.clone()),
                None => ("404 Not Found", Vec::new()),
            };
            let head = format!(
                "HTTP/1.1 {status}\r\ncontent-length: {}\r\nconnection: close\r\n\r\n",
                body.len()
            );
            let _ = stream.write_all(head.as_bytes());
            let _ = stream.write_all(&body);
        }
    });
    format!("http://{at}")
}

/// A release archive holding a `devflow` that answers as `version`.
fn release(version: &str) -> Vec<u8> {
    let dir = tempfile::tempdir().expect("dir");
    let bin = dir.path().join("devflow");
    std::fs::write(
        &bin,
        format!(
            "#!/bin/sh\ncase \"$1\" in\n--version) echo 'devflow {version}' ;;\n\
             doctor) echo 'ok  devflow is on PATH'; echo 'FIX claude: devflow is not registered'; exit 1 ;;\n\
             esac\n"
        ),
    )
    .expect("bin");
    std::fs::set_permissions(&bin, std::fs::Permissions::from_mode(0o755)).expect("chmod");
    std::fs::write(dir.path().join("README.md"), "devflow").expect("readme");
    let out = dir.path().join("a.tar.gz");
    let made = std::process::Command::new("tar")
        .arg("czf")
        .arg(&out)
        .arg("-C")
        .arg(dir.path())
        .args(["devflow", "README.md"])
        .status()
        .expect("tar");
    assert!(made.success());
    std::fs::read(out).expect("archive")
}

fn sums(name: &str, of: &[u8]) -> Vec<u8> {
    format!("{}  {name}\n", sha256_hex(of)).into_bytes()
}

#[tokio::test]
async fn a_verified_archive_lands_in_the_bin_folder_and_answers_its_version() {
    let name = archive("0.2.1", "linux_amd64");
    let body = release("0.2.1");
    let base = serve(vec![
        (name.clone(), body.clone()),
        ("checksums.txt".into(), sums(&name, &body)),
    ]);
    let home = tempfile::tempdir().expect("home");
    let bin = home.path().join(".local/bin");
    let scratch = home.path().join("scratch");
    let installed = install_from(&base, "0.2.1", "linux_amd64", &bin, &scratch)
        .await
        .expect("installed");
    assert_eq!(installed, bin.join("devflow"));
    let state = state_of(&installed, true);
    assert_eq!(state.version.as_deref(), Some("0.2.1"));
    assert_eq!(
        state.doctor,
        [
            "ok  devflow is on PATH",
            "FIX claude: devflow is not registered"
        ]
    );
    assert!(!scratch.exists(), "the download is cleaned up");
}

#[tokio::test]
async fn an_archive_that_does_not_match_its_checksum_is_refused_and_nothing_is_installed() {
    let name = archive("0.2.1", "linux_amd64");
    let body = release("0.2.1");
    let base = serve(vec![
        (name.clone(), body.clone()),
        ("checksums.txt".into(), sums(&name, b"another archive")),
    ]);
    let home = tempfile::tempdir().expect("home");
    let bin = home.path().join(".local/bin");
    std::fs::create_dir_all(&bin).expect("bin");
    std::fs::write(bin.join("devflow"), "the one before").expect("before");
    let scratch = home.path().join("scratch");
    let err = install_from(&base, "0.2.1", "linux_amd64", &bin, &scratch)
        .await
        .expect_err("refused");
    assert!(
        err.message.contains("does not match its checksum"),
        "{}",
        err.message
    );
    assert_eq!(
        std::fs::read_to_string(bin.join("devflow")).expect("kept"),
        "the one before"
    );
    assert!(!scratch.exists());
}

#[tokio::test]
async fn with_no_network_the_install_fails_cleanly() {
    // Nothing listens on port 1: what an offline machine answers, at once.
    let home = tempfile::tempdir().expect("home");
    let bin = home.path().join(".local/bin");
    let scratch = home.path().join("scratch");
    let err = install_from("http://127.0.0.1:1", "0.2.1", "linux_amd64", &bin, &scratch)
        .await
        .expect_err("offline");
    assert!(
        err.message.starts_with("devflow could not be downloaded"),
        "{}",
        err.message
    );
    assert!(!bin.join("devflow").exists());
    assert!(!scratch.exists());
}

#[test]
fn a_binary_that_is_not_there_has_no_version() {
    let home = tempfile::tempdir().expect("home");
    let state = state_of(&home.path().join(".local/bin/devflow"), false);
    assert_eq!(state.version, None);
    assert!(state.doctor.is_empty());
    assert_eq!(state.known, crate::devflow_plugin::KNOWN);
}

/// The real v0.2.1 release, into a throwaway folder under `DEVPIT_PROBE_DIR`:
/// needs the network, so `make test` never runs it.
#[tokio::test]
#[ignore = "needs the network"]
async fn the_real_release_installs_and_doctor_runs() {
    let base = std::env::var_os("DEVPIT_PROBE_DIR").expect("DEVPIT_PROBE_DIR");
    let probe = tempfile::tempdir_in(base).expect("probe");
    let here = platform(std::env::consts::OS, std::env::consts::ARCH).expect("platform");
    let bin = probe.path().join("bin");
    let installed = install_from(RELEASES, KNOWN, &here, &bin, &probe.path().join("scratch"))
        .await
        .expect("installed");
    let state = state_of(&installed, true);
    assert_eq!(state.version.as_deref(), Some(KNOWN));
    assert!(!state.doctor.is_empty());
}
