use serde::{Serialize, Deserialize};
use chrono::prelude::*;
use uuid::Uuid;

pub use api::*;
pub use auth::*;

pub mod api;
pub mod auth;
pub mod cache;
pub mod db;

/// user representation matching `users` db table
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct User {
    pub user_id: Uuid,
    pub email: String,
    pub key: [u8; 32],
    pub created: DateTime<Utc>,
}

/// workout representation matching `workouts` db table
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Workout {
    pub workout_id: Uuid,
    pub user_id: Uuid,
    pub start_time: DateTime<Utc>,
    pub end_time: DateTime<Utc>,
}

pub trait UserId {
    fn user_id(&self) -> Uuid;
}

macro_rules! impl_user_id {
    ($t:ty) => {
        impl UserId for $t {
            fn user_id(&self) -> Uuid {
                self.user_id
            }
        }
    }
}

impl_user_id!(User);
impl_user_id!(Workout);
impl_user_id!(SubscribeEventsRequest);
impl_user_id!(ListWorkoutsRequest);
impl_user_id!(NewWorkoutsRequest);
