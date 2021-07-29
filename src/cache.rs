use std::sync::{Arc, RwLock};
use std::collections::BTreeMap;
use serde::{Serialize, Deserialize};
use uuid::Uuid;
use chrono::prelude::*;
use hashbrown::HashMap;
use crate::auth::PublicKey;
use crate::{Workout, UserId};

pub type UserKeys = Arc<RwLock<HashMap<Uuid, [u8; 32]>>>;
pub type UserWorkouts = Arc<RwLock<HashMap<Uuid, BTreeMap<DateTime<Utc>, Workout>>>>;

#[derive(Clone, Default)]
pub struct Cache {
    keys: UserKeys,
    workouts: UserWorkouts,
    buf: Vec<u8>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AuthError {
    UserNotFound(Uuid),
    MissingHeader(&'static str),
    InvalidSignature,
    ParseError(String),
}

impl Cache {
    pub fn insert_key(&self, user_id: Uuid, key: PublicKey) -> Option<PublicKey> {
        self.keys.write()
            .unwrap()
            .insert(user_id, key)
    }

    pub fn verify_request(&self, user_id: Uuid, sig: &[u8], timestamp: &[u8], body: &[u8]) -> Result<(), AuthError> {
        match self.keys.read().unwrap().get(&user_id) {
            Some(public_key) => {
                let mut buf = Vec::with_capacity(timestamp.len() + body.len());
                match crate::auth::verify_request(sig, timestamp, body, &public_key[..], &mut buf) {
                    true => Ok(()),
                    false => Err(AuthError::InvalidSignature),
                }
            }

            None => Err(AuthError::UserNotFound(user_id))
        }
    }

    pub fn parse_and_verify_request<T>(&self, sig: &[u8], timestamp: &[u8], body: &[u8]) -> Result<T, AuthError>
        where T: UserId + for<'de> Deserialize<'de>
    {
        let parsed: T = serde_json::from_slice(body)
            .map_err(|e| AuthError::ParseError(format!("failed to parse request body: {}", e)))?;

        let user_id = parsed.user_id();

        self.verify_request(user_id, sig, timestamp, body)?;

        Ok(parsed)
    }

    pub fn parse_and_verify_http_request<T>(&self, req: &http::Request<bytes::Bytes>) -> Result<T, AuthError>
        where T: UserId + for<'de> Deserialize<'de>
    {
        let parsed: T = serde_json::from_slice(&req.body().slice(..))
            .map_err(|e| AuthError::ParseError(format!("failed to parse request body: {}", e)))?;

        if req.headers().get("x-fitbod-god-mode").is_some() {
            if self.key_exists(&parsed.user_id()) {
                return Ok(parsed)
            } else {
                return Err(AuthError::UserNotFound(parsed.user_id()))
            }
        }

        let sig = req.headers().get(crate::SIG_HEADER)
            .map(|x| x.as_bytes())
            .ok_or_else(|| AuthError::MissingHeader(crate::SIG_HEADER))?;

        let timestamp = req.headers().get(crate::TIMESTAMP_HEADER)
            .map(|x| x.as_bytes())
            .ok_or_else(|| AuthError::MissingHeader(crate::TIMESTAMP_HEADER))?;

        let body = &req.body().slice(..);

        let user_id = parsed.user_id();

        self.verify_request(user_id, sig, timestamp, body)?;

        Ok(parsed)
    }

    /// returns a list of previously unseen (un-cached) workouts
    pub fn cache_workouts(&self, user_id: Uuid, workouts: &mut [Workout]) -> Vec<Workout> {
        workouts.sort_unstable_by(|a, b| a.start_time.cmp(&b.start_time));

        let mut write_lock = self.workouts.write().unwrap();

        let user_cache = write_lock
            .entry(user_id)
            .or_default();

        let mut new_workouts = Vec::new();

        for workout in workouts {
            debug_assert_eq!(workout.user_id, user_id);

            let _entry = user_cache.entry(workout.start_time)
                .or_insert_with(|| {
                    new_workouts.push(workout.clone());
                    workout.clone()
                });
        }

        new_workouts
    }

    pub fn get_cached_workouts(
        &self,
        user_id: &Uuid,
        start: Option<DateTime<Utc>>,
        end: Option<DateTime<Utc>>,
        limit: Option<usize>,
    ) -> Option<Vec<Workout>> {
        let read_lock = self.workouts.read().unwrap();

        let user_cache = read_lock.get(user_id)?;

        let start   = start.unwrap_or_else(|| Utc.ymd(1970, 1, 1).and_hms(0, 0, 0));
        let end     = end  .unwrap_or_else(|| Utc.ymd(2142, 7, 27).and_hms(0, 0, 0));
        let limit   = limit.unwrap_or(usize::MAX);

        let items = user_cache.range(start..end)
            .map(|(_, x)| x.clone())
            .rev()
            .take(limit)
            .collect();

        Some(items)
    }

    pub fn key_exists(&self, user_id: &Uuid) -> bool {
        self.keys.read().unwrap().contains_key(user_id)
    }

    pub fn workouts_exist(&self, user_id: &Uuid) -> bool {
        self.workouts.read().unwrap()
            .contains_key(user_id)
    }

    pub fn n_cached_workouts(&self, user_id: &Uuid) -> Option<usize> {
        self.workouts.read().unwrap()
            .get(user_id)
            .map(|kv| kv.len())
    }
}

#[allow(unused)]
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sanity_check_verify_request() {
        let cache = Cache::default();
        let user_id = Uuid::new_v4();
        assert_eq!(cache.key_exists(&user_id), false);
        assert_eq!(cache.workouts_exist(&user_id), false);
        assert!(matches!(cache.n_cached_workouts(&user_id), None));
        let (priv_key, pub_key) = crate::auth::gen_keypair();
        assert!(matches!(cache.insert_key(user_id, pub_key), None));
        assert_eq!(cache.key_exists(&user_id), true);

        let req = crate::api::ListWorkoutsRequest::from(user_id);
        let req_json = serde_json::to_string(&req).unwrap();
        let ts = Utc::now().timestamp();
        let ts_str = ts.to_string();
        let sig = crate::auth::sign_request(ts, &req_json, &priv_key);

        let res: Result<crate::api::ListWorkoutsRequest, AuthError> = cache.parse_and_verify_request(sig.as_bytes(), ts_str.as_bytes(), req_json.as_bytes());
        assert!(matches!(res, Ok(req)));

        let another_user_id = Uuid::new_v4();
        let another_req = crate::api::ListWorkoutsRequest::from(another_user_id);
        let another_req_json = serde_json::to_string(&another_req).unwrap();
        let res: Result<crate::api::ListWorkoutsRequest, AuthError> = cache.parse_and_verify_request(sig.as_bytes(), ts_str.as_bytes(), another_req_json.as_bytes());
        assert!(matches!(res, Err(AuthError::UserNotFound(another_user_id))));

        let res: Result<crate::api::ListWorkoutsRequest, AuthError> = cache.parse_and_verify_request("invalid sig".as_bytes(), ts_str.as_bytes(), req_json.as_bytes());
        assert!(matches!(res, Err(AuthError::InvalidSignature)));

        let res: Result<crate::api::ListWorkoutsRequest, AuthError> = cache.parse_and_verify_request(sig.as_bytes(), ts_str.as_bytes(), "invalid body".as_bytes());
        assert!(matches!(res, Err(AuthError::ParseError(_))));
    }

    #[test]
    fn sanity_check_cache_and_retrieve_workouts() {
        let cache = Cache::default();
        let user_id = Uuid::new_v4();
        let (priv_key, pub_key) = crate::auth::gen_keypair();
        cache.insert_key(user_id, pub_key);
        assert_eq!(cache.workouts_exist(&user_id), false);
        assert!(matches!(cache.get_cached_workouts(&user_id, None, None, None), None));

        let t0 = Utc.ymd(2021, 7, 27).and_hms(6, 30, 0);
        let t1 = Utc.ymd(2021, 7, 28).and_hms(6, 30, 0);
        let t2 = Utc.ymd(2021, 7, 29).and_hms(6, 30, 0);

        let get_workout = |t| -> Workout {
            Workout {
                user_id,
                workout_id: Uuid::new_v4(),
                start_time: t,
                end_time: t + chrono::Duration::hours(1),
            }
        };

        let w0 = get_workout(t0);
        let w1 = get_workout(t1);
        let w2 = get_workout(t2);

        // cache [w0]
        assert_eq!(cache.cache_workouts(user_id, &mut [w0.clone()][..]), vec![w0.clone()]);
        assert_eq!(cache.workouts_exist(&user_id), true);
        assert!(matches!(cache.n_cached_workouts(&user_id), Some(1)));

        assert!(cache.get_cached_workouts(&user_id, None, None, None).is_some());
        assert_eq!(cache.get_cached_workouts(&user_id, None, None, None).unwrap(), vec![w0.clone()]);

        // check filtering that results in 0 rows returned
        assert!(cache.get_cached_workouts(&user_id, None, None, Some(0)).is_some());
        assert_eq!(cache.get_cached_workouts(&user_id, None, None, Some(0)).unwrap(), Vec::new());
        assert_eq!(cache.get_cached_workouts(&user_id, Some(t1), None, None).unwrap(), Vec::new());
        assert_eq!(cache.get_cached_workouts(&user_id, None, Some(t0), None).unwrap(), Vec::new());

        // cache [w0, w1]
        assert_eq!(cache.cache_workouts(user_id, &mut [w0.clone(), w1.clone()][..]), vec![w1.clone()]);
        assert_eq!(cache.get_cached_workouts(&user_id, None, None, None).unwrap(), vec![w1.clone(), w0.clone()]);
        assert!(matches!(cache.n_cached_workouts(&user_id), Some(2)));

        // cache [w0, w1, w2]
        assert_eq!(cache.cache_workouts(user_id, &mut [w0.clone(), w1.clone(), w2.clone()][..]), vec![w2.clone()]);
        assert_eq!(cache.get_cached_workouts(&user_id, None, None, None).unwrap(), vec![w2.clone(), w1.clone(), w0.clone()]);
        assert!(matches!(cache.n_cached_workouts(&user_id), Some(3)));

        // check filtering by limit
        assert_eq!(cache.get_cached_workouts(&user_id, None, None, Some(0)).unwrap(), Vec::new());
        assert_eq!(cache.get_cached_workouts(&user_id, None, None, Some(1)).unwrap(), vec![w2.clone()]);
        assert_eq!(cache.get_cached_workouts(&user_id, None, None, Some(2)).unwrap(), vec![w2.clone(), w1.clone()]);
        assert_eq!(cache.get_cached_workouts(&user_id, None, None, Some(3)).unwrap(), vec![w2.clone(), w1.clone(), w0.clone()]);

        // check filtering by start
        assert_eq!(cache.get_cached_workouts(&user_id, Some(t0), None, None).unwrap(), vec![w2.clone(), w1.clone(), w0.clone()]);
        assert_eq!(cache.get_cached_workouts(&user_id, Some(t1), None, None).unwrap(), vec![w2.clone(), w1.clone()]);
        assert_eq!(cache.get_cached_workouts(&user_id, Some(t2), None, None).unwrap(), vec![w2.clone()]);
        assert_eq!(cache.get_cached_workouts(&user_id, Some(t2 + chrono::Duration::hours(1)), None, None).unwrap(), Vec::new());

        // check filtering by end
        assert_eq!(cache.get_cached_workouts(&user_id, None, Some(t2 + chrono::Duration::hours(1)), None).unwrap(), vec![w2.clone(), w1.clone(), w0.clone()]);
        assert_eq!(cache.get_cached_workouts(&user_id, None, Some(t2), None).unwrap(), vec![w1.clone(), w0.clone()]);
        assert_eq!(cache.get_cached_workouts(&user_id, None, Some(t1), None).unwrap(), vec![w0.clone()]);
        assert_eq!(cache.get_cached_workouts(&user_id, None, Some(t0), None).unwrap(), Vec::new());
    }
}
