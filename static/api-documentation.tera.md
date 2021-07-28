# fitbod api

*Generated {{current_time}}*

## Overview

This repo contains code for a JSON-based api server that responds primarily to two endpoints:

- `/api/v1/workouts/new`
- `/api/v1/workouts/list`

Code is in Rust, using the [warp](https://github.com/seanmonstar/warp) web framework (uses tokio async runtime under the hood).

#### Significant departures from instructions and other design notes

- API requests are signed for authentication (See "Authentication" section). For debugging purposes including `x-fitbod-god-mode` header will skip authenticating request signatures
- Schema stores `start_time` and `end_time` of workouts, not durations as in the `workout.csv` example file
- Users are assigned a UUID `user_id` and this is the primary means of identifying them in requests (not email)
- Architecture can be described as application + cache in one layer. Data is retrieved from db once and kept in memory, subsequent requests fetch from RAM. See #Design section for discussion.

## Usage

#### How to generate this document

```console
$ cargo run --bin generate-api-docs
```

`generate-api-docs` generates JSON and other examples and renders a template (`static/api-documentation.tera.md`)
using those outputs.

#### How to run the tests

```console
cargo test
```

#### How to build the server

```console
cargo build --bin fitbod-server --release
```

#### `fitbod-server --help`

for cli menu, first build server via `cargo build --bin fitbod-server --release`.

```console
$ ./target/release/fitbod-server --help
{{ fitbod_server_main_help }}
```
`fitbod-server run`:

```console
$ ./target/release/fitbod-server run --help
{{ fitbod_server_main_help }}
```

#### How to generate signed example api requests

`fitbod-server list-workouts-request`:

```console
$ ./target/release/fitbod-server list-workouts-request --help
{{ fitbod_server_list_req_help }}
```

Output with default params:

```console
$ ./target/release/fitbod-server list-workouts-request
{{ fitbod_server_list_req_http }}
```

`--curl` mode:

```console
$ ./target/release/fitbod-server list-workouts-request --curl
{{ fitbod_server_list_req_curl }}
```

`eval`ing `--curl` mode output:

Note: server must be running for this to work.

```console
$ eval "$(./target/release/fitbod-server list-workouts-request --curl) -s" | python3 -m json.tool
```

#### Justfile

project includes a [justfile](https://github.com/casey/just) with additional functionality:

```console
just --list
```

## Api Endpoints

#### HTTP Request: `POST /api/{{api_version}}/workouts/new`

Save one or more new workouts.

Requests to this endpoint are idempotent, so long as the `workout_id` field remains consistent across
multiple requests. Distinct `workout_id` values will result in multiple workouts saved.

**JSON Request Body Example:**

```json
{{new_workout_request_json}}
```

A successful request will return an empty `204 No Content` response from the server.

Failed request will return either `400` or `500` status code with short message describing error.

#### HTTP Request: `POST /api/{{api_version}}/workouts/list`

Retrieve a list of most recent workouts, with optional filter parameters.

- specifying `start` will return only workouts that occured at or after `start`
- specifying `end` will return only workouts that occured before `end`
- specifying `limit` will return only the last (most recent) *n* entries
- for `start` and `end` parameters, datetimes should be represented as strings in RFC3339
  format (e.g. "2021-07-23T05:58:44.867020774Z")

**JSON Request Body Example:**

```json
{{ list_req_json }}
```

Optional fields: `start`, `end`, `limit`:

```json
{{ list_req_opt_json }}
```

Optional fields may also be omitted:

```json
{{ only_user_id_json }}
```

**JSON Response Body Example:**

```json
{{ list_resp_json }}
```

## Authentication

The authentication process used here is realistic but does not contain all of the component parts that would be required.

#### Authentication steps assumed to be in place

- Client generates a ed25519 (private key, public key) pair, and stores its private key on the mobile device
- Client negotiates registering with server, sending public key, server stores entry in users table connecting
  `user_id` uuid to public key

#### Authentication steps included in this codebase

- Using cryptographic key stored on mobile device, client signs api requests 
- Server stores public key for each user, and verifies signatures of signed api requests

#### Signing a Request

Signature is generated from a unix timestamp in decimal (i.e. string) form combined with the request body (just
the body, does not include HTTP headers).

Both timestamp and base64-encoded signature should be included as HTTP headers included with the request.

Signature should be included as `{{ sig_header }}` HTTP header in the request, and the timestamp used should be included
as `{{ timestamp_header }}` HTTP header:

```
POST /api/{{api_version}}/workouts/list HTTP/1.1
host: fitbod.me
content-type: application/json
content-length: 50
{{ sig_header }}: XoWLlSwjjApTAbSYfK85w0ljbfKlNP7Chb/MsWUMnBXU3sT3JtHALzfc0h9e3DElYejutmXrLiR54lz3FJgfCQ==
{{ timestamp_header }}: 1627062582

{"user_id":"3a2cbc79-00e5-4598-a5b2-74c5059724af"}
```

Rust example of signing a request:

```rust
let priv_key_encoded = "jCNLYN8zGyiVM7omRHGlY1iyJuvAZBWZGuN+9TjaWJTSzZ3oEvXq7QNHTwwD785/rBnmRCPkl2D68lRyvBWHUg==";
let priv_key = base64::decode(priv_key_encoded.as_bytes()).unwrap();
assert_eq!(priv_key.len(), 64);

let unix_timestamp = "1627062582";
let request_body = r#"{"user_id":"3a2cbc79-00e5-4598-a5b2-74c5059724af"}"#;

let signature_contents = format!("{}{}", unix_timestamp, request_body);
let sig = crypto::ed25519::signature(signature_contents.as_bytes(), &priv_key[..]);
let sig_encoded = base64::encode(&sig[..]);
let sig_header = format!("{{ sig_header}}: {}", sig_encoded);
let timestamp_header = format!("{{ timestamp_header}}: {}", unix_timestamp);

// to verify sig
let pub_key = &priv_key[32..]; // this will be retrieved from users table in actual application code
assert!( crypto::ed25519::verify(signature_contents.as_bytes(), pub_key, &sig[..]) );
```

The above example is also included in code as an automated test (`check_ed25519_sig_example_in_api_docs`).

## schema

postgresql-flavored database schema:

```sql
{{ schema_sql }}
```

#### syncronization between api server and database (important)

Api server is not designed to remain perfectly in sync if database is modified by external services. The server stores (i.e. caches)
a good deal of application data in memory during its operation, updating that state as new data arrives via http requests. It does
not pull data from database on every request, only if it is needed.

New data is always written immediately to the database, so the database can be expected to be in sync with api server for reading
at all times.

To force the api server to be in sync with database, restart the api server, which will result in reading everything fresh from
database during initialization.

There is no anticipated risk of data corruption or other serious problems from modifying the database externally to the 
api server, just that the api server could respond with stale data in that case (relative to the database).
