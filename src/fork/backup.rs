//! Export / import of peers, settings and server profiles, without any password.
use hbb_common::{
    config::{self, Config, LocalConfig, PeerConfig},
    ResultType,
};
use serde_derive::{Deserialize, Serialize};
use std::collections::HashMap;

const FORMAT: &str = "ruskdesk-backup";

#[derive(Serialize, Deserialize)]
struct Backup {
    format: String,
    version: u32,
    options: HashMap<String, String>,
    local: LocalBackup,
    peers: Vec<PeerBackup>,
}

/// The parts of `<app>_local.toml` worth carrying over; server profiles live in `options`.
#[derive(Debug, Default, Serialize, Deserialize)]
struct LocalBackup {
    #[serde(default)]
    fav: Vec<String>,
    #[serde(default)]
    options: HashMap<String, String>,
    #[serde(default)]
    ui_flutter: HashMap<String, String>,
}

#[derive(Serialize, Deserialize)]
struct PeerBackup {
    id: String,
    /// The peer's file as stored on disk (TOML), so its custom deserializers apply on import.
    config: String,
}

fn is_secret(key: &str) -> bool {
    key.contains("password")
}

fn without_secrets(map: HashMap<String, String>) -> HashMap<String, String> {
    map.into_iter().filter(|(k, _)| !is_secret(k)).collect()
}

fn local_file() -> std::path::PathBuf {
    Config::path(format!("{}_local.toml", crate::get_app_name()))
}

pub fn export() -> ResultType<String> {
    let options: HashMap<String, String> =
        serde_json::from_str(&crate::ui_interface::get_options())?;
    let mut local: LocalBackup = config::load_path(local_file());
    local.options = without_secrets(local.options);
    let peers = PeerConfig::peers(None)
        .into_iter()
        .map(|(id, _, mut c)| -> ResultType<PeerBackup> {
            c.password = Vec::new();
            c.trusted_device_v2 = String::new();
            c.options = without_secrets(c.options);
            Ok(PeerBackup {
                id,
                config: hbb_common::toml::to_string(&c)?,
            })
        })
        .collect::<ResultType<_>>()?;
    let backup = Backup {
        format: FORMAT.to_owned(),
        version: 1,
        options: without_secrets(options),
        local,
        peers,
    };
    Ok(serde_json::to_string_pretty(&backup)?)
}

/// Merges a backup into the current settings; saved peer passwords on this device are kept.
pub fn import(json: &str) -> ResultType<()> {
    let backup: Backup = serde_json::from_str(json)?;
    if backup.format != FORMAT {
        hbb_common::bail!("not a {FORMAT} file");
    }
    let mut options: HashMap<String, String> =
        serde_json::from_str(&crate::ui_interface::get_options())?;
    options.extend(without_secrets(backup.options));
    crate::ui_interface::set_options(options);

    for (k, v) in without_secrets(backup.local.options) {
        LocalConfig::set_option(k, v);
    }
    for (k, v) in backup.local.ui_flutter {
        LocalConfig::set_flutter_option(k, v);
    }
    let mut fav = LocalConfig::get_fav();
    for id in backup.local.fav {
        if !fav.contains(&id) {
            fav.push(id);
        }
    }
    LocalConfig::set_fav(fav);

    for p in backup.peers {
        let mut c: PeerConfig = hbb_common::toml::from_str(&p.config)?;
        let old = PeerConfig::load(&p.id);
        c.password = old.password;
        c.trusted_device_v2 = old.trusted_device_v2;
        c.options = without_secrets(c.options);
        c.options.extend(old.options.into_iter().filter(|(k, _)| is_secret(k)));
        c.store(&p.id);
    }
    Ok(())
}
