use std::time::*;
use std::convert::TryInto;
use std::net::SocketAddr;
use uuid::Uuid;
use std::path::{PathBuf, Path};
use chrono::prelude::*;
use rand::prelude::*;
use serde::{Serialize, Deserialize};
use itertools::Itertools;
use pretty_toa::ThousandsSep;
use tokio::runtime::Runtime;
use warp::{Filter, Rejection, Reply, filters::path::FullPath};
use structopt::StructOpt;
use fitbod::{Workout, ListWorkoutsRequest, ListWorkoutsResponse, NewWorkoutsRequest};

/// fitbod api example server
///
/// DATABASE_URL env var must be present with postgres connection info
#[derive(StructOpt)]
#[structopt(author = env!("CARGO_PKG_AUTHORS"))]
enum Opt {
    /// run the server, listening on the provided address for incoming http requests
    Run {
        /// api server address to listen on
        #[structopt(value_name = "ADDR")]
        bind: SocketAddr,
    },

    /// print example http request for /api/v1/workouts/list endpoint to stdout
    ListWorkoutsRequest {
        #[structopt(short = "u", long, default_value = "var/example-users.csv")]
        users_csv_path: PathBuf,

        /// defaults to a user id randomly chosen from the file
        #[structopt(long)]
        user_id: Option<Uuid>,

        /// pick user by email instead of user_id. this will search the --users-csv-path
        /// data to find the correct UUID by email
        #[structopt(long, conflicts_with = "user_id")]
        email: Option<String>,

        /// filter results by end (YYYY-MM-DD)
        #[structopt(long)]
        start: Option<NaiveDate>,

        /// filter results by end (YYYY-MM-DD)
        #[structopt(long)]
        end: Option<NaiveDate>,

        /// specify limit to request
        #[structopt(long)]
        limit: Option<usize>,

        /// value of http host header
        #[structopt(long, default_value = "fitbod.jstrong.dev")]
        host: String,

        /// output curl command instead of http request text
        #[structopt(long)]
        curl: bool,

        /// for --curl mode, what address to connect to to send request
        #[structopt(short, long, default_value = "https://fitbod.jstrong.dev")]
        connect: String,
    },

    /// print example http request for /api/v1/workouts/new endpoint to stdout
    NewWorkoutsRequest {
        #[structopt(short = "u", long, default_value = "var/example-users.csv")]
        users_csv_path: PathBuf,

        /// defaults to a user id randomly chosen from the file
        #[structopt(long)]
        user_id: Option<Uuid>,

        /// pick user by email instead of user_id. this will search the --users-csv-path
        /// data to find the correct UUID by email
        #[structopt(long, conflicts_with = "user_id")]
        email: Option<String>,

        /// date of workout
        date: NaiveDate,

        /// workout duration in minutes
        duration: u32,

        /// value of http host header
        #[structopt(long, default_value = "fitbod.jstrong.dev")]
        host: String,

        /// output curl command instead of http request text
        #[structopt(long)]
        curl: bool,

        /// for --curl mode, what address to connect to to send request
        #[structopt(short, long, default_value = "https://fitbod.jstrong.dev")]
        connect: String,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LocalUserData {
    pub user_id: Uuid,
    pub email: String,
    pub private_key: String,
    pub public_key: String,
}

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

            //dbg!(&request);

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


fn run(db_url: &str, bind: SocketAddr) -> Result<(), Box<dyn std::error::Error>> {
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
                        // if no workouts are cached, fetch from db so we know which of these
                        // ones are new
                        //
                        // this could be improved - perhaps the insert query could be converted
                        // to upsert + select in the case that we have no cache for the user
                        //
                        if ! cache.workouts_exist(&req.user_id) {
                            if let Ok(mut db_workouts) = db.fetch_user_workouts(&req.user_id).await {
                                let _ = cache.cache_workouts(req.user_id, &mut db_workouts[..]);
                            }
                        }

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

        let base_ping = warp::get()
            .and(warp::path("ping"));

        let api_ping = warp::get()
            .and(warp::path("api"))
            .and(warp::path("v1"))
            .and(warp::path("ping"));

        let ping = base_ping.or(api_ping);

        let god_mode_ping = ping.clone()
            .and(warp::header::exact("x-fitbod-god-mode", "1"))
            .map(|_| { 
                "GOD MODE PONG!\n"
            });

        let ping = god_mode_ping.or(ping.map(|_| "pong\n"));
          
        let routes = list_workouts
            .or(new_workouts)
            .or(ping)
            .recover(handle_rejection);

        warp::serve(routes).run(bind).await;
    });

    Ok(())
}

fn load_csv<T, P>(input_path: P) -> Vec<T>
    where T: for<'de> Deserialize<'de>,
          P: AsRef<Path>
{
    assert!(input_path.as_ref().exists(), "path does not exist: {}", input_path.as_ref().display());
    let bytes = std::fs::read(input_path).unwrap();
    let mut rdr = csv::Reader::from_reader(&bytes[..]);
    let mut out = Vec::new();
    for row in rdr.deserialize() {
        let row = row.unwrap();
        out.push(row);
    }
    out
}

fn load_private_keys<P: AsRef<Path>>(input_path: P) -> Vec<LocalUserData> {
    load_csv(input_path)
}

