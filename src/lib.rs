//! SigmaOS app-env — application runtime environment manager.
//!
//! Defines sandboxed app environment profiles: env vars, filesystem mounts,
//! network access, and resource hints. Generates launch commands/env.

#![forbid(unsafe_code)]

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

/// Errors produced by this component.
#[derive(Debug, thiserror::Error)]
pub enum AppEnvError {
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    #[error("json error: {0}")]
    Json(#[from] serde_json::Error),
    #[error("profile not found: {0}")]
    NotFound(String),
    #[error("validation error: {0}")]
    Validation(String),
}

/// A filesystem mount inside the sandbox.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Mount {
    /// host path
    pub source: PathBuf,
    /// path inside sandbox
    pub target: PathBuf,
    /// "ro" | "rw"
    #[serde(default = "default_rw")]
    pub mode: String,
}

fn default_rw() -> String {
    "rw".into()
}

/// An app environment profile.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppProfile {
    pub id: String,
    pub name: String,
    /// Executable to run.
    pub command: Vec<String>,
    /// Environment variables (KEY=VALUE).
    #[serde(default)]
    pub env: BTreeMap<String, String>,
    /// Filesystem mounts.
    #[serde(default)]
    pub mounts: Vec<Mount>,
    /// Allow network access.
    #[serde(default)]
    pub network: bool,
    /// Working directory inside sandbox.
    #[serde(default)]
    pub workdir: Option<PathBuf>,
    /// Description.
    #[serde(default)]
    pub description: String,
}

/// Profile store.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ProfileStore {
    #[serde(default)]
    pub profiles: BTreeMap<String, AppProfile>,
}

impl ProfileStore {
    /// Register a profile.
    pub fn add(&mut self, p: AppProfile) -> Result<(), AppEnvError> {
        if p.id.is_empty() {
            return Err(AppEnvError::Validation("empty profile id".into()));
        }
        if p.command.is_empty() {
            return Err(AppEnvError::Validation(format!(
                "profile '{}' has no command",
                p.id
            )));
        }
        self.profiles.insert(p.id.clone(), p);
        Ok(())
    }

    /// Get a profile.
    pub fn get(&self, id: &str) -> Option<&AppProfile> {
        self.profiles.get(id)
    }

    /// Remove a profile.
    pub fn remove(&mut self, id: &str) -> Result<(), AppEnvError> {
        if self.profiles.remove(id).is_none() {
            return Err(AppEnvError::NotFound(id.into()));
        }
        Ok(())
    }

    /// List profiles.
    pub fn list(&self) -> Vec<&AppProfile> {
        self.profiles.values().collect()
    }

    /// Render a profile as a launch spec: env vars + command.
    pub fn render_launch(&self, id: &str) -> Result<LaunchSpec, AppEnvError> {
        let p = self
            .profiles
            .get(id)
            .ok_or_else(|| AppEnvError::NotFound(id.into()))?;
        Ok(LaunchSpec {
            env: p.env.clone(),
            command: p.command.clone(),
            mounts: p.mounts.clone(),
            network: p.network,
            workdir: p.workdir.clone(),
        })
    }

    /// Save to JSON.
    pub fn save(&self, path: &Path) -> Result<(), AppEnvError> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let tmp = path.with_extension("json.tmp");
        std::fs::write(&tmp, serde_json::to_string_pretty(self)?)?;
        std::fs::rename(&tmp, path)?;
        Ok(())
    }

    /// Load from JSON.
    pub fn load(path: &Path) -> Result<Self, AppEnvError> {
        let text = std::fs::read_to_string(path)
            .map_err(|_| AppEnvError::NotFound(path.display().to_string()))?;
        Ok(serde_json::from_str(&text)?)
    }
}

/// A resolved launch specification.
#[derive(Debug, Clone, Serialize)]
pub struct LaunchSpec {
    pub env: BTreeMap<String, String>,
    pub command: Vec<String>,
    pub mounts: Vec<Mount>,
    pub network: bool,
    pub workdir: Option<PathBuf>,
}

impl LaunchSpec {
    /// Render as `env KEY=VAL ... cmd args` shell string.
    pub fn to_shell(&self) -> String {
        let env_str = self
            .env
            .iter()
            .map(|(k, v)| format!("{k}={}", shell_quote(v)))
            .collect::<Vec<_>>()
            .join(" ");
        let cmd = self
            .command
            .iter()
            .map(|c| shell_quote(c))
            .collect::<Vec<_>>()
            .join(" ");
        if env_str.is_empty() {
            cmd
        } else {
            format!("env {env_str} {cmd}")
        }
    }
}

fn shell_quote(s: &str) -> String {
    if s.chars()
        .all(|c| c.is_ascii_alphanumeric() || "_./-".contains(c))
    {
        s.to_string()
    } else {
        format!("'{}'", s.replace('\'', "'\\''"))
    }
}

/// Default profile store path.
pub fn default_store_path() -> Option<PathBuf> {
    directories::ProjectDirs::from("org", "sigmaos", "sigmaos")
        .map(|p| p.config_dir().join("app-env").join("profiles.json"))
}

/// Built-in profiles: browser, terminal, files.
pub fn builtin_profiles() -> ProfileStore {
    let mut store = ProfileStore::default();
    let mut env = BTreeMap::new();
    env.insert("SIGMAOS_SESSION".into(), "1".into());
    env.insert("XDG_CONFIG_HOME".into(), "~/.config".into());

    let _ = store.add(AppProfile {
        id: "browser".into(),
        name: "Web Browser".into(),
        command: vec!["firefox".into()],
        env: env.clone(),
        mounts: Vec::new(),
        network: true,
        workdir: None,
        description: "Sandboxed browser profile".into(),
    });
    let _ = store.add(AppProfile {
        id: "terminal".into(),
        name: "Terminal".into(),
        command: vec!["foot".into()],
        env,
        mounts: vec![Mount {
            source: PathBuf::from("/home"),
            target: PathBuf::from("/home"),
            mode: "rw".into(),
        }],
        network: true,
        workdir: Some(PathBuf::from("~")),
        description: "Terminal with home access".into(),
    });
    store
}
