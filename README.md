# fitbod api

*Generated Wed, 28 Jul 2021 21:31:03 +0000*

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
fitbod 0.1.0
Jonathan Strong <jonathan.strong@gmail.com>
fitbod api example server

DATABASE_URL env var must be present with postgres connection info

USAGE:
    fitbod-server <SUBCOMMAND>

FLAGS:
    -h, --help       
            Prints help information

    -V, --version    
            Prints version information


SUBCOMMANDS:
    help                     Prints this message or the help of the given subcommand(s)
    list-workouts-request    print example http request for /api/v1/workouts/list endpoint to stdout
    new-workouts-request     print example http request for /api/v1/workouts/new endpoint to stdout
    run                      run the server, listening on the provided address for incoming http requests

```
`fitbod-server run`:

```console
$ ./target/release/fitbod-server run --help
fitbod 0.1.0
Jonathan Strong <jonathan.strong@gmail.com>
fitbod api example server

DATABASE_URL env var must be present with postgres connection info

USAGE:
    fitbod-server <SUBCOMMAND>

FLAGS:
    -h, --help       
            Prints help information

    -V, --version    
            Prints version information


SUBCOMMANDS:
    help                     Prints this message or the help of the given subcommand(s)
    list-workouts-request    print example http request for /api/v1/workouts/list endpoint to stdout
    new-workouts-request     print example http request for /api/v1/workouts/new endpoint to stdout
    run                      run the server, listening on the provided address for incoming http requests

```

#### How to generate signed example api requests

`fitbod-server list-workouts-request`:

```console
$ ./target/release/fitbod-server list-workouts-request --help
fitbod-server-list-workouts-request 0.1.0
print example http request for /api/v1/workouts/list endpoint to stdout

USAGE:
    fitbod-server list-workouts-request [FLAGS] [OPTIONS]

FLAGS:
        --curl       output curl command instead of http request text
    -h, --help       Prints help information
    -V, --version    Prints version information

OPTIONS:
    -c, --connect <connect>                  for --curl mode, what address to connect to to send request [default:
                                             127.0.0.1:4242]
        --email <email>                      pick user by email instead of user_id. this will search the --users-csv-
                                             path data to find the correct UUID by email
        --end <end>                          filter results by end (YYYY-MM-DD)
        --host <host>                        value of http host header [default: fitbod.jstrong.dev]
        --limit <limit>                      specify limit to request
        --start <start>                      filter results by end (YYYY-MM-DD)
        --user-id <user-id>                  defaults to a user id randomly chosen from the file
    -u, --users-csv-path <users-csv-path>     [default: var/example-users.csv]

```

Output with default params:

```console
$ ./target/release/fitbod-server list-workouts-request
POST /api/v1/workouts/list HTTP/1.1
host: fitbod.jstrong.dev 
content-type: application/json
content-length: 87
x-fitbod-access-signature: hcim2VLsRX2Mvqa9SHfgbBvyTgn37MkbLn1XXs/KS7p+ssqrlA+QvZFKmH9DlUTlIRWPrKt3DBkFcVXIfr6NAg==
x-fitbod-access-timestamp: 1627507858

{"user_id":"6cefe9d3-963e-4a24-ade2-e51b596c74dc","start":null,"end":null,"limit":null}

```

`--curl` mode:

```console
$ ./target/release/fitbod-server list-workouts-request --curl
curl -H 'x-fitbod-access-signature: ORD6SOnKHOaXAGYQ5I/Ibi1L77WFEKJKxVu3F6oC9+H6HDP75/7Hu76HCPApIAdbJBwYX1Qypuc52+jdZn9FAw==' -H 'x-fitbod-access-timestamp: 1627507858' --data '{"user_id":"afbd7a78-d85c-4f22-aaae-6174cb890233","start":null,"end":null,"limit":null}' 127.0.0.1:4242/api/v1/workouts/list

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

#### HTTP Request: `POST /api/v1/workouts/new`

Save one or more new workouts.

Requests to this endpoint are idempotent, so long as the `workout_id` field remains consistent across
multiple requests. Distinct `workout_id` values will result in multiple workouts saved.

**JSON Request Body Example:**

```json
[
  {
    "user_id": "ce24e5a9-b7fb-407f-b46e-e2f8aa4956b4",
    "items": [
      {
        "workout_id": "e3b627d7-cf01-44ed-be1d-f64425ab6a9e",
        "user_id": "ce24e5a9-b7fb-407f-b46e-e2f8aa4956b4",
        "start_time": "2021-07-28T21:31:03.822548904Z",
        "end_time": "2021-07-28T22:26:03.822548904Z"
      }
    ]
  }
]
```

A successful request will return an empty `204 No Content` response from the server.

Failed request will return either `400` or `500` status code with short message describing error.

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
  "user_id": "ce24e5a9-b7fb-407f-b46e-e2f8aa4956b4",
  "start": "2021-07-07T21:31:03.822705088Z",
  "end": "2021-07-28T21:31:03.822709180Z",
  "limit": 10
}
```

Optional fields: `start`, `end`, `limit`:

```json
{
  "user_id": "ce24e5a9-b7fb-407f-b46e-e2f8aa4956b4",
  "start": null,
  "end": null,
  "limit": null
}
```

Optional fields may also be omitted:

```json
{
  "user_id": "ce24e5a9-b7fb-407f-b46e-e2f8aa4956b4"
}
```

**JSON Response Body Example:**

```json
{
  "user_id": "ce24e5a9-b7fb-407f-b46e-e2f8aa4956b4",
  "n_items": 1,
  "items": [
    {
      "workout_id": "e3b627d7-cf01-44ed-be1d-f64425ab6a9e",
      "date": "2021-07-28",
      "duration_minutes": 55
    }
  ]
}
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
        ON DELETE CASCADE,

    CONSTRAINT user_start_uniq UNIQUE(user_id, start_time) -- prevent duplicate start_time entries for given user, application code assumes this condition
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
