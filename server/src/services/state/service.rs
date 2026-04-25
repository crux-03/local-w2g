use std::{
    collections::{HashMap, HashSet},
    sync::Arc,
    time::{Duration, Instant},
};

use tokio::sync::{Mutex, RwLock};

use crate::{
    Snowflake,
    services::{
        state::{HandshakeOutcome, UserReadiness, UserReadinessView, Verdict, VideoReadiness},
        user::UserService,
        videos::VideoService,
    },
};

const HEARTBEAT_TIMEOUT: Duration = Duration::from_secs(25);
pub const PLAY_HANDSHAKE_TIMEOUT: Duration = Duration::from_secs(3);

#[derive(Debug)]
struct PendingPlay {
    required: HashSet<Snowflake>,
    confirmed: HashSet<Snowflake>,
    video_id: Snowflake,
}

pub struct StateService {
    ready_states: RwLock<HashMap<Snowflake, UserReadiness>>,
    last_heartbeats: RwLock<HashMap<Snowflake, Instant>>,
    pending_plays: Mutex<HashMap<Snowflake, PendingPlay>>,
    user_service: Arc<UserService>,
    video_service: Arc<VideoService>,
}

impl StateService {
    pub fn new(user_service: Arc<UserService>, video_service: Arc<VideoService>) -> Self {
        Self {
            ready_states: RwLock::new(HashMap::new()),
            last_heartbeats: RwLock::new(HashMap::new()),
            pending_plays: Mutex::new(HashMap::new()),
            user_service,
            video_service,
        }
    }

    /// Record a client's assertion about one of their local files. Assertions
    /// for video_ids not in the index are silently dropped.
    pub async fn assert_ready(
        &self,
        user_id: Snowflake,
        video_id: Snowflake,
        on_device: bool,
    ) -> Option<UserReadinessView> {
        if self.video_service.resolve_path(video_id).await.is_none() {
            tracing::debug!(%user_id, %video_id, "ignored readiness for unknown video");
            return None;
        }

        let new_state = if on_device {
            VideoReadiness::OnDevice
        } else {
            VideoReadiness::NotStarted
        };

        {
            let mut states = self.ready_states.write().await;
            let entry = states.entry(user_id).or_insert_with(|| UserReadiness {
                videos: HashMap::new(),
            });
            entry.videos.insert(video_id, new_state);
        }

        Some(self.view_for(user_id).await)
    }

    pub async fn assert_ready_bulk(
        &self,
        user_id: Snowflake,
        on_device: Vec<Snowflake>,
    ) -> UserReadinessView {
        let indexed: Vec<Snowflake> = self.video_service.list_ids().await;
        let on_device_set: HashSet<Snowflake> = on_device
            .into_iter()
            .filter(|id| indexed.contains(id))
            .collect();

        let mut states = self.ready_states.write().await;
        let entry = states.entry(user_id).or_insert_with(|| UserReadiness {
            videos: HashMap::new(),
        });
        entry.videos.clear();
        for id in &indexed {
            let status = if on_device_set.contains(id) {
                VideoReadiness::OnDevice
            } else {
                VideoReadiness::NotStarted
            };
            entry.videos.insert(*id, status);
        }
        drop(states);

        self.view_for(user_id).await
    }

    pub async fn heartbeat(&self, user_id: Snowflake) {
        self.last_heartbeats
            .write()
            .await
            .insert(user_id, Instant::now());
    }

    /// Build the view for a user, including derived verdict.
    pub async fn view_for(&self, user_id: Snowflake) -> UserReadinessView {
        let videos = self
            .ready_states
            .read()
            .await
            .get(&user_id)
            .map(|r| r.videos.clone())
            .unwrap_or_default();

        let verdict = self.compute_verdict(&videos).await;

        UserReadinessView { videos, verdict }
    }

    async fn compute_verdict(&self, videos: &HashMap<Snowflake, VideoReadiness>) -> Verdict {
        let indexed: Vec<Snowflake> = self.video_service.list_ids().await;
        if indexed.is_empty() {
            return Verdict::Ready;
        }
        let on_device = indexed
            .iter()
            .filter(|id| matches!(videos.get(id), Some(VideoReadiness::OnDevice)))
            .count();

        if on_device == indexed.len() {
            Verdict::Ready
        } else if on_device == 0 {
            Verdict::NotReady
        } else {
            Verdict::Partial
        }
    }

    /// Scan heartbeats; any user past the timeout has their readiness cleared
    /// and is returned so the caller can broadcast updates.
    pub async fn sweep_stale(&self) -> Vec<Snowflake> {
        let now = Instant::now();
        let mut stale: Vec<Snowflake> = Vec::new();
        {
            let heartbeats = self.last_heartbeats.read().await;
            for (user_id, last) in heartbeats.iter() {
                if now.duration_since(*last) > HEARTBEAT_TIMEOUT {
                    stale.push(*user_id);
                }
            }
        }
        if !stale.is_empty() {
            let mut states = self.ready_states.write().await;
            for id in &stale {
                states.remove(id);
            }
        }
        stale
    }

    pub async fn remove_user(&self, user_id: Snowflake) {
        self.ready_states.write().await.remove(&user_id);
        self.last_heartbeats.write().await.remove(&user_id);
        // Any pending play this user was required for: remove them from
        // required so remaining confirmations can complete.
        let mut plays = self.pending_plays.lock().await;
        for play in plays.values_mut() {
            play.required.remove(&user_id);
            play.confirmed.remove(&user_id);
        }
    }

    /// Begin a handshake. Records which users need to confirm and returns
    /// the request id.
    pub async fn begin_play_handshake(&self, request_id: Snowflake, video_id: Snowflake) {
        let required: HashSet<Snowflake> = self
            .user_service
            .get_users()
            .await
            .iter()
            .map(|u| u.id)
            .collect();
        self.pending_plays.lock().await.insert(
            request_id,
            PendingPlay {
                required,
                confirmed: HashSet::new(),
                video_id,
            },
        );
    }

    pub async fn confirm_play(
        &self,
        request_id: Snowflake,
        user_id: Snowflake,
    ) -> HandshakeOutcome {
        let mut plays = self.pending_plays.lock().await;
        let Some(play) = plays.get_mut(&request_id) else {
            return HandshakeOutcome::AlreadyResolved;
        };
        if !play.required.contains(&user_id) {
            return HandshakeOutcome::Pending;
        }
        play.confirmed.insert(user_id);
        if play.confirmed == play.required {
            let video_id = play.video_id;
            plays.remove(&request_id);
            HandshakeOutcome::AllConfirmed { video_id }
        } else {
            HandshakeOutcome::Pending
        }
    }

    /// Called by the timeout task. Returns the non-confirmers if the request
    /// was still pending, or None if it had already resolved.
    pub async fn timeout_play(&self, request_id: Snowflake) -> Option<Vec<Snowflake>> {
        let mut plays = self.pending_plays.lock().await;
        let play = plays.remove(&request_id)?;
        Some(play.required.difference(&play.confirmed).copied().collect())
    }
}
