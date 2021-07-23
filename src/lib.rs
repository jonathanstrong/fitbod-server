use serde::{Serialize, Deserialize};
use chrono::prelude::*;
use uuid::Uuid;
use rand::prelude::*;

pub const SIG_HEADER: &str = "x-fitbod-access-signature";
pub const TIMESTAMP_HEADER: &str = "x-fitbod-access-timestamp";
pub const API_VERSION: &str = "v1";

/// user representation matching `users` db table
pub struct User {
    pub user_id: Uuid,
    pub email: String,
    pub key: [u8; 32],
    pub created: DateTime<Utc>,
}

/// workout representation matching `workouts` db table
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Workout {
    pub workout_id: Uuid,
    pub user_id: Uuid,
    pub start_time: DateTime<Utc>,
    pub end_time: DateTime<Utc>,
}

/// an api request to save a workout to the database
pub type NewWorkoutRequest = Workout; // api request is identical to schema representation

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

type PrivateKey     = [u8; 64];
type PublicKey      = [u8; 32];

pub fn gen_keypair() -> (PrivateKey, PublicKey) {
    let mut seed = [0u8; 32];
    let mut rng = thread_rng();
    rng.fill(&mut seed[..]);
    crypto::ed25519::keypair(&seed[..])
}

pub fn verify_request(sig: &str, timestamp: &str, body: &str, pub_key: &[u8], buf: &mut Vec<u8>) -> bool {
    buf.clear();
    buf.extend_from_slice(timestamp.as_bytes());
    buf.extend_from_slice(body.as_bytes());
    let n = buf.len();
    if let Err(_) = base64::decode_config_buf(sig.as_bytes(), base64::STANDARD, buf) {
        return false
    }
    let msg = &buf[..n];
    let decoded_sig = &buf[n..];

    debug_assert_eq!(msg.len(), timestamp.len() + body.len());
    debug_assert_eq!(pub_key.len(), 64);
    debug_assert_eq!(decoded_sig.len(), 64);

    crypto::ed25519::verify(msg, pub_key, decoded_sig)
}

#[allow(unused)]
#[cfg(test)]
mod tests {
    use super::*;
    use rand::prelude::*;

    #[test]
    fn check_ed25519_sig_example_in_api_docs() {
        let priv_key_encoded = "jCNLYN8zGyiVM7omRHGlY1iyJuvAZBWZGuN+9TjaWJTSzZ3oEvXq7QNHTwwD785/rBnmRCPkl2D68lRyvBWHUg==";
        let priv_key = base64::decode(priv_key_encoded.as_bytes()).unwrap();
        assert_eq!(priv_key.len(), 64);
        let unix_timestamp = "1627062582";
        let request_body = r#"{"user_id":"3a2cbc79-00e5-4598-a5b2-74c5059724af"}"#;
        assert_eq!(request_body.len(), 50);
        let signature_contents = format!("{}{}", unix_timestamp, request_body);
        let sig = crypto::ed25519::signature(signature_contents.as_bytes(), &priv_key[..]);
        let encoded_sig = base64::encode(&sig[..]);
        let sig_header = format!("{}: {}", SIG_HEADER, encoded_sig);
        let timestamp_header = format!("{}: {}", TIMESTAMP_HEADER, unix_timestamp);
        let pub_key = &priv_key[32..]; // this will be retrieved from users table in actual application code
        assert!( crypto::ed25519::verify(signature_contents.as_bytes(), pub_key, &sig[..]) );
        let mut buf = Vec::new();
        assert!( verify_request(&encoded_sig, unix_timestamp, request_body, pub_key, &mut buf) );
    }

    #[test]
    fn check_how_ed25519_pub_priv_keypair_works() {
        let mut seed = [0u8; 32];
        let mut rng = thread_rng();
        rng.fill(&mut seed[..]);
        let (priv_key, pub_key) = crypto::ed25519::keypair(&seed[..]);
        let exch_key = crypto::ed25519::exchange(&pub_key[..], &priv_key[..]);
        let msg = r#"{"user_id":"3a2cbc79-00e5-4598-a5b2-74c5059724af","kind":"ping"}"#;
        let sig = crypto::ed25519::signature(msg.as_bytes(), &priv_key[..]);
        assert_eq!(crypto::ed25519::verify(msg.as_bytes(), &pub_key[..], &sig[..]), true);
    }
}
