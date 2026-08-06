use anyhow::{bail, Context, Result};
use serde::Deserialize;
use std::{env, fs, net::SocketAddr, path::PathBuf};

#[derive(Clone, Debug, Deserialize)]
#[serde(default)]
pub struct Config {
    pub listen: SocketAddr,
    pub static_dir: PathBuf,
    pub kvmd: KvmdConfig,
    pub live777: Live777Config,
    pub video: VideoConfig,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(default)]
pub struct KvmdConfig {
    pub base_url: String,
    pub username: String,
    pub password: String,
    pub accept_invalid_certs: bool,
    pub request_timeout_ms: u64,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(default)]
pub struct Live777Config {
    pub health_url: String,
    pub base_url: String,
    pub whep_url: String,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(default)]
pub struct VideoConfig {
    pub width: u32,
    pub height: u32,
    pub fps: u32,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            listen: "0.0.0.0:9080".parse().unwrap(),
            static_dir: PathBuf::from("web/dist"),
            kvmd: KvmdConfig::default(),
            live777: Live777Config::default(),
            video: VideoConfig::default(),
        }
    }
}

impl Default for KvmdConfig {
    fn default() -> Self {
        Self {
            base_url: "https://127.0.0.1".into(),
            username: "ipkvm-gateway".into(),
            password: String::new(),
            accept_invalid_certs: true,
            request_timeout_ms: 1500,
        }
    }
}

impl Default for Live777Config {
    fn default() -> Self {
        Self {
            health_url: "http://127.0.0.1:7777/".into(),
            base_url: "http://127.0.0.1:7777".into(),
            whep_url: "/ipkvm/media/whep/ipkvm".into(),
        }
    }
}

impl Default for VideoConfig {
    fn default() -> Self {
        Self { width: 1280, height: 720, fps: 30 }
    }
}

impl Config {
    pub fn load() -> Result<Self> {
        let path = env::var("IPKVM_CONFIG").unwrap_or_else(|_| "config/ipkvm.toml".into());
        let mut cfg = if std::path::Path::new(&path).exists() {
            toml::from_str(&fs::read_to_string(&path).with_context(|| format!("read {path}"))?)
                .with_context(|| format!("parse {path}"))?
        } else {
            Self::default()
        };
        if let Ok(value) = env::var("IPKVM_KVMD_PASSWORD") {
            cfg.kvmd.password = value;
        }
        if cfg.kvmd.password.is_empty() || cfg.kvmd.password == "change-me" {
            bail!("set a real KVMD password with IPKVM_KVMD_PASSWORD");
        }
        Ok(cfg)
    }
}
