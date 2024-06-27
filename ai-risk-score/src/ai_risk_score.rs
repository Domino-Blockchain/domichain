use bs58;
use chrono::DateTime;
use chrono::Utc;
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
    risk_score: String,
    wallet: String,
    data: String,
    public_key: String,
    signature: String,
    timeout: String,
    timestamp: DateTime<Utc>,
}

#[derive(Debug)]
#[allow(dead_code)]
pub struct RewardData {
    pub risk_score: f64,
    pub count: u64
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

pub fn update_risk_scores(wallet: String, reward_account: String, risk_score: f64) {

    if risk_score > 5.0 {
        // Max score accepted is 5
        return;
    }

    let mut risk_score_map = RISK_SCORE_MAP.write().unwrap();

    let wallet_entry = risk_score_map.entry(wallet.clone()).or_insert_with(HashMap::new);

    let reward_data = RewardData {
        risk_score,
        count: 30, 
    };

    // Update the entry only if there is no present count
    match wallet_entry.get(&reward_account) {
        Some(existing_reward_data) => {
            if existing_reward_data.count > 0 {
                // If count is over 0, do not update.
                //println!("Count is greater than 0 for wallet: {}, no update performed.", wallet);
            } else {
                // If count is 0, update the entry.
                wallet_entry.insert(reward_account, reward_data);
                //println!("Count is 0 for wallet: {}, entry updated.", wallet);
            }
        },
        None => {
            // If the reward account is not in the map, insert new entry.
            wallet_entry.insert(reward_account, reward_data);
            //println!("Inserted new entry for wallet: {}.", wallet);
        },
    }
}
