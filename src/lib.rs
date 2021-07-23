use serde::{Serialize, Deserialize};
use chrono::prelude::*;
use uuid::Uuid;
use rand::prelude::*;
use crypto::hmac::Hmac;
use crypto::sha2::Sha256;
use crypto::mac::Mac;

pub const SIG_HEADER: &str = "x-fitbod-signature";
pub const API_VERSION: &str = "v1";

/// user representation matching `users` db table
pub struct User {
    pub user_id: Uuid,
    pub email: String,
    pub secret: Vec<u8>,
    pub created: DateTime<Utc>,
}

/// workout representation matching `workouts` db table
pub struct Workout {
    pub workout_id: Uuid,
    pub user_id: Uuid,
    pub start_time: DateTime<Utc>,
    pub end_time: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NewWorkoutRequest {
    pub workout_id: Uuid,
    pub user_id: Uuid,
    pub start_time: DateTime<Utc>,
    pub end_time: DateTime<Utc>,
}

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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ListWorkoutsItem {
    pub workout_id: Uuid,
    pub date: NaiveDate,
    pub duration_minutes: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ListWorkoutsResponse {
    pub user_id: Uuid,
    pub n_items: usize,
    pub items: Vec<ListWorkoutsItem>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "event_kind")]
#[serde(rename_all = "snake_case")]
pub enum Event {
    NewWorkout(ListWorkoutsItem),
    DopamineShot {
        message: String
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubscribeEventsRequest {
    pub user_id: Uuid,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NewEvents {
    pub user_id: Uuid,
    pub n_items: usize,
    pub items: Vec<Event>,
}

pub fn get_hmac(secret: &[u8]) -> Hmac<Sha256> {
    Hmac::new(Sha256::new(), secret)
}

pub fn sign_request(body: &[u8], hmac: &mut Hmac<Sha256>, buf: &mut [u8]) -> usize {
    hmac.reset();
    hmac.input(body);
    base64::encode_config_slice(hmac.result().code(), base64::STANDARD, buf)
}

pub fn gen_secret() -> [u8; 64] {
    let mut rng = thread_rng();
    let mut buf = [0u8; 64];
    rng.fill(&mut buf[..]);
    buf
}

pub fn gen_secret_base64() -> String {
    let secret = gen_secret();
    let mut out = String::with_capacity(64);
    base64::encode_config_buf(&secret[..], base64::STANDARD, &mut out);
    out
}

#[allow(unused)]
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn check_sig_example_against_actual_output() {
        let secret = "6KQ1CMZGFP84mJoip2crsGw5HpBhctnQ6Zkpj4/pVEqx/enTKvvwjpp57Nq7JS9gqjxyM1PtXcEHJxC0gag+dA==";
        let secret_decoded = base64::decode_config(secret.as_bytes(), base64::STANDARD).unwrap();
        let mut hmac = Hmac::new(Sha256::new(), &secret_decoded);
        let request_body = r#"{"user_id":"3a2cbc79-00e5-4598-a5b2-74c5059724af","kind":"ping"}"#;
        let mut buf = [0u8; 1024];
        let sig_length = crate::sign_request(request_body.as_bytes(), &mut hmac, &mut buf[..]);
        let sig = &buf[..sig_length]; // -> Fn7nQsY3UqVKVr1kL7O+yP7J7WSM660oaNbSq42Vy7A=
        let sig = std::str::from_utf8(&buf[..sig_length]).unwrap(); // -> Fn7nQsY3UqVKVr1kL7O+yP7J7WSM660oaNbSq42Vy7A=
        let expected_sig = "Fn7nQsY3UqVKVr1kL7O+yP7J7WSM660oaNbSq42Vy7A=";
        assert_eq!(sig, expected_sig);
    }
}
