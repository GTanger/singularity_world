//! WebSocket 連線上限與全服廣播（對齊 Go `server/websocket.go`）。

use std::sync::Mutex;

use tokio::sync::mpsc::Sender;
use uuid::Uuid;

pub const SEND_BUFFER_SIZE: usize = 256;

pub struct HubSlot {
    pub id: Uuid,
    pub tx: Sender<Vec<u8>>,
}

pub struct Hub {
    max_conn: usize,
    slots: Mutex<Vec<HubSlot>>,
}

impl Hub {
    #[must_use]
    pub fn new(max_conn: usize) -> Self {
        Self {
            max_conn,
            slots: Mutex::new(Vec::new()),
        }
    }

    /// 已達上限回傳 `None`。
    pub fn register(&self, id: Uuid, tx: Sender<Vec<u8>>) -> bool {
        let mut g = self.slots.lock().expect("hub poisoned");
        if g.len() >= self.max_conn {
            return false;
        }
        g.push(HubSlot { id, tx });
        true
    }

    pub fn unregister(&self, id: Uuid) {
        let mut g = self.slots.lock().expect("hub poisoned");
        g.retain(|s| s.id != id);
    }

    pub fn broadcast(&self, msg: Vec<u8>) {
        let g = self.slots.lock().expect("hub poisoned");
        for s in g.iter() {
            let _ = s.tx.try_send(msg.clone());
        }
    }
}
