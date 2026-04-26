use std::{collections::HashMap, sync::Arc};

use tokio::sync::{RwLock, mpsc};

use crate::{Snowflake, core::ServiceProvider, routes::UploadSession};

type ConnectionMap = Arc<RwLock<HashMap<Snowflake, mpsc::UnboundedSender<String>>>>;

pub struct AppState {
    provider: Arc<ServiceProvider>,
    connections: ConnectionMap,
    upload_sessions: Arc<RwLock<HashMap<Snowflake, UploadSession>>>,
}

impl AppState {
    pub async fn new() -> Result<Self, crate::Error> {
        Ok(Self {
            provider: Arc::new(ServiceProvider::new().await?),
            connections: Arc::new(RwLock::new(HashMap::new())),
            upload_sessions: Arc::new(RwLock::new(HashMap::new())),
        })
    }

    pub fn services(&self) -> &Arc<ServiceProvider> {
        &self.provider
    }

    pub fn upload_sessions(&self) -> &Arc<RwLock<HashMap<Snowflake, UploadSession>>> {
        &self.upload_sessions
    }

    pub async fn add_connection(&self, user_id: Snowflake, tx: mpsc::UnboundedSender<String>) {
        let mut conns = self.connections.write().await;
        conns.insert(user_id, tx);
        tracing::debug!("User {} connected", user_id);
    }

    pub async fn remove_connection(&self, user_id: &Snowflake) {
        let mut conns = self.connections.write().await;
        conns.remove(user_id);
        tracing::debug!("User {} disconnected", user_id);
    }

    pub async fn send_to_client(&self, user_id: &Snowflake, message: String) {
        let conns = self.connections.read().await;
        if let Some(tx) = conns.get(user_id) {
            let _ = tx.send(message);
        }
    }

    pub async fn broadcast(&self, message: String) {
        let conns = self.connections.read().await;
        for tx in conns.values() {
            let _ = tx.send(message.clone());
        }
    }

    pub async fn broadcast_except(&self, except: Snowflake, message: String) {
        let conns = self.connections.read().await;
        for (id, tx) in conns.iter() {
            if *id != except {
                let _ = tx.send(message.clone());
            }
        }
    }
}
