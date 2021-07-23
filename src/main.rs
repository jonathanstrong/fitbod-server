#![allow(unused_imports)]

use std::time::*;
use std::sync::{Arc, RwLock};
use std::collections::BTreeMap;
use std::convert::{TryInto, Infallible};
use chrono::prelude::*;
use serde::{Serialize, Deserialize};
use uuid::Uuid;
use pretty_toa::ThousandsSep;
use hashbrown::HashMap;
use sqlx::Pool;
use sqlx::postgres::Postgres;
use crypto::hmac::Hmac;
use crypto::sha2::Sha256;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::runtime::Runtime;
use warp::{Filter, Rejection, Reply, filters::path::FullPath};
use http::StatusCode;
use fitbod::{Workout, ListWorkoutsRequest, ListWorkoutsResponse, User, UserId};

type UserKeys = Arc<RwLock<HashMap<Uuid, [u8; 32]>>>;
type UserWorkouts = Arc<RwLock<HashMap<Uuid, BTreeMap<DateTime<Utc>, Workout>>>>;

// /// used to extract user_id from json body
//#[derive(Debug, Clone, Copy, Deserialize)]
//struct UserId {
//    pub user_id: Uuid,
//}

#[derive(Debug, Clone, Copy)]
struct UserNotFound(Uuid);

#[derive(Debug, Clone)]
struct ParseError(String);

#[derive(Debug, Clone, Serialize, Deserialize)]
struct ErrorMsg {
    status: u16,
    error: String,
}

impl warp::reject::Reject for UserNotFound {}
impl warp::reject::Reject for ParseError {}
impl warp::reject::Reject for ErrorMsg {}

async fn handle_rejection(err: Rejection) -> Result<impl Reply, Infallible> {
    let code;
    let message;
    if let Some(UserNotFound(user_id)) = err.find() {
        code = StatusCode::UNAUTHORIZED;
        message = format!("user_id not found: {}", user_id);
    } else {
        code = StatusCode::NOT_FOUND;
        message = "not found".to_string();
    };
    let json = warp::reply::json(&ErrorMsg {
        status: code.as_u16(),
        error: message,
    });

    Ok(warp::reply::with_status(json, code))
}

fn http_request() -> impl Filter<Extract = (http::Request<bytes::Bytes>,), Error = Rejection> + Copy {
    warp::any()
        .and(warp::method())
        .and(warp::filters::path::full())
        //.and(warp::filters::query::raw())
        .and(warp::header::headers_cloned())
        .and(warp::body::bytes())
        .and_then(|method, path: FullPath, headers, bytes| async move {
            let uri = http::uri::Builder::new()
                .path_and_query(path.as_str())
                .build()
                .unwrap();
                //.map_err(Error::from)?;

            let mut request = http::Request::builder()
                .method(method)
                .uri(uri)
                .body(bytes)
                .unwrap();
                //.map_err(Error::from)?;

            *request.headers_mut() = headers;

            dbg!(&request);

            Ok::<http::Request<bytes::Bytes>, Rejection>(request)
        })
}

fn check_sig<T>(keys: &UserKeys, req: &http::Request<bytes::Bytes>) -> Result<T, ErrorMsg> //impl Filter<Extract = (T,), Error = Rejection> //+ Copy
    where for<'de> T: serde::de::Deserialize<'de>,
          T: UserId
{
    let parsed_body: T = serde_json::from_slice(&req.body().slice(..))
        .map_err(|e| ErrorMsg { status: 400, error: format!("failed to parse json body: {}", e) })?;
    let user_id = parsed_body.user_id();
    let sig = req.headers().get(fitbod::SIG_HEADER)
        .and_then(|x| x.to_str().ok())
        .ok_or_else(|| {
            eprintln!("retrieving signature header failed: {}", user_id);
            ErrorMsg { status: 400, error: format!("missing header: {}", fitbod::SIG_HEADER) }
        })?;

    let timestamp = req.headers().get(fitbod::TIMESTAMP_HEADER)
        .and_then(|x| x.to_str().ok())
        .ok_or_else(|| ErrorMsg { status: 400, error: format!("missing header: {}", fitbod::TIMESTAMP_HEADER) })?;

    let key: [u8; 32] = keys.read().unwrap()
        .get(&user_id)
        .cloned()
        .ok_or_else(|| {
            eprintln!("user not found: {}", user_id);
            ErrorMsg { status: 401, error: "authentication failed".into() }
        })?;

    let mut buf = Vec::with_capacity(1024);

    if ! fitbod::verify_request(sig.as_bytes(), timestamp.as_bytes(), &req.body().slice(..), &key[..], &mut buf) {
        eprintln!("verify_request failed: {}", user_id);
        return Err(ErrorMsg { status: 401, error: "authentication failed".into() })
    }

    //Ok::<T, Rejection>(parsed_body)
    Ok::<T, ErrorMsg>(parsed_body)
}

