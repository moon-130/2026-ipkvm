use crate::{config::Config, kvmd::KvmdClient};
use std::{collections::HashMap, sync::{atomic::{AtomicU64, AtomicUsize, Ordering}, Arc}};
use tokio::sync::Mutex;

#[derive(Clone)]
pub struct AppState {
    pub cfg: Config,
    pub kvmd: KvmdClient,
    pub controller: Arc<Mutex<Option<String>>>,
    pub sessions: Arc<Mutex<HashMap<String, u64>>>,
    pub connections: Arc<AtomicUsize>,
    pub forwarded: Arc<AtomicU64>,
    pub rejected: Arc<AtomicU64>,
    pub last_error: Arc<Mutex<Option<String>>>,
}

impl AppState {
    pub fn new(cfg: Config, kvmd: KvmdClient) -> Self {
        Self {
            cfg, kvmd,
            controller: Default::default(),
            sessions: Default::default(),
            connections: Default::default(),
            forwarded: Default::default(),
            rejected: Default::default(),
            last_error: Default::default(),
        }
    }

    pub fn connected(&self) { self.connections.fetch_add(1, Ordering::Relaxed); }
    pub fn disconnected(&self) { self.connections.fetch_sub(1, Ordering::Relaxed); }
}

