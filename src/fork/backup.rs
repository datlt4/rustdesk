//! Export / import of peers (with saved passwords), settings and server profiles.
//! The device's own permanent password is not included: only a machine-encrypted hash of it exists.
use hbb_common::{
    config::{self, Config, LocalConfig, PeerConfig},
    ResultType,
};
use serde_derive::{Deserialize, Serialize};
use std::collections::HashMap;

const FORMAT: &str = "rustdesk-backup";

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

fn local_file() -> std::path::PathBuf {
    Config::path(format!("{}_local.toml", crate::get_app_name()))
}

pub fn export() -> ResultType<String> {
    let options: HashMap<String, String> =
        serde_json::from_str(&crate::ui_interface::get_options())?;
    let local: LocalBackup = config::load_path(local_file());
    let peers = PeerConfig::peers(None)
        .into_iter()
        .map(|(id, _, mut c)| -> ResultType<PeerBackup> {
            c.trusted_device_v2 = String::new();
            Ok(PeerBackup {
                id,
                config: hbb_common::toml::to_string(&c)?,
            })
        })
        .collect::<ResultType<_>>()?;
    let backup = Backup {
        format: FORMAT.to_owned(),
        version: 1,
        options,
        local,
        peers,
    };
    Ok(serde_json::to_string_pretty(&backup)?)
}

/// Merges a backup into the current settings; peers that already exist here are left as they are.
pub fn import(json: &str) -> ResultType<()> {
    let backup: Backup = serde_json::from_str(json)?;
    if backup.format != FORMAT {
        hbb_common::bail!("not a {FORMAT} file");
    }
    let mut options: HashMap<String, String> =
        serde_json::from_str(&crate::ui_interface::get_options())?;
    options.extend(backup.options);
    crate::ui_interface::set_options(options);

    for (k, v) in backup.local.options {
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
        if PeerConfig::exists(&p.id) {
            continue;
        }
        let c: PeerConfig = hbb_common::toml::from_str(&p.config)?;
        c.store(&p.id);
    }
    Ok(())
}
