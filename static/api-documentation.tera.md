# fitbod api takehome

*Generated {{current_time}}*

## how to generate this document

```console
$ cargo run --bin generate-api-docs --features tera
```

`generate-api-docs` generates JSON and other examples and renders a template (`static/api-documentation.tera.md`)
using those outputs.

## schema

postgresql-flavored database schema:

```sql
{{ schema_sql }}
```

#### syncronization between api server and database (important)

Api server is not designed to remain perfectly in sync if database is modified by external services. the server will store (i.e. cache)
a good deal of application data in memory during its operation, updating that state as new data arrives via http requests. It does
not pull data from database on every request, only if it is needed.

New data is always written immediately to the database, so the database can be expected to be in sync with api server for reading
at all times.

To force the api server to be in sync with database, restart the api server, which will result in reading everything fresh from
database during initialization.

There is no anticipated risk of data corruption or other serious problems from modifying the database externally to the 
api server, just that the api server could respond with stale data in that case (relative to the database).

## authentication

Using cryptographic key embedded in mobile app, client must sign api requests to authenticate them.

(Note: this is meant to mimic real-world scenario, not sure if it lines up exactly).

#### Signing a Request

The request signature is a sha256 HMAC of the request body, using the client's secret key, encoded with standard base64.

Signature should be included as `{{ sig_header }}` HTTP header in the request:

```
POST /api/{{api_version}}/workouts/list HTTP/1.1
host: fitbod.me
content-type: application/json
{{ sig_header }}: Fn7nQsY3UqVKVr1kL7O+yP7J7WSM660oaNbSq42Vy7A=
content-length: {{ list_req_json | length }}

{{ list_req_json }}
```

Rust example of signing a request:

```rust
use crypto::hmac::Hmac;
use crypto::sha2::Sha256;
use crypto::mac::Mac;

let secret = "6KQ1CMZGFP84mJoip2crsGw5HpBhctnQ6Zkpj4/pVEqx/enTKvvwjpp57Nq7JS9gqjxyM1PtXcEHJxC0gag+dA==";
let secret_decoded = base64::decode_config(secret.as_bytes(), base64::STANDARD).unwrap();
let mut hmac = Hmac::new(Sha256::new(), &secret_decoded);
let request_body = r#"{"user_id":"3a2cbc79-00e5-4598-a5b2-74c5059724af","kind":"ping"}"#;
let mut buf = [0u8; 1024];
let sig_length = crate::sign_request(request_body.as_bytes(), &mut hmac, &mut buf[..]);
let sig = &buf[..sig_length]; // -> Fn7nQsY3UqVKVr1kL7O+yP7J7WSM660oaNbSq42Vy7A=
```

The body of `sign_request` is:

```rust
pub fn sign_request(body: &[u8], hmac: &mut Hmac<Sha256>, buf: &mut [u8]) -> usize {
    hmac.reset();
    hmac.input(body);
    base64::encode_config_slice(hmac.result().code(), base64::STANDARD, buf)
}
```

Note: the "secret" column in the "users" table of the database stores the key as raw bytes. In the example below, the
initial representation of `secret` is base64-encoded so it can be represented as a string and displayed. Decoding from
base64 to raw bytes is not required when using the raw bytes retrieved from the "users" table.

Javascript example of signing a request:

```javascript
var crypto = require('crypto');

var body = JSON.stringify({
    user_id: '3a2cbc79-00e5-4598-a5b2-74c5059724af',
    kind: 'ping',
});

var secret = "6KQ1CMZGFP84mJoip2crsGw5HpBhctnQ6Zkpj4/pVEqx/enTKvvwjpp57Nq7JS9gqjxyM1PtXcEHJxC0gag+dA==";

// decode base64-encoded secret to raw bytes
var key = Buffer.from(secret, 'base64');

// create a sha256 hmac with the secret
var hmac = crypto.createHmac('sha256', key);

// sign the require message with the hmac
// and finally base64 encode the result
var sig = hmac.update(body).digest('base64'); // -> Fn7nQsY3UqVKVr1kL7O+yP7J7WSM660oaNbSq42Vy7A=
```

## JSON representation notes

- **datetime:** datetimes are represented as strings with RFC3339 format (e.g. "2021-07-23T05:58:44.867020774Z")
- **uuid:** uuids are represented as 36-character strings (standard string representation of uuid, e.g. "7bfe4f31-bbdb-4fd5-88d9-8ba161db8e18")

## endpoints

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

