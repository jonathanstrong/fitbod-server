use std::io;
use serde::Serialize;
use chrono::prelude::*;
use uuid::Uuid;
use fitbod::*;

const API_DOCS_TEMPLATE: &str = include_str!("../static/api-documentation.tera.md");
const OUTPUT_PATH: &str = "./README.md";
const SCHEMA_SQL: &str = include_str!("../sql/schema-postgresql.sql");

fn main() -> Result<(), io::Error> {
    let mut tera = tera::Tera::default();
    tera.add_raw_template("api-documentation.md", API_DOCS_TEMPLATE).unwrap();
    let mut ctx = tera::Context::new();
    let current_time = Utc::now().to_rfc2822();
    ctx.insert("current_time", &current_time);
    ctx.insert("api_version", API_VERSION);
    ctx.insert("sig_header", SIG_HEADER);
    ctx.insert("timestamp_header", TIMESTAMP_HEADER);
    ctx.insert("schema_sql", SCHEMA_SQL);

    let user_id = Uuid::new_v4();
    let workout_id = Uuid::new_v4();
    ctx.insert("user_id", &user_id);
    ctx.insert("workout_id", &workout_id);
    let start_time = Utc::now();
    let end_time = start_time + chrono::Duration::minutes(55);

    let new_workout_req = NewWorkoutRequest {
        user_id,
        workout_id,
        start_time,
        end_time,
    };

    let new_workout_req_json = serde_json::to_string_pretty(&[new_workout_req]).unwrap();
    ctx.insert("new_workout_request_json", &new_workout_req_json);

    let new_workout_success_resp = NewWorkoutResponse::Success { workout_id };
    let new_workout_success_resp_json = serde_json::to_string_pretty(&[new_workout_success_resp]).unwrap();
    ctx.insert("new_workout_success_resp_json", &new_workout_success_resp_json);

    let new_workout_err_resp = NewWorkoutResponse::Error { workout_id, err_code: 123, msg: "short message describing error".to_string() };
    let new_workout_err_resp_json = serde_json::to_string_pretty(&new_workout_err_resp).unwrap();
    ctx.insert("new_workout_err_resp_json", &new_workout_err_resp_json);

    let list_req = ListWorkoutsRequest {
        user_id,
        start: Some(Utc::now() - chrono::Duration::hours(24 * 7 * 3)),
        end: Some(Utc::now()),
        limit: Some(10),
    };
    let list_req_json = serde_json::to_string_pretty(&list_req).unwrap();
    ctx.insert("list_req_json", &list_req_json);

    let list_req_opt = ListWorkoutsRequest {
        user_id,
        start: None,
        end: None,
        limit: None,
    };
    let list_req_opt_json = serde_json::to_string_pretty(&list_req_opt).unwrap();
    ctx.insert("list_req_opt_json", &list_req_opt_json);

    #[derive(Serialize)]
    struct OnlyUserId {
        user_id: Uuid,
    }
    let only_user_id = OnlyUserId { user_id };
    let only_user_id_json = serde_json::to_string_pretty(&only_user_id).unwrap();
    assert_eq!(serde_json::from_str::<ListWorkoutsRequest>(&only_user_id_json).unwrap().user_id, list_req.user_id);
    ctx.insert("only_user_id_json", &only_user_id_json);

    let list_item = ListWorkoutsItem { 
        workout_id,
        date: start_time.date().naive_local(),
        duration_minutes: (end_time - start_time).num_minutes() as u32,
    };

    let list_resp = ListWorkoutsResponse {
        user_id,
        n_items: 1,
        items: vec![list_item.clone()],
    };
    let list_resp_json = serde_json::to_string_pretty(&list_resp).unwrap();
    ctx.insert("list_resp_json", &list_resp_json);

    let new_workout = NewEvents {
        user_id,
        n_items: 1,
        items: vec![Event::NewWorkout(list_item.clone())],
    };
    let new_workout_json = serde_json::to_string_pretty(&new_workout).unwrap();
    ctx.insert("new_workout_json", &new_workout_json);

    let dopamine_shot = NewEvents {
        user_id,
        n_items: 1,
        items: vec![Event::DopamineShot { message: "you can do it!".to_string() }],
    };
    let dopamine_shot_json = serde_json::to_string_pretty(&dopamine_shot).unwrap();
    ctx.insert("dopamine_shot_json", &dopamine_shot_json);

    let api_docs = tera.render("api-documentation.md", &ctx).unwrap();
    std::fs::write(OUTPUT_PATH, &api_docs)?;
    Ok(())
}
