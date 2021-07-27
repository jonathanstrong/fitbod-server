use rand::prelude::*;

pub type PrivateKey     = [u8; 64];
pub type PublicKey      = [u8; 32];

pub fn gen_keypair() -> (PrivateKey, PublicKey) {
    let mut seed = [0u8; 32];
    let mut rng = thread_rng();
    rng.fill(&mut seed[..]);
    crypto::ed25519::keypair(&seed[..])
}

/// check request signature using provided public key
pub fn verify_request(sig: &[u8], timestamp: &[u8], body: &[u8], pub_key: &[u8], buf: &mut Vec<u8>) -> bool {
    buf.clear();
    buf.extend_from_slice(timestamp);
    buf.extend_from_slice(body);
    let n = buf.len();
    if let Err(_) = base64::decode_config_buf(sig, base64::STANDARD, buf) {
        return false
    }
    let msg = &buf[..n];
    let decoded_sig = &buf[n..];

    debug_assert_eq!(msg.len(), timestamp.len() + body.len());
    debug_assert_eq!(pub_key.len(), 32);
    debug_assert_eq!(decoded_sig.len(), 64);

    crypto::ed25519::verify(msg, pub_key, decoded_sig)
}

/// generate base64-encoded signature using provided private key
pub fn sign_request(unix_timestamp: i64, request_body: &str, priv_key: &PrivateKey) -> String {
    let signature_contents = format!("{}{}", unix_timestamp, request_body);
    let sig = crypto::ed25519::signature(signature_contents.as_bytes(), &priv_key[..]);
    let encoded_sig = base64::encode(&sig[..]);
    encoded_sig
}

#[allow(unused)]
#[cfg(test)]
mod tests {
    use super::*;
    use rand::prelude::*;
    use chrono::prelude::*;
    use crate::api::{SIG_HEADER, TIMESTAMP_HEADER};

    #[test]
    fn verify_output_of_sign_request() {
        let (priv_key, pub_key) = gen_keypair();
        let ts = Utc::now().timestamp();
        let body = "hello world";
        let sig = sign_request(ts, body, &priv_key);
        let ts_str = ts.to_string();
        let mut buf = Vec::new();
        assert_eq!(verify_request(sig.as_bytes(), ts_str.as_bytes(), body.as_bytes(), &pub_key[..], &mut buf), true);
    }

    #[test]
    fn check_ed25519_sig_example_in_api_docs() {
        let priv_key_encoded = "jCNLYN8zGyiVM7omRHGlY1iyJuvAZBWZGuN+9TjaWJTSzZ3oEvXq7QNHTwwD785/rBnmRCPkl2D68lRyvBWHUg==";
        let priv_key = base64::decode(priv_key_encoded.as_bytes()).unwrap();
        assert_eq!(priv_key.len(), 64);
        let unix_timestamp = "1627062582";
        //let request_body = r#"{"user_id":"3a2cbc79-00e5-4598-a5b2-74c5059724af"}"#;
        let request_body = r#"{"user_id":"1fe9e4f0-8cd1-46be-963a-7f51470db6af"}"#;
        assert_eq!(request_body.len(), 50);
        let signature_contents = format!("{}{}", unix_timestamp, request_body);
        let sig = crypto::ed25519::signature(signature_contents.as_bytes(), &priv_key[..]);
        let encoded_sig = base64::encode(&sig[..]);
        let sig_header = format!("{}: {}", SIG_HEADER, encoded_sig);
        let timestamp_header = format!("{}: {}", TIMESTAMP_HEADER, unix_timestamp);
        let pub_key = &priv_key[32..]; // this will be retrieved from users table in actual application code
        println!("{}", sig_header);
        println!("{}", request_body);
        assert!( crypto::ed25519::verify(signature_contents.as_bytes(), pub_key, &sig[..]) );
        let mut buf = Vec::new();
        assert!( verify_request(encoded_sig.as_bytes(), unix_timestamp.as_bytes(), request_body.as_bytes(), pub_key, &mut buf) );
    }

    #[test]
    fn check_how_ed25519_pub_priv_keypair_works() {
        let mut seed = [0u8; 32];
        let mut rng = thread_rng();
        rng.fill(&mut seed[..]);
        let (priv_key, pub_key) = crypto::ed25519::keypair(&seed[..]);
        let exch_key = crypto::ed25519::exchange(&pub_key[..], &priv_key[..]);
        let msg = r#"{"user_id":"3a2cbc79-00e5-4598-a5b2-74c5059724af","kind":"ping"}"#;
        let sig = crypto::ed25519::signature(msg.as_bytes(), &priv_key[..]);
        assert_eq!(crypto::ed25519::verify(msg.as_bytes(), &pub_key[..], &sig[..]), true);
    }
}
