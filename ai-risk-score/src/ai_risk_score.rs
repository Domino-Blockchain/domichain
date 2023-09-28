use bs58;
use ed25519_dalek::ed25519::signature::Signature;
use ed25519_dalek::PublicKey;
use ed25519_dalek::Verifier;
use hex::FromHex;
use lazy_static::lazy_static;
use log::{info, warn};
use serde::Deserialize;
use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use std::time::Duration;
use tokio::sync::OnceCell;
use tokio::time;

lazy_static! {
    pub static ref RISK_SCORE_MAP: Arc<RwLock<HashMap<String, HashMap<String, RewardData>>>> =
        Arc::new(RwLock::new(HashMap::new()));
}

pub static AI_REWARDS_RATE: OnceCell<f64> = OnceCell::const_new();

const QUERY_TIME_PERIOD: u64 = 60;

#[derive(Debug, Deserialize)]
struct ParsedResponse {
    risk_score: f64,
    wallet: String,
    data: String,
    public_key: String,
    signature: String,
    timeout: String,
    timestamp: String,
}

#[allow(dead_code)]
pub struct RewardData {
    pub risk_score: f64,
    pub timeout: usize,
    pub timestamp: String,
}

fn from_base58_str(s: &str) -> Vec<u8> {
    bs58::decode(s)
        .into_vec()
        .expect("Failed to decode base58 string")
}

fn verify_signature(data_hex: &str, signature_hex: &str, public_key_hex: &str) -> bool {
    let data_bytes = Vec::from_hex(data_hex).expect("Failed to decode data hex string");
    let signature_bytes =
        Vec::from_hex(signature_hex).expect("Failed to decode signature hex string");
    let public_key_bytes = from_base58_str(public_key_hex);

    let public_key =
        PublicKey::from_bytes(&public_key_bytes).expect("Failed to create PublicKey from bytes");
    let signature =
        Signature::from_bytes(&signature_bytes).expect("Failed to create Signature from bytes");

    public_key.verify(&data_bytes, &signature).is_ok()
}

pub async fn get_risk_score(url: String, ai_reward_rate: f64) {
    AI_REWARDS_RATE
        .set(ai_reward_rate)
        .expect("Failed to set AI rewards rate");
    loop {
        // Use the tokio runtime to run the asynchronous function and get the JSON response.
        let json_response = match send_http_request(&url).await {
            Ok(json) => json,
            Err(err) => {
                warn!("Error sending the request: {:?}", err);
                time::sleep(Duration::from_secs(QUERY_TIME_PERIOD)).await;
                continue;
            }
        };
        info!("Response from AI node {:?}", json_response);

        let parsed_response: Vec<ParsedResponse> = match serde_json::from_str(&json_response) {
            Ok(parsed) => parsed,
            Err(err) => {
                warn!("Error parsing JSON response: {:?}", err);
                time::sleep(Duration::from_secs(QUERY_TIME_PERIOD)).await;
                continue;
            }
        };

        info!("---AI test parsed_response {:?}", parsed_response);
        {
            let mut risk_score_map = RISK_SCORE_MAP.write().unwrap();
            for entry in &parsed_response {
                let wallet = &entry.wallet;
                let reward_account: &String = &entry.public_key;
                let risk_score = entry.risk_score;
                let timestamp: &String = &entry.timestamp;
                let timeout_str = &entry.timeout;
                let timeout = match timeout_str.parse::<usize>() {
                    Ok(parsed_timeout) => parsed_timeout,
                    Err(_) => 0,
                };

                let data_hex = &entry.data;
                let signature_hex = &entry.signature;
                let public_key_hex = &entry.public_key;
                let is_signature_valid = verify_signature(data_hex, signature_hex, public_key_hex);

                if is_signature_valid {
                    // Only proceed if the signature is valid
                    let wallet_entry = risk_score_map
                        .entry(wallet.to_owned())
                        .or_insert_with(HashMap::new);
                    let reward_data = RewardData {
                        risk_score,
                        timeout,
                        timestamp: timestamp.to_string(),
                    };
                    let rewards_entry = wallet_entry.entry(reward_account.to_owned());
                    rewards_entry.or_insert(reward_data);
                } else {
                    warn!("Invalid signature for wallet: {}", wallet);
                }
            }

            drop(risk_score_map);
        }

        time::sleep(Duration::from_secs(QUERY_TIME_PERIOD)).await;
    }
}

async fn send_http_request(url: &str) -> Result<String, reqwest::Error> {
    let client = reqwest::Client::new();
    let response = client.get(url).send().await?;
    response.text().await
}