fn list_workouts_request(
    users_csv_path: &Path,
    user_id: Option<Uuid>,
    email: Option<String>,
    start: Option<NaiveDate>,
    end: Option<NaiveDate>,
    limit: Option<usize>,
    curl: bool,
    host: String,
    addr: String,
) {
    let mut keys = load_private_keys(users_csv_path);
    let user_id = user_id.unwrap_or_else(|| {
        if let Some(email) = email {
            keys.iter().find(|x| x.email == email).unwrap().user_id
        } else {
            let mut rng = thread_rng();
            keys.shuffle(&mut rng);
            keys[0].user_id
        }
    });
    let encoded_key = keys.iter().find(|x| x.user_id == user_id)
        .expect("key not found in users_csv_path")
        .private_key
        .as_str();
    let key = as_priv_key(base64::decode(encoded_key).unwrap());
    let req = fitbod::api::ListWorkoutsRequest {
        user_id,
        start: start.map(|dt| Utc.from_local_date(&dt).unwrap().and_hms(0, 0, 0)),
        end: end.map(|dt| Utc.from_local_date(&dt).unwrap().and_hms(0, 0, 0)),
        limit,
    };
    example_api_req("/api/v1/workouts/list", &req, &key, &host, curl, &addr);
}

fn new_workouts_request(
    users_csv_path: &Path,
    user_id: Option<Uuid>,
    email: Option<String>,
    dt: NaiveDate,
    duration: u32,
    curl: bool,
    host: String,
    addr: String,
) {
    let mut keys = load_private_keys(users_csv_path);
    let user_id = user_id.unwrap_or_else(|| {
        if let Some(email) = email {
            keys.iter().find(|x| x.email == email).unwrap().user_id
        } else {
            let mut rng = thread_rng();
            keys.shuffle(&mut rng);
            keys[0].user_id
        }
    });
    let encoded_key = keys.iter().find(|x| x.user_id == user_id)
        .expect("key not found in users_csv_path")
        .private_key
        .as_str();
    let key = as_priv_key(base64::decode(encoded_key).unwrap());
    let start_time: DateTime<Utc> = Utc.from_local_date(&dt)
        .unwrap()
        .and_hms(14, 30, 0); // 6:30am PST
    let end_time = start_time + chrono::Duration::minutes(duration as i64);
    let workout = fitbod::Workout {
        user_id,
        workout_id: Uuid::new_v4(),
        start_time,
        end_time,
    };
    let req = fitbod::api::NewWorkoutsRequest {
        user_id,
        items: vec![workout],
    };
    example_api_req("/api/v1/workouts/new", &req, &key, &host, curl, &addr);
}

fn as_priv_key<T: AsRef<[u8]>>(bytes: T) -> fitbod::auth::PrivateKey {
    bytes.as_ref().try_into().unwrap()
}

fn example_api_req<T>(path: &str, req: &T, key: &fitbod::auth::PrivateKey, host: &str, curl: bool, addr: &str)
    where T: Serialize
{
    let req_json = serde_json::to_string(req).unwrap();
    let timestamp = Utc::now().timestamp();
    let sig = fitbod::auth::sign_request(timestamp, &req_json, &key);
    let timestamp_str = timestamp.to_string();

    if curl {
        println!("curl -H 'x-fitbod-access-signature: {sig}' -H 'x-fitbod-access-timestamp: {ts}' --data '{d}' {a}{p}",
            sig = sig,
            ts = timestamp_str,
            d = req_json,
            a = addr,
            p = path,
        );
    } else {
        const API_REQUEST_TEMPLATE: &str = include_str!("../static/api-request.tera");
        let mut tera = tera::Tera::default();
        tera.add_raw_template("api-request", API_REQUEST_TEMPLATE).unwrap();
        let mut ctx = tera::Context::new();
        ctx.insert("path", path);
        ctx.insert("body", &req_json);
        ctx.insert("sig", &sig);
        ctx.insert("host", &host);
        ctx.insert("timestamp", &timestamp_str);
        let http_req = tera.render("api-request", &ctx).unwrap();
        print!("{}", http_req);
    }
}

fn main() {
    dotenv::dotenv().ok();
    let db_url = std::env::var("DATABASE_URL").expect("DATABASE_URL env var required");
    assert_ne!(&db_url[..], "", "DATABASE_URL env var required");

    match Opt::from_args() {
        Opt::Run { bind } => {
            run(&db_url, bind).unwrap()
        }

        Opt::ListWorkoutsRequest {
            users_csv_path, user_id, email, start, end,
            limit, host, curl, connect,
        } => {
            assert!(users_csv_path.exists(), "path does not exist: {}", users_csv_path.display());
            list_workouts_request(
                &users_csv_path, user_id, email, start,
                end, limit, curl, host, connect,
            );
        }

        Opt::NewWorkoutsRequest {
            users_csv_path, user_id, email, date: dt, duration,
            host, curl, connect,
        } => {
            assert!(users_csv_path.exists(), "path does not exist: {}", users_csv_path.display());
            new_workouts_request(
                &users_csv_path, user_id, email, dt,
                duration, curl, host, connect,
            );
        }
    }
}
