//! Fork-only settings: updates come from datlt4/rustdesk releases, upstream releases are announced.
use hbb_common::{anyhow::anyhow, bytes::Bytes, log, ResultType};
use serde_json::Value;

pub mod backup;

pub const OWNER: &str = "datlt4";
pub const LATEST_RELEASE_API: &str = "https://api.github.com/repos/datlt4/rustdesk/releases/latest";
const UPSTREAM_LATEST_RELEASE_API: &str =
    "https://api.github.com/repos/rustdesk/rustdesk/releases/latest";
// GitHub's API rejects requests without one.
pub const USER_AGENT: &str = "RustDesk";

/// Upstream version the code is synced to; set by res/fork/set-version.sh.
/// Peers and the rendezvous server get this instead of the fork version, since
/// get_version_number reads 1.5.101 as 1.6.10 and peers would expect newer features.
pub const UPSTREAM_VERSION: &str = "1.5.1";

/// `mainGetCommon` keys handled by the fork; errors come back as `error:<message>`.
pub fn get_common(key: &str) -> Option<String> {
    let res = if key == "fork-export-backup" {
        backup::export()
    } else if let Some(json) = key.strip_prefix("fork-import-backup:") {
        backup::import(json).map(|_| String::new())
    } else if key == "fork-check-update" {
        check_update_now()
    } else {
        return None;
    };
    Some(res.unwrap_or_else(|e| format!("error:{e}")))
}

/// Turns GitHub's latest-release JSON into the `{"url": <release page>}` the update check expects.
pub fn to_version_check_response(bytes: Bytes) -> ResultType<Bytes> {
    let release: Value = serde_json::from_slice(&bytes)?;
    let url = release["html_url"]
        .as_str()
        .ok_or_else(|| anyhow!("GitHub release has no html_url"))?;
    Ok(Bytes::from(serde_json::json!({ "url": url }).to_string()))
}

/// Manual "Check for updates": runs the startup check regardless of its option and returns the
/// release page of a newer fork version, or "" when this one is the latest.
fn check_update_now() -> ResultType<String> {
    // do_check_software_update builds its own runtime, so keep it off the caller's thread.
    std::thread::spawn(crate::common::do_check_software_update)
        .join()
        .map_err(|_| anyhow!("update check panicked"))??;
    check_upstream_release();
    Ok(crate::common::SOFTWARE_UPDATE_URL.lock().unwrap().clone())
}

pub fn check_upstream_release() {
    std::thread::spawn(|| {
        if let Err(e) = notify_upstream_release() {
            log::debug!("Upstream release check failed: {e}");
        }
    });
}

fn notify_upstream_release() -> ResultType<()> {
    let client = crate::hbbs_http::create_http_client_with_url_strict(UPSTREAM_LATEST_RELEASE_API)?;
    let resp = client
        .get(UPSTREAM_LATEST_RELEASE_API)
        .header("User-Agent", USER_AGENT)
        .send()?;
    if !resp.status().is_success() {
        return Err(anyhow!("HTTP {}", resp.status()));
    }
    let release: Value = serde_json::from_slice(&resp.bytes()?)?;
    let version = release["tag_name"]
        .as_str()
        .unwrap_or_default()
        .trim_start_matches('v');
    let published = release["published_at"].as_str().unwrap_or_default();
    if !is_upstream_newer(version, published) {
        return Ok(());
    }
    #[cfg(feature = "flutter")]
    {
        let url = release["html_url"].as_str().unwrap_or_default();
        let data = std::collections::HashMap::from([
            ("name", "check_upstream_update_finish"),
            ("version", version),
            ("url", url),
        ]);
        if let Ok(data) = serde_json::to_string(&data) {
            let _ = crate::flutter::push_global_event(crate::flutter::APP_TYPE_MAIN, data);
        }
    }
    Ok(())
}

/// A release of the synced version counts only if it came out after this build,
/// because the code is synced to upstream master before every fork release.
fn is_upstream_newer(version: &str, published_at: &str) -> bool {
    use hbb_common::get_version_number as n;
    let (v, base) = (n(version), n(UPSTREAM_VERSION));
    v > base || (v == base && published_at.get(..10) > crate::BUILD_DATE.get(..10))
}

