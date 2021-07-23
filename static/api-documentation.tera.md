# fitbod api takehome

*Generated {{current_time}}*

## how to generate this document

```console
$ cargo run --bin generate-api-docs --features tera
```

`generate-api-docs` generates JSON and other examples and renders a template (`static/api-documentation.tera.md`)
using those outputs.

## authentication

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

## Api Endpoints

#### HTTP Request: `POST /api/{{api_version}}/workouts/new`

Save one or more new workouts. A single new workout should be represented as a list of one item.

Requests to this endpoint are idempotent, so long as the `workout_id` field remains consistent across
multiple requests. Distinct `workout_id` values will result in multiple workouts saved.

**JSON Request Body Example:**

```json
{{new_workout_request_json}}
```

**JSON Response Body Example (Success):**

- `workout_id`: matches value of submitted request

```json
{{new_workout_success_resp_json}}
```

**JSON Response Body Example (Error):**

- `workout_id`: matches value of submitted request
- `err_code`: numeric identifier for the kind of error encountered. this number will remain
   stable across updates and other chnages to `/api/{{api_version}}` endpoints
- `msg`: a short description of the error for diagnostic purposes. this message may change
   at any time

```json
{{new_workout_err_resp_json}}
```

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

#### HTTP/Websocket Request: `POST /api/{{api_version}}/events`

Subscribe to a websocket feed of events pertaining to a user.

Server will emit new events, but will not read or listen for any data sent from the client.

A websocket frame may contain multiple events. New types of events may be added, but the structure
of existing messages (as determined by `event_kind` field) will remain stable.

To unsubscribe, simply close the websocket connection.

**JSON Request Body Example:**

```json
{{ only_user_id_json }}
```

**Response Examples:**

New workout:

```json
{{new_workout_json}}
```

Dopamine shot (encouraging message in alerts panel):

```json
{{dopamine_shot_json}}
```

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
