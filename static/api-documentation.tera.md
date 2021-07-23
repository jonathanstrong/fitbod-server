# fitbod api takehome

*Generated {{current_time}}*

## how to generate this document

```console
$ cargo run --bin generate-api-docs --features tera
```

`generate-api-docs` generates JSON and other examples and renders a template (`static/api-documentation.tera.md`)
using those outputs.

## authentication

Using cryptographic key embedded in mobile app, client must sign api requests to authenticate them.

(Note: this is meant to mimic real-world scenario, not sure if it lines up exactly).

#### Signing a Request

The request signature is a sha256 HMAC of the request body, using the client's secret key, encoded with standard base64.

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

**HTTP Request:** `POST /api/{{api_version}}/workouts/new`

Requests to this endpoint are idempotent, so long as the `workout_id` field remains consistent across
multiple requests. Distinct `workout_id` values will result in multiple workouts saved.

**JSON Request Body Example:**

```json
{{new_workout_request_json}}
```

**JSON Response Body Example (Success):**

Fields:
- `workout_id`: matches submitted value

```json
{{new_workout_success_resp_json}}
```

**JSON Response Body Example (Error):**

- `workout_id`: matches submitted value
- `err_code`: numeric identifier for the kind of error encountered. this number will remain
   stable across updates and other chnages to `/api/{{api_version}}` endpoints
- `msg`: a short description of the error for diagnostic purposes. this message may change
   at any time

```json
{{new_workout_err_resp_json}}
```