/// The package this Linux install came from, as named in rustdesk-update.json.
#[cfg(target_os = "linux")]
pub fn linux_update_format() -> &'static str {
    use std::process::{Command, Stdio};
    let ok = |cmd: &str, args: &[&str]| {
        Command::new(cmd)
            .args(args)
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .map(|s| s.success())
            .unwrap_or(false)
    };
    if std::env::var_os("APPIMAGE").is_some() {
        "appimage"
    } else if std::env::var_os("FLATPAK_ID").is_some() {
        "unknown"
    } else if ok("dpkg", &["-s", "rustdesk"]) {
        "deb"
    } else if ok("rpm", &["-q", "rustdesk"]) {
        "rpm"
    } else {
        "unknown"
    }
}

/// Installs a downloaded package after checking it, then restarts into the new version.
#[cfg(target_os = "linux")]
pub fn update_to_verified_package(file: &str, expected_sha256: &str) -> ResultType<()> {
    use std::{os::unix::fs::PermissionsExt, process::Command};

    // Copy into a private directory first so the checked file is the one installed.
    let dir = std::env::temp_dir().join(format!("rustdesk-update-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir(&dir)?;
    std::fs::set_permissions(&dir, std::fs::Permissions::from_mode(0o700))?;
    let name = std::path::Path::new(file)
        .file_name()
        .ok_or_else(|| anyhow!("invalid update file: {file}"))?;
    let pkg = dir.join(name);
    std::fs::copy(file, &pkg)?;
    let mut f = std::fs::File::open(&pkg)?;
    crate::update_hash::verify_sha256_reader(&mut f, expected_sha256)
        .map_err(|e| anyhow!("update file failed verification: {e:?}"))?;
    drop(f);

    let format = linux_update_format();
    let status = match format {
        "deb" => Command::new("pkexec")
            .args(["apt-get", "install", "-y", "--allow-downgrades"])
            .arg(&pkg)
            .status()?,
        "rpm" => Command::new("pkexec")
            .args(["rpm", "-U", "--force"])
            .arg(&pkg)
            .status()?,
        "appimage" => {
            let target = std::env::var_os("APPIMAGE").ok_or_else(|| anyhow!("APPIMAGE not set"))?;
            let staged = std::path::PathBuf::from(&target).with_extension("new");
            std::fs::copy(&pkg, &staged)?;
            std::fs::set_permissions(&staged, std::fs::Permissions::from_mode(0o755))?;
            std::fs::rename(&staged, &target)?;
            let _ = std::fs::remove_dir_all(&dir);
            return relaunch(&target.to_string_lossy());
        }
        _ => return Err(anyhow!("automatic update is not supported for this install")),
    };
    let _ = std::fs::remove_dir_all(&dir);
    if !status.success() {
        return Err(anyhow!("package install failed: {status}"));
    }
    relaunch("rustdesk")
}

/// Starts the new version once this process is gone, so its single-instance check passes.
#[cfg(target_os = "linux")]
fn relaunch(exe: &str) -> ResultType<()> {
    std::process::Command::new("sh")
        .args(["-c", "sleep 2; exec \"$0\"", exe])
        .spawn()?;
    std::process::exit(0);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn version_check_response_uses_release_page() {
        let gh = br#"{"url":"https://api.github.com/repos/datlt4/rustdesk/releases/1","html_url":"https://github.com/datlt4/rustdesk/releases/tag/1.5.102","tag_name":"1.5.102"}"#;
        let out = to_version_check_response(Bytes::from_static(gh)).unwrap();
        let v: Value = serde_json::from_slice(&out).unwrap();
        assert_eq!(v["url"], "https://github.com/datlt4/rustdesk/releases/tag/1.5.102");
    }

    #[test]
    fn upstream_newer_by_version_or_date() {
        assert!(is_upstream_newer("9.0.0", "2000-01-01T00:00:00Z"));
        assert!(!is_upstream_newer("1.0.0", "2999-01-01T00:00:00Z"));
        assert!(is_upstream_newer(UPSTREAM_VERSION, "2999-01-01T00:00:00Z"));
        assert!(!is_upstream_newer(UPSTREAM_VERSION, "2000-01-01T00:00:00Z"));
    }
}
