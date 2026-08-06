use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Clone, Copy, Debug, Deserialize)]
pub struct ControlMessage {
    #[serde(rename = "type")]
    pub kind: ControlKind,
    pub seq: u64,
    #[serde(default)]
    pub payload: Value,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ControlKind {
    Key,
    MouseMoveAbs,
    MouseMoveRel,
    MouseButton,
    Wheel,
    Ping,
    ReleaseAll,
}

#[derive(Debug, Deserialize)]
pub struct KeyPayload { pub code: String, pub pressed: bool }

#[derive(Debug, Deserialize)]
pub struct AbsPayload { pub x: i16, pub y: i16 }

#[derive(Debug, Deserialize)]
pub struct RelPayload { pub dx: i16, pub dy: i16 }

#[derive(Debug, Deserialize)]
pub struct ButtonPayload { pub button: String, pub pressed: bool }

#[derive(Debug, Deserialize)]
pub struct WheelPayload { pub dy: i16 }

#[derive(Debug, Serialize)]
pub struct ServerMessage<'a> {
    #[serde(rename = "type")]
    pub kind: &'a str,
    pub seq: Option<u64>,
    pub ok: bool,
    pub message: &'a str,
}

#[derive(Debug, Serialize)]
pub struct StatusResponse {
    pub kvmd: bool,
    pub hid: bool,
    pub live777: bool,
    pub controller: Option<String>,
    pub viewers: usize,
    pub width: u32,
    pub height: u32,
    pub fps: u32,
    pub whep_url: String,
}

#[derive(Debug, Serialize)]
pub struct MetricsResponse {
    pub cpu_percent: f32,
    pub memory_used_bytes: u64,
    pub memory_total_bytes: u64,
    pub websocket_connections: usize,
    pub forwarded_events: u64,
    pub rejected_events: u64,
    pub last_kvmd_error: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_keyboard_message() {
        let message: ControlMessage = serde_json::from_str(
            r#"{"type":"key","seq":101,"payload":{"code":"KeyA","pressed":true}}"#,
        ).unwrap();
        assert_eq!(message.seq, 101);
        assert!(matches!(message.kind, ControlKind::Key));
        let key: KeyPayload = serde_json::from_value(message.payload).unwrap();
        assert_eq!(key.code, "KeyA");
        assert!(key.pressed);
    }

    #[test]
    fn accepts_signed_absolute_coordinates() {
        let payload: AbsPayload = serde_json::from_str(r#"{"x":-32768,"y":32767}"#).unwrap();
        assert_eq!(payload.x, i16::MIN);
        assert_eq!(payload.y, i16::MAX);
    }
}
