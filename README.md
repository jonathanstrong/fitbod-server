# fitbod api takehome

*Generated Fri, 23 Jul 2021 22:24:40 +0000*

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

Signature should be included as `x-fitbod-access-signature` HTTP header in the request, and the timestamp used should be included
as `x-fitbod-access-timestamp` HTTP header:

```
POST /api/v1/workouts/list HTTP/1.1
host: fitbod.me
content-type: application/json
content-length: 50
x-fitbod-access-signature: XoWLlSwjjApTAbSYfK85w0ljbfKlNP7Chb/MsWUMnBXU3sT3JtHALzfc0h9e3DElYejutmXrLiR54lz3FJgfCQ==
x-fitbod-access-timestamp: 1627062582

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
let sig_header = format!("x-fitbod-access-signature: {}", sig_encoded);
let timestamp_header = format!("x-fitbod-access-timestamp: {}", unix_timestamp);

// to verify sig
let pub_key = &priv_key[32..]; // this will be retrieved from users table in actual application code
assert!( crypto::ed25519::verify(signature_contents.as_bytes(), pub_key, &sig[..]) );
```

The above example is also included in code as an automated test (`check_ed25519_sig_example_in_api_docs`).

## Api Endpoints

#### HTTP Request: `POST /api/v1/workouts/new`

Save one or more new workouts. A single new workout should be represented as a list of one item.

Requests to this endpoint are idempotent, so long as the `workout_id` field remains consistent across
multiple requests. Distinct `workout_id` values will result in multiple workouts saved.

**JSON Request Body Example:**

```json
[
  {
    "workout_id": "2d6cb117-741d-4ea3-baca-37391232e649",
    "user_id": "ec90c5c1-541a-45be-8a4c-072673823c1a",
    "start_time": "2021-07-23T22:24:40.773992610Z",
    "end_time": "2021-07-23T23:19:40.773992610Z"
  }
]
```

**JSON Response Body Example (Success):**

- `workout_id`: matches value of submitted request

```json
[
  {
    "result": "success",
    "workout_id": "2d6cb117-741d-4ea3-baca-37391232e649"
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
  "workout_id": "2d6cb117-741d-4ea3-baca-37391232e649",
  "err_code": 123,
  "msg": "short message describing error"
}
```

#### HTTP Request: `POST /api/v1/workouts/list`

Retrieve a list of most recent workouts, with optional filter parameters.

- specifying `start` will return only workouts that occured at or after `start`
- specifying `end` will return only workouts that occured before `end`
- specifying `limit` will return only the last (most recent) *n* entries
- for `start` and `end` parameters, datetimes should be represented as strings in RFC3339
  format (e.g. "2021-07-23T05:58:44.867020774Z")

**JSON Request Body Example:**

```json
{
  "user_id": "ec90c5c1-541a-45be-8a4c-072673823c1a",
  "start": "2021-07-02T22:24:40.774113822Z",
  "end": "2021-07-23T22:24:40.774116894Z",
  "limit": 10
}
```

Optional fields: `start`, `end`, `limit`:

```json
{
  "user_id": "ec90c5c1-541a-45be-8a4c-072673823c1a",
  "start": null,
  "end": null,
  "limit": null
}
```

Optional fields may also be omitted:

```json
{
  "user_id": "ec90c5c1-541a-45be-8a4c-072673823c1a"
}
```

**JSON Response Body Example:**

```json
{
  "user_id": "ec90c5c1-541a-45be-8a4c-072673823c1a",
  "n_items": 1,
  "items": [
    {
      "workout_id": "2d6cb117-741d-4ea3-baca-37391232e649",
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
  "user_id": "ec90c5c1-541a-45be-8a4c-072673823c1a"
}
```

**Response Examples:**

New workout:

```json
{
  "user_id": "ec90c5c1-541a-45be-8a4c-072673823c1a",
  "n_items": 1,
  "items": [
    {
      "event_kind": "new_workout",
      "workout_id": "2d6cb117-741d-4ea3-baca-37391232e649",
      "date": "2021-07-23",
      "duration_minutes": 55
    }
  ]
}
```

Dopamine shot (encouraging message in alerts panel):

```json
{
  "user_id": "ec90c5c1-541a-45be-8a4c-072673823c1a",
  "n_items": 1,
  "items": [
    {
      "event_kind": "dopamine_shot",
      "message": "you can do it!"
    }
  ]
}
```

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

    key         bytea NOT NULL                          -- ed25519 public key used to sign requests
                CHECK (length(key) = 32),

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

    -- "start_time" and "end_time" because "start" and "end" caused reserved keyword conflicts

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

-- useful for debugging/cli purposes
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

-- facilitate future schema upgrades
CREATE TABLE migrations (
    id          serial PRIMARY KEY,

    version     text not null,

    descr       text,

    applied     timestamp with time zone NOT NULL
                DEFAULT now()
);

-- record this query as initial migration
insert into migrations(version, descr) values (
    '1.0.0',
    'initial schema: users, workouts, and migrations tables; workout_durations view'
);

-- some dummy data for testing
insert into users (user_id, email, key) values (
    '1fe9e4f0-8cd1-46be-963a-7f51470db6af',
    'jstrong@fitmod.me',
    '\xd2cd9de812f5eaed03474f0c03efce7fac19e64423e49760faf25472bc158752' -- base64-encoded priv key in .env (FITBOD_SECRET_ACCESS_KEY)
);

insert into workouts (user_id, start_time, end_time) values (
    '1fe9e4f0-8cd1-46be-963a-7f51470db6af',
    now() + interval '15 minutes',
    now() + interval '37 minutes'
);

COMMIT;

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
