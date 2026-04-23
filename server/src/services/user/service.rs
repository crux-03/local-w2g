use std::{collections::HashMap, sync::Arc};

use tokio::sync::{Mutex, RwLock};

use crate::{
    Snowflake,
    services::{permissions::Permissions, snowflake::SnowflakeService, user::User},
};

pub struct UserService {
    users: Arc<RwLock<HashMap<Snowflake, User>>>,
    admin: Mutex<Option<Snowflake>>,
    snowflake_service: Arc<SnowflakeService>,
}

impl UserService {
    pub fn new(snowflake_service: Arc<SnowflakeService>) -> Self {
        Self {
            users: Arc::new(RwLock::new(HashMap::new())),
            admin: Mutex::new(None),
            snowflake_service,
        }
    }

    pub async fn add_user(&self, display_name: Option<String>) -> User {
        let mut users = self.users.write().await;
        let mut admin_lock = self.admin.lock().await;

        let mut new_user = User {
            id: self.snowflake_service.generate(),
            display_name,
            permissions: Permissions::default(),
        };

        if admin_lock.is_none() {
            new_user.permissions = Permissions::admin();
            *admin_lock = Some(new_user.id);
        }

        users.entry(new_user.id).or_insert(new_user).clone()
    }

    pub async fn get_user(&self, id: &Snowflake) -> Option<User> {
        let users = self.users.read().await;
        users.get(id).cloned()
    }

    pub async fn remove_user(&self, user_id: &Snowflake) {
        let mut users = self.users.write().await;
        let mut admin_lock = self.admin.lock().await;

        if users.remove(user_id).is_none() {
            return;
        }

        if admin_lock.as_ref() == Some(user_id) {
            // Snowflakes are time-ordered, so min() == oldest
            let new_admin_id = users.keys().min().copied();
            *admin_lock = new_admin_id;

            if let Some(id) = new_admin_id
                && let Some(user) = users.get_mut(&id)
            {
                user.permissions = Permissions::admin();
            }
        }
    }

    pub async fn get_users(&self) -> Vec<User> {
        let users = self.users.read().await;
        users.values().cloned().collect()
    }
}
