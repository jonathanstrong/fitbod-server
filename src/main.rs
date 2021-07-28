use std::time::*;
use chrono::prelude::*;
use serde::{Serialize, Deserialize};
use itertools::Itertools;
use pretty_toa::ThousandsSep;
use tokio::runtime::Runtime;
use warp::{Filter, Rejection, Reply, filters::path::FullPath};
use fitbod::{Workout, ListWorkoutsRequest, ListWorkoutsResponse, NewWorkoutsRequest};

#[derive(Debug, Clone, Serialize, Deserialize)]
struct ErrorMsg {
    status: u16,
    error: String,
}

impl warp::reject::Reject for ErrorMsg {}

async fn handle_rejection(err: Rejection) -> Result<impl Reply, std::convert::Infallible> {
    let code;
    let message;
    if let Some(ErrorMsg { status, error }) = err.find() {
        code = http::StatusCode::from_u16(*status).unwrap();
        message = error.to_string();
    } else {
        code = http::StatusCode::NOT_FOUND;
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

async fn init_cache(cache: &fitbod::cache::Cache, db: &fitbod::db::DataBase) {
    let init_start = Instant::now();
    let user_keys = db.fetch_user_keys().await.unwrap();
    let n_users = user_keys.len();
    for (user_id, key) in user_keys {
        cache.insert_key(user_id, key);
    }
    let mut prefetch_workouts: Vec<fitbod::Workout> = db.fetch_recently_active_user_workouts().await.unwrap();
    prefetch_workouts.sort_unstable_by_key(|x| (x.user_id, x.start_time));
    let n_workouts = prefetch_workouts.len();
    let mut n_users_cached = 0;
    for (user_id, user_workouts) in &prefetch_workouts.into_iter().group_by(|x| x.user_id) {
        n_users_cached += 1;
        let mut user_workouts: Vec<Workout> = user_workouts.collect();
        cache.cache_workouts(user_id, &mut user_workouts[..]);
    }

    println!("cached {} workouts from {} users ({} users total) in {:?}",
        n_workouts.thousands_sep(),
        n_users_cached.thousands_sep(),
        n_users.thousands_sep(),
        Instant::now() - init_start,
    );
}


fn run() -> Result<(), Box<dyn std::error::Error>> {
    dotenv::dotenv().ok();

    let db_url = std::env::var("DATABASE_URL")
        .ok()
        .unwrap_or_else(|| "postgres://localhost:5432/fitbod".to_string());

    let rt  = Runtime::new()?;

    rt.block_on(async {
        let cache = fitbod::cache::Cache::default();
        let db = fitbod::db::DataBase::new(&db_url).await.unwrap();

        init_cache(&cache, &db).await;

        let cache = warp::any().map(move || cache.clone());
        let db = warp::any().map(move || db.clone());

        let api_routes = warp::path("api")
            .and(warp::path("v1"))
            .and(warp::post())
            .and(cache)
            .and(db);

        let list_workouts = api_routes.clone()
            .and(warp::path("workouts"))
            .and(warp::path("list"))
            .and(http_request())
            .and_then(|cache: fitbod::cache::Cache, db: fitbod::db::DataBase, http_req| async move {
                match cache.parse_and_verify_http_request::<ListWorkoutsRequest>(&http_req) {
                    Ok(req) => {
                        match cache.get_cached_workouts(&req.user_id, req.start, req.end, req.limit) {
                            Some(workouts) => {
                                let items: Vec<_> = workouts.iter()
                                    .map(|x| fitbod::api::ListWorkoutsItem::from(x))
                                    .collect();
                                let resp = ListWorkoutsResponse {
                                    user_id: req.user_id,
                                    n_items: items.len(),
                                    items,
                                };
                                Ok(warp::reply::json(&resp))
                            }

                            None => {
                                match db.fetch_user_workouts(&req.user_id).await {
                                    Ok(mut workouts) if ! workouts.is_empty() => {
                                        // cache db results
                                        cache.cache_workouts(req.user_id, &mut workouts[..]);
                                        // apply request filters to db results
                                        workouts.sort_unstable_by(|a, b| a.start_time.cmp(&b.start_time));
                                        let start   = req.start.unwrap_or_else(|| Utc.ymd(1970, 1, 1).and_hms(0, 0, 0));
                                        let end     = req.end  .unwrap_or_else(|| Utc.ymd(2142, 7, 27).and_hms(0, 0, 0));
                                        let limit   = req.limit.unwrap_or(usize::MAX);
                                        let filtered: Vec<_> = workouts.into_iter()
                                            .rev()
                                            .filter(|x| {
                                                x.start_time >= start
                                                && x.end_time < end
                                            })
                                            .take(limit)
                                            .map(|x| fitbod::api::ListWorkoutsItem::from(&x))
                                            .collect();
                                        let resp = ListWorkoutsResponse {
                                            user_id: req.user_id,
                                            n_items: filtered.len(),
                                            items: filtered,
                                        };
                                        Ok(warp::reply::json(&resp))
                                    }

                                    Ok(empty) => {
                                        assert!(empty.is_empty());
                                        let resp = ListWorkoutsResponse {
                                            user_id: req.user_id,
                                            n_items: 0,
                                            items: Vec::new(),
                                        };
                                        Ok(warp::reply::json(&resp))
                                    }

                                    Err(e) => {
                                        Err(warp::reject::custom(ErrorMsg {
                                            status: 500,
                                            error: format!("database error: {}", e),
                                        }))
                                    }
                                }

                            }
                        }
                    }

                    Err(e) => {
                        Err(warp::reject::custom(ErrorMsg {
                            status: 400,
                            error: format!("auth error: {:?}", e),
                        }))
                    }
                }
            });

        let new_workouts = api_routes
            .and(warp::path("workouts"))
            .and(warp::path("new"))
            .and(http_request())
            .and_then(|cache: fitbod::cache::Cache, db: fitbod::db::DataBase, http_req| async move {
                match cache.parse_and_verify_http_request::<NewWorkoutsRequest>(&http_req) {
                    Ok(mut req) => {
                        let unseen = cache.cache_workouts(req.user_id, &mut req.items[..]);

                        if ! unseen.is_empty() {
                            match db.insert_workouts(&unseen).await {
                                Ok(_) => Ok(warp::reply::with_status(warp::reply::reply(), http::StatusCode::NO_CONTENT)),

                                Err(e) => {
                                    Err(warp::reject::custom(ErrorMsg {
                                        status: 500,
                                        error: format!("database error: {}", e),
                                    }))
                                }
                            }
                        } else {
                            Ok(warp::reply::with_status(warp::reply::reply(), http::StatusCode::NO_CONTENT))
                        }
                    }

                    Err(e) => {
                        Err(warp::reject::custom(ErrorMsg {
                            status: 400,
                            error: format!("auth error: {:?}", e),
                        }))
                    }
                }
            });

        let ping = warp::get()
            .and(warp::path("ping"))
            .map(|| { 
                warp::reply::with_status(warp::reply::reply(), http::StatusCode::NO_CONTENT)
            });
          
        let routes = list_workouts
            .or(new_workouts)
            .or(ping)
            .recover(handle_rejection);

        warp::serve(routes).run(([127, 0, 0, 1], 3030)).await;
    });

    Ok(())
}

fn main() {
    run().unwrap()
}