fn run() -> Result<(), Box<dyn std::error::Error>> {
    let run_start = Instant::now();
    dotenv::dotenv().ok();

    let rt  = Runtime::new()?;

    let keys = UserKeys::default();
    let workouts = UserWorkouts::default();

    rt.block_on(async {
        let keys = keys.clone();

        let pool = Pool::<Postgres>::connect("postgres://localhost:5432/fitbod").await.unwrap();
        let users: Vec<(Uuid, Vec<u8>)> = sqlx::query_as("select user_id, key from users").fetch_all(&pool).await.unwrap();

        {
            let mut keys = keys.write().unwrap();
            for (user_id, key) in users {
                assert_eq!(key.len(), 32);
                let key: [u8; 32] = key.try_into().unwrap();
                keys.insert(user_id, key);
            }
        }

        let mut workouts_from_recently_active_users: Vec<(Uuid, Uuid, DateTime<Utc>, DateTime<Utc>)> =
            sqlx::query_as(
                "select w.user_id, w.workout_id, w.start_time, w.end_time from workouts w \
                 where w.user_id in ( \
                     select distinct(user_id) from workouts \
                     where start_time >= now() - interval '7 days' \
                 ) and w.start_time > now() - interval '90 days' \
                 limit 1000000"
            ).fetch_all(&pool).await.unwrap();

        workouts_from_recently_active_users.sort_unstable_by_key(|x| (x.0, x.2)); // sort by (user_id, start_time)

        let n_workouts = workouts_from_recently_active_users.len();

        {
            let mut workouts = workouts.write().unwrap();
            for (user_id, workout_id, start_time, end_time) in workouts_from_recently_active_users {
                workouts.entry(user_id)
                    .or_default()
                    .insert(start_time, Workout { user_id, workout_id, start_time, end_time });
            }
        }

        println!("cached {} workouts from {} users ({} users total) in {:?}",
            n_workouts.thousands_sep(),
            workouts.read().unwrap().len().thousands_sep(),
            keys.read().unwrap().len().thousands_sep(),
            Instant::now() - run_start,
        );

        let request_sig = warp::header::<String>(fitbod::SIG_HEADER);
        let request_timestamp = warp::header::<String>(fitbod::TIMESTAMP_HEADER);

        let get_keys = warp::any().map(move || keys.clone());
        let get_workouts = warp::any().map(move || workouts.clone());

        let api_routes = warp::path("api")
            .and(warp::path("v1"))
            .and(warp::post())
            .and(request_sig)
            .and(request_timestamp);

        let list_workouts = api_routes
            .and(warp::path("workouts"))
            .and(warp::path("list"))
            .and(http_request())
            .and(get_keys)
            .and_then(|_, _, req, keys| async move { //-> Result<ListWorkoutsResponse, ErrorMsg> {
                //let keys = keys.clone();
                match check_sig::<ListWorkoutsRequest>(&keys, &req) {
                    Ok(list_req) => {
                        let list_resp = ListWorkoutsResponse {
                            user_id: list_req.user_id(),
                            n_items: 0,
                            items: vec![],
                        };
                        Ok(warp::reply::json(&list_resp))
                    }

                    Err(e) => Err(warp::reject::custom(e))
                }
            });
            
            //.and(warp::body::json())
            //.map(move |sig, ts, req: ListWorkoutsRequest| {
            //    match keys.read().unwrap().get(&req.user_id) {
            //        Some(key) => {
            //            format!("sig = {}, ts = {}, req = {:?}", sig, ts, req)
            //        }

            //        None => { // user not found
            //            //warp::reject::custom(UserNotFound(req.user_id))
            //            "unauthorized".to_string()
            //        }
            //    }
            //});

            //.and_then(|sig, ts, body: bytes::Bytes| -> {
            //    serde_json::from_slice::<UserId>(&body.slice(..))
            //        .map_err(|e| warp::reject::custom(ParseError(format!("parsing json failed: {}", e))))
            //        .and_then(|UserId { user_id }| {
            //            match keys.read().unwrap().get(&user_id) {
            //                Some(key) => {
            //                    Ok(warp::reply::html("authorized"))
            //                }

            //                None => { // user not found
            //                    Err(warp::reject::custom(UserNotFound(user_id)))
            //                }
            //            }
            //        })
            //    //format!("sig = {}, ts = {}", sig, ts)
            //});

        let routes = list_workouts;
        warp::serve(routes).run(([127, 0, 0, 1], 3030)).await;
    });

    Ok(())
}

fn main() {
    run().unwrap()
}
