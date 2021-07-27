use serde::{Serialize, Deserialize};
use chrono::prelude::*;
use uuid::Uuid;
use crate::Workout;

pub const SIG_HEADER        : &str = "x-fitbod-access-signature";
pub const TIMESTAMP_HEADER  : &str = "x-fitbod-access-timestamp";
pub const API_VERSION       : &str = "v1";

/// an api request to save a workout to the database
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NewWorkoutsRequest {
    pub user_id: Uuid,
    pub items: Vec<Workout>,
}

/// api response to a `NewWorkoutRequest`
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "result")]
#[serde(rename_all = "snake_case")]
pub enum NewWorkoutResponse {
    Success {
        workout_id: Uuid,
    },

    Error {
        workout_id: Uuid,
        err_code: u32,
        msg: String,
    },
}

/// api request to list workouts for a user
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ListWorkoutsRequest {
    pub user_id: Uuid,
    #[serde(default)]
    pub start: Option<DateTime<Utc>>,
    #[serde(default)]
    pub end: Option<DateTime<Utc>>,
    #[serde(default)]
    pub limit: Option<usize>,
}

/// listed workout in api response `ListWorkoutsResponse`
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ListWorkoutsItem {
    pub workout_id: Uuid,
    pub date: NaiveDate,
    pub duration_minutes: u32,
}

/// api response to `ListWorkoutsRequest`
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ListWorkoutsResponse {
    pub user_id: Uuid,
    pub n_items: usize,
    pub items: Vec<ListWorkoutsItem>,
}

/// api response representing various kinds of events
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "event_kind")]
#[serde(rename_all = "snake_case")]
pub enum Event {
    NewWorkout(ListWorkoutsItem),
    DopamineShot {
        message: String
    },
}

/// api request to subscribe to events for a user
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubscribeEventsRequest {
    pub user_id: Uuid,
}

/// api response that contains a list of new events for a user
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NewEvents {
    pub user_id: Uuid,
    pub n_items: usize,
    pub items: Vec<Event>,
}

impl<'a> From<&'a Workout> for ListWorkoutsItem {
    fn from(workout: &'a Workout) -> Self {
        let &Workout { workout_id, start_time: start, end_time: end, .. } = workout;
        Self {
            workout_id,
            date: start.date().naive_local(),
            duration_minutes: ((end - start).num_seconds() as f64 / 60.0).round() as u32,
        }
    }
}

impl From<Uuid> for ListWorkoutsRequest {
    fn from(user_id: Uuid) -> Self {
        Self {
            user_id,
            start: None,
            end: None,
            limit: None,
        }
    }
}
