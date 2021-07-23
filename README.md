# fitbod api takehome

*Generated Fri, 23 Jul 2021 08:40:25 +0000*

## how to generate this document

```console
$ cargo run --bin generate-api-docs --features tera
```

`generate-api-docs` generates JSON and other examples and renders a template (`static/api-documentation.tera.md`)
using those outputs.

## schema

postgresql-flavored database schema:

```sql
BEGIN TRANSACTION;

DROP schema public CASCADE;
CREATE schema public;

CREATE EXTENSION IF NOT EXISTS pgcrypto;                -- enables gen_random_uuid() function

CREATE TABLE users (
    user_id     uuid NOT NULL UNIQUE
                DEFAULT gen_random_uuid()
                PRIMARY KEY,

    email       text NOT NULL UNIQUE
                CHECK (length(email) > 0),

    secret      bytea NOT NULL                          -- used to sign requests
                DEFAULT gen_random_bytes(64)
                CHECK (length(secret) > 0),

    created     timestamp with time zone NOT NULL
                DEFAULT now()
);

CREATE INDEX users_email ON users USING hash (
    email
);

CREATE TABLE workouts (
    workout_id  uuid NOT NULL UNIQUE
                DEFAULT gen_random_uuid()
                PRIMARY KEY,

    user_id     uuid NOT NULL,

    start_time  timestamp with time zone NOT NULL,

    end_time    timestamp with time zone NOT NULL,

    CONSTRAINT workouts_user_fkey FOREIGN KEY (user_id)
        REFERENCES users (user_id)
        ON DELETE CASCADE
);

CREATE INDEX workouts_start_time ON workouts USING btree (
    start_time  DESC
);

CREATE INDEX workouts_user_start_time ON workouts USING btree (
    user_id,
    start_time  DESC
);

CREATE VIEW workout_durations AS
    SELECT
        u.email,
        u.user_id,
        w.workout_id,
        date(w.start_time) as dt,
        date_part('minutes', w.end_time - w.start_time) as duration_minutes
    FROM users u
    INNER JOIN workouts w ON u.user_id = w.user_id
;

CREATE TABLE migrations (
    id serial PRIMARY KEY,
    version text not null,
    descr text,
    applied timestamp with time zone NOT NULL DEFAULT now()
);

-- record this query as initial migration
insert into migrations(version, descr) values (
    '1.0.0',
    'initial schema: declares tables users, workouts, migrations, and workout_durations view'
);

-- some dummy data for testing
insert into users (user_id, email) values ('1fe9e4f0-8cd1-46be-963a-7f51470db6af', 'jstrong@fitmod.me');
insert into workouts (user_id, start_time, end_time) values (
    '1fe9e4f0-8cd1-46be-963a-7f51470db6af',
    now() + interval '15 minutes',
    now() + interval '37 minutes'
);

COMMIT;

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

Signature should be included as `x-fitbod-signature` HTTP header in the request:

```
POST /api/v1/workouts/list HTTP/1.1
host: fitbod.me
content-type: application/json
x-fitbod-signature: Fn7nQsY3UqVKVr1kL7O+yP7J7WSM660oaNbSq42Vy7A=
content-length: 158

{
  "user_id": "60be25ee-0a0d-4d9d-abd8-0d9248c8510f",
  "start": "2021-07-02T08:40:25.839199158Z",
  "end": "2021-07-23T08:40:25.839202679Z",
  "limit": 10
}
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

#### HTTP Request: `POST /api/v1/workouts/new`

Save one or more new workouts. A single new workout should be represented as a list of one item.

Requests to this endpoint are idempotent, so long as the `workout_id` field remains consistent across
multiple requests. Distinct `workout_id` values will result in multiple workouts saved.

**JSON Request Body Example:**

```json
[
  {
    "workout_id": "b42e74a1-b391-41ed-84e9-3f996d04fa5c",
    "user_id": "60be25ee-0a0d-4d9d-abd8-0d9248c8510f",
    "start_time": "2021-07-23T08:40:25.839062885Z",
    "end_time": "2021-07-23T09:35:25.839062885Z"
  }
]
```

**JSON Response Body Example (Success):**

- `workout_id`: matches value of submitted request

```json
[
  {
    "result": "success",
    "workout_id": "b42e74a1-b391-41ed-84e9-3f996d04fa5c"
  }
]
```

**JSON Response Body Example (Error):**

- `workout_id`: matches value of submitted request
- `err_code`: numeric identifier for the kind of error encountered. this number will remain
   stable across updates and other chnages to `/api/v1` endpoints
- `msg`: a short description of the error for diagnostic purposes. this message may change
   at any time

```json
{
  "result": "error",
  "workout_id": "b42e74a1-b391-41ed-84e9-3f996d04fa5c",
  "err_code": 123,
  "msg": "short message describing error"
}
```

#### HTTP Request: `POST /api/v1/workouts/list`

Retrieve a list of most recent workouts, with optional filter parameters.

- specifying `start` will return only workouts that occured at or after `start`
- specifying `end` will return only workouts that occured before `end`
- specifying `limit` will return only the last (most recent) *n* entries

**JSON Request Body Example:**

```json
{
  "user_id": "60be25ee-0a0d-4d9d-abd8-0d9248c8510f",
  "start": "2021-07-02T08:40:25.839199158Z",
  "end": "2021-07-23T08:40:25.839202679Z",
  "limit": 10
}
```

Optional fields: `start`, `end`, `limit`:

```json
{
  "user_id": "60be25ee-0a0d-4d9d-abd8-0d9248c8510f",
  "start": null,
  "end": null,
  "limit": null
}
```

Optional fields may also be omitted:

```json
{
  "user_id": "60be25ee-0a0d-4d9d-abd8-0d9248c8510f"
}
```

**JSON Response Body Example:**

```json
{
  "user_id": "60be25ee-0a0d-4d9d-abd8-0d9248c8510f",
  "n_items": 1,
  "items": [
    {
      "workout_id": "b42e74a1-b391-41ed-84e9-3f996d04fa5c",
      "date": "2021-07-23",
      "duration_minutes": 55
    }
  ]
}
```

#### HTTP/Websocket Request: `POST /api/v1/events`

Subscribe to a websocket feed of events pertaining to a user.

Server will emit new events, but will not read or listen for any data sent from the client.

A websocket frame may contain multiple events. New types of events may be added, but the structure
of existing messages (as determined by `event_kind` field) will remain stable.

To unsubscribe, simply close the websocket connection.

**JSON Request Body Example:**

```json
{
  "user_id": "60be25ee-0a0d-4d9d-abd8-0d9248c8510f"
}
```

**Response Examples:**

New workout:

```json
{
  "user_id": "60be25ee-0a0d-4d9d-abd8-0d9248c8510f",
  "n_items": 1,
  "items": [
    {
      "event_kind": "new_workout",
      "workout_id": "b42e74a1-b391-41ed-84e9-3f996d04fa5c",
      "date": "2021-07-23",
      "duration_minutes": 55
    }
  ]
}
```

Dopamine shot (encouraging message in alerts panel):

```json
{
  "user_id": "60be25ee-0a0d-4d9d-abd8-0d9248c8510f",
  "n_items": 1,
  "items": [
    {
      "event_kind": "dopamine_shot",
      "message": "you can do it!"
    }
  ]
}
```

