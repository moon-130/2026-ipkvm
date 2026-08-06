use crate::{config::KvmdConfig, model::*};
use anyhow::{bail, Context, Result};
use reqwest::Client;
use serde::Serialize;
use std::{collections::HashSet, sync::Arc, time::Duration};
use tokio::sync::Mutex;

#[derive(Clone)]
pub struct KvmdClient {
    cfg: KvmdConfig,
    http: Client,
    pressed_keys: Arc<Mutex<HashSet<String>>>,
    pressed_buttons: Arc<Mutex<HashSet<String>>>,
}

impl KvmdClient {
    pub fn new(cfg: KvmdConfig) -> Result<Self> {
        let http = Client::builder()
            .danger_accept_invalid_certs(cfg.accept_invalid_certs)
            .timeout(Duration::from_millis(cfg.request_timeout_ms))
            .build()?;
        Ok(Self {
            cfg,
            http,
            pressed_keys: Default::default(),
            pressed_buttons: Default::default(),
        })
    }

    fn endpoint(&self, path: &str) -> String {
        format!("{}{}", self.cfg.base_url.trim_end_matches('/'), path)
    }

    async fn post<T: Serialize + ?Sized>(&self, path: &str, query: &T) -> Result<()> {
        let response = self.http.post(self.endpoint(path))
            .header("X-KVMD-User", &self.cfg.username)
            .header("X-KVMD-Passwd", &self.cfg.password)
            .query(query).send().await.context("KVMD request failed")?;
        if !response.status().is_success() {
            bail!("KVMD returned {} for {}", response.status(), path);
        }
        Ok(())
    }

    pub async fn health(&self) -> bool {
        self.http.get(self.endpoint("/api/hid"))
            .header("X-KVMD-User", &self.cfg.username)
            .header("X-KVMD-Passwd", &self.cfg.password)
            .send().await.map(|r| r.status().is_success()).unwrap_or(false)
    }

    pub async fn authorize_cookie(&self, cookie: &str) -> bool {
        if cookie.is_empty() { return false; }
        self.http.get(self.endpoint("/api/auth/check"))
            .header(reqwest::header::COOKIE, cookie)
            .send().await.map(|r| r.status().is_success()).unwrap_or(false)
    }

    pub async fn dispatch(&self, msg: &ControlMessage) -> Result<()> {
        match msg.kind {
            ControlKind::Key => {
                let p: KeyPayload = serde_json::from_value(msg.payload.clone())?;
                self.post("/api/hid/events/send_key", &[("key", p.code.as_str()), ("state", if p.pressed { "1" } else { "0" })]).await?;
                let mut keys = self.pressed_keys.lock().await;
                if p.pressed { keys.insert(p.code); } else { keys.remove(&p.code); }
            }
            ControlKind::MouseMoveAbs => {
                let p: AbsPayload = serde_json::from_value(msg.payload.clone())?;
                self.post("/api/hid/events/send_mouse_move", &[("to_x", p.x), ("to_y", p.y)]).await?;
            }
            ControlKind::MouseMoveRel => {
                let p: RelPayload = serde_json::from_value(msg.payload.clone())?;
                self.post("/api/hid/events/send_mouse_relative", &[("delta_x", p.dx), ("delta_y", p.dy)]).await?;
            }
            ControlKind::MouseButton => {
                let p: ButtonPayload = serde_json::from_value(msg.payload.clone())?;
                self.post("/api/hid/events/send_mouse_button", &[("button", p.button.as_str()), ("state", if p.pressed { "1" } else { "0" })]).await?;
                let mut buttons = self.pressed_buttons.lock().await;
                if p.pressed { buttons.insert(p.button); } else { buttons.remove(&p.button); }
            }
            ControlKind::Wheel => {
                let p: WheelPayload = serde_json::from_value(msg.payload.clone())?;
                self.post("/api/hid/events/send_mouse_wheel", &[("delta_x", 0_i16), ("delta_y", p.dy)]).await?;
            }
            ControlKind::ReleaseAll => self.release_all().await?,
            ControlKind::Ping => {}
        }
        Ok(())
    }

    pub async fn release_all(&self) -> Result<()> {
        let keys: Vec<_> = self.pressed_keys.lock().await.drain().collect();
        for key in keys {
            let _ = self.post("/api/hid/events/send_key", &[("key", key.as_str()), ("state", "0")]).await;
        }
        let buttons: Vec<_> = self.pressed_buttons.lock().await.drain().collect();
        for button in buttons {
            let _ = self.post("/api/hid/events/send_mouse_button", &[("button", button.as_str()), ("state", "0")]).await;
        }
        Ok(())
    }
}
