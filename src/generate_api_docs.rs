use std::io;
use chrono::prelude::*;
use uuid::Uuid;
use crypto::hmac::Hmac;
use crypto::sha2::Sha256;
use crypto::mac::Mac;
use fitbod::*;

const API_DOCS_TEMPLATE: &str = include_str!("../static/api-documentation.tera.md");
const OUTPUT_PATH: &str = "./README.md";

fn main() -> Result<(), io::Error> {
    let mut tera = tera::Tera::default();
    tera.add_raw_template("api-documentation.md", API_DOCS_TEMPLATE).unwrap();
    let mut ctx = tera::Context::new();
    let current_time = Utc::now().to_rfc2822();
    ctx.insert("current_time", &current_time);
    ctx.insert("api_version", API_VERSION);

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

    let new_workout_req_json = serde_json::to_string_pretty(&new_workout_req).unwrap();
    ctx.insert("new_workout_request_json", &new_workout_req_json);

    let new_workout_success_resp = NewWorkoutResponse::Success { workout_id };
    let new_workout_success_resp_json = serde_json::to_string_pretty(&new_workout_success_resp).unwrap();
    ctx.insert("new_workout_success_resp_json", &new_workout_success_resp_json);

    let new_workout_err_resp = NewWorkoutResponse::Error { workout_id, err_code: 123, msg: "short description message".to_string() };
    let new_workout_err_resp_json = serde_json::to_string_pretty(&new_workout_err_resp).unwrap();
    ctx.insert("new_workout_err_resp_json", &new_workout_err_resp_json);

    // //let example_secret: &str = "private key 64 bytes in length";
    // let example_secret: String = fitbod::gen_secret_base64();
    // let example_secret_decoded = base64::decode_config(example_secret.as_bytes(), base64::STANDARD).unwrap();
    // let mut hmac = Hmac::new(Sha256::new(), &example_secret_decoded[..]);
    // let request_body = format!("{{\"user_id\": \"{}\", \"kind\": \"ping\"}}", user_id);
    // let mut buf = [0u8; 1024];
    // let n_bytes = sign_request(request_body.as_bytes(), &mut hmac, &mut buf[..]);
    // let sig = std::str::from_utf8(&buf[..n_bytes]).unwrap();
    // ctx.insert("sig_example_secret", &example_secret);
    // ctx.insert("sig_example_request_body", &request_body);
    // ctx.insert("sig_example_output", &sig);


    let api_docs = tera.render("api-documentation.md", &ctx).unwrap();
    std::fs::write(OUTPUT_PATH, &api_docs)?;
    Ok(())
}
