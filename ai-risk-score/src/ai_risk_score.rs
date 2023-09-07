use lazy_static::lazy_static;
use log::{info, warn};
use tokio::sync::OnceCell;
use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use serde::Deserialize;
use std::time::Duration;
use tokio::time;

lazy_static! {
    pub static ref RISK_SCORE_MAP: Arc<RwLock<HashMap<String, HashMap<String, f64>>>> =
        Arc::new(RwLock::new(HashMap::new()));
}

pub static AI_REWARDS_RATE: OnceCell<f64> = OnceCell::const_new();

const QUERY_TIME_PERIOD:u64 = 600;

#[derive(Debug, Deserialize)]
struct ParsedResponse {
    risk_score: f64,
    wallet: String,
    data: String,
    public_key: String,
    signature:String,
}

pub async fn get_risk_score(url:String, ai_reward_rate:f64) {

    AI_REWARDS_RATE.set(ai_reward_rate).expect("Failed to set AI rewards rate");
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
        for entry in parsed_response {
            let wallet = entry.wallet;
            let reward_account: String = entry.public_key;
            let risk_score = entry.risk_score;
            //println!("Wallet: {:?}, Risk Score: {:?}", wallet, risk_score);
            let wallet_entry = risk_score_map.entry(wallet).or_insert_with(HashMap::new);
            let rewards_entry = wallet_entry.entry(reward_account);
            rewards_entry.or_insert(risk_score);
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
