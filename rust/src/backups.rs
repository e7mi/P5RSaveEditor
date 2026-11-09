//! Timestamped backups outside the game's save folder.

use std::fs;
use std::path::{Path, PathBuf};

use chrono::{Local, NaiveDateTime};
use serde_json::json;
use sha1_smol::Sha1;
use sha2::{Digest, Sha256};

use crate::paths::{clean_path, local_app_dir};

#[derive(Clone, Debug)]
pub struct BackupRecord {
    pub key: String,
    pub timestamp: String,
    pub slot: String,
    pub path: PathBuf,
    pub size: u64,
    pub reason: String,
    pub source: String,
}

impl BackupRecord {
    pub fn display_time(&self) -> String {
        let prefix = self.key.get(0..15).unwrap_or(&self.key);
        if let Ok(time) = NaiveDateTime::parse_from_str(prefix, "%Y%m%d_%H%M%S") {
            time.format("%Y/%m/%d %H:%M:%S").to_string()
        } else {
            self.timestamp.clone()
        }
    }
}

pub struct BackupManager {
    pub root: PathBuf,
}

impl BackupManager {
    pub fn new(save_dir: &Path) -> Self {
        let resolved = fs::canonicalize(save_dir)
            .map(|path| clean_path(&path))
            .unwrap_or_else(|_| clean_path(save_dir));
        let digest = Sha1::from(resolved.to_string_lossy().as_bytes())
            .digest()
            .to_string();
        let root = local_app_dir().join("backups").join(&digest[..12]);
        let _ = fs::create_dir_all(&root);
        Self { root }
    }

    pub fn create(
        &self,
        source_path: &Path,
        slot_name: &str,
        reason: &str,
    ) -> std::io::Result<BackupRecord> {
        let now = Local::now();
        let key = format!(
            "{}_{:03}",
            now.format("%Y%m%d_%H%M%S"),
            now.timestamp_subsec_millis()
        );
        let destination = self.root.join(&key);
        fs::create_dir_all(&destination)?;
        let target = destination.join(slot_name).join("DATA.DAT");
        if let Some(parent) = target.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::copy(source_path, &target)?;
        let size = fs::metadata(&target)?.len();
        let sha256 = sha256(&target)?;
        let meta = json!({
            "timestamp": &key,
            "slot": slot_name,
            "reason": reason,
            "source": source_path.to_string_lossy(),
            "size": size,
            "sha256": sha256,
        });
        let meta_text = serde_json::to_string_pretty(&meta).map_err(std::io::Error::other)?;
        fs::write(destination.join("meta.json"), meta_text)?;
        Ok(BackupRecord {
            key: key.clone(),
            timestamp: key,
            slot: slot_name.to_string(),
            path: target,
            size,
            reason: reason.to_string(),
            source: source_path.to_string_lossy().to_string(),
        })
    }

    pub fn list(&self) -> Vec<BackupRecord> {
        let mut records = Vec::new();
        let Ok(entries) = fs::read_dir(&self.root) else {
            return records;
        };
        for entry in entries.flatten() {
            let folder = entry.path();
            if !folder.is_dir() {
                continue;
            }
            let meta_path = folder.join("meta.json");
            if meta_path.is_file() {
                let Ok(text) = fs::read_to_string(&meta_path) else {
                    continue;
                };
                let Ok(meta) = serde_json::from_str::<serde_json::Value>(&text) else {
                    continue;
                };
                let slot = meta
                    .get("slot")
                    .and_then(|value| value.as_str())
                    .unwrap_or("")
                    .to_string();
                if slot.is_empty() {
                    continue;
                }
                let path = folder.join(&slot).join("DATA.DAT");
                if !path.is_file() {
                    continue;
                }
                let size = meta
                    .get("size")
                    .and_then(|value| value.as_u64())
                    .or_else(|| fs::metadata(&path).ok().map(|meta| meta.len()))
                    .unwrap_or(0);
                records.push(BackupRecord {
                    key: folder
                        .file_name()
                        .map(|name| name.to_string_lossy().to_string())
                        .unwrap_or_default(),
                    timestamp: meta
                        .get("timestamp")
                        .and_then(|value| value.as_str())
                        .unwrap_or_default()
                        .to_string(),
                    slot,
                    path,
                    size,
                    reason: meta
                        .get("reason")
                        .and_then(|value| value.as_str())
                        .unwrap_or_default()
                        .to_string(),
                    source: meta
                        .get("source")
                        .and_then(|value| value.as_str())
                        .unwrap_or_default()
                        .to_string(),
                });
            } else {
                let Ok(children) = fs::read_dir(&folder) else {
                    continue;
                };
                for child in children.flatten() {
                    let data = child.path().join("DATA.DAT");
                    if data.is_file() {
                        records.push(BackupRecord {
                            key: format!(
                                "{}/{}",
                                folder
                                    .file_name()
                                    .map(|name| name.to_string_lossy().to_string())
                                    .unwrap_or_default(),
                                child.file_name().to_string_lossy()
                            ),
                            timestamp: folder
                                .file_name()
                                .map(|name| name.to_string_lossy().to_string())
                                .unwrap_or_default(),
                            slot: child.file_name().to_string_lossy().to_string(),
                            size: fs::metadata(&data).map(|meta| meta.len()).unwrap_or(0),
                            path: data,
                            reason: "自動バックアップ".into(),
                            source: String::new(),
                        });
                    }
                }
            }
        }
        records.sort_by(|a, b| b.timestamp.cmp(&a.timestamp));
        records
    }

    pub fn restore(
        &self,
        record: &BackupRecord,
        target: &Path,
    ) -> std::io::Result<Option<BackupRecord>> {
        let safety = if target.is_file() {
            Some(self.create(target, &record.slot, "復元前の自動バックアップ")?)
        } else {
            None
        };
        if let Some(parent) = target.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::copy(&record.path, target)?;
        Ok(safety)
    }
}

fn sha256(path: &Path) -> std::io::Result<String> {
    let bytes = fs::read(path)?;
    Ok(format!("{:x}", Sha256::digest(bytes)))
}
