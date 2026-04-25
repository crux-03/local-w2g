use std::{collections::VecDeque, sync::Arc};

use chrono::Utc;
use tokio::sync::RwLock;

use crate::{
    Snowflake,
    services::{
        message::{Entry, EntryKind, WidgetState},
        snowflake::SnowflakeService,
    },
};

pub struct MessageService {
    messages: Arc<RwLock<VecDeque<Entry>>>,
    snowflake_service: Arc<SnowflakeService>,
}

impl MessageService {
    pub fn new(capacity: usize, snowflake_service: Arc<SnowflakeService>) -> Self {
        Self {
            messages: Arc::new(RwLock::new(VecDeque::with_capacity(capacity))),
            snowflake_service,
        }
    }

    pub async fn get(&self, id: Snowflake) -> Option<Entry> {
        self.messages
            .read()
            .await
            .iter()
            .find(|e| e.id == id)
            .cloned()
    }

    pub async fn list(&self) -> Vec<Entry> {
        self.messages.read().await.iter().cloned().collect()
    }

    async fn append(&self, kind: EntryKind) -> Entry {
        let entry = Entry {
            id: self.snowflake_service.generate(),
            timestamp: Utc::now().timestamp_millis(),
            kind,
        };
        self.messages.write().await.push_back(entry.clone());
        entry
    }

    pub async fn user_message(&self, user_id: Snowflake, content: String) -> Entry {
        self.append(EntryKind::Chat {
            sender: user_id,
            content,
        })
        .await
    }

    pub async fn system_log(&self, content: String) -> Entry {
        self.append(EntryKind::System { content }).await
    }

    pub async fn create_widget(&self, state: WidgetState) -> Entry {
        self.append(EntryKind::Widget { state, done: false }).await
    }

    pub async fn update_widget(
        &self,
        id: Snowflake,
        new_state: WidgetState,
    ) -> Result<Entry, crate::Error> {
        let mut messages = self.messages.write().await;
        let widget_entry = messages
            .iter_mut()
            .find(|x| x.id == id)
            .ok_or(crate::Error::MessageNotFound)?;

        match &mut widget_entry.kind {
            EntryKind::Widget { state, done } => {
                if *done {
                    return Err(crate::Error::WidgetAlreadyDone);
                }
                *state = new_state;
            }
            _ => return Err(crate::Error::NotAWidget),
        }
        Ok(widget_entry.clone())
    }

    pub async fn finish_widget(
        &self,
        id: Snowflake,
        new_state: WidgetState,
    ) -> Result<Entry, crate::Error> {
        let mut messages = self.messages.write().await;

        let widget_entry = messages
            .iter_mut()
            .find(|x| x.id == id)
            .ok_or(crate::Error::MessageNotFound)?;

        match &mut widget_entry.kind {
            EntryKind::Widget { state, done } => {
                if *done {
                    return Err(crate::Error::WidgetAlreadyDone);
                }
                *state = new_state;
                *done = true; // Permanently set to true in the store
            }
            _ => return Err(crate::Error::NotAWidget),
        }

        // 3. Clone and return the updated state
        Ok(widget_entry.clone())
    }
}
