use lazy_static::lazy_static;
use std::collections::HashMap;
use std::sync::{Arc, Mutex, RwLock};
use serde::{Deserialize, Serialize};
use std::time::Duration;
use tokio::time;

lazy_static! {
    pub static ref RISK_SCORE_MAP: Arc<RwLock<HashMap<String, u32>>> = Arc::new(RwLock::new(HashMap::new()));
}

#[derive(Debug, Deserialize)]
struct ParsedResponse {
    ip_address: String,
    risk_score: f64,
    wallet: String,
}

pub async fn get_risk_score() {
    let url = "http://127.0.0.1:5000/retrieve_risk_score_by_timestamp?time=600";

    println!("AI Risk Score Test ");
    
    loop {
        // Use the tokio runtime to run the asynchronous function and get the JSON response.
        let json_response = match send_http_request(url).await {
            Ok(json) => json,
            Err(err) => {
                println!("Error sending the request: {:?}", err);
                time::sleep(Duration::from_secs(60)).await;
                continue;
            }
        };
        println!("Response from AI node {:?}", json_response);
        
        let parsed_response: Vec<ParsedResponse> = match serde_json::from_str(&json_response) {
            Ok(parsed) => parsed,
            Err(err) => {
                println!("Error parsing JSON response: {:?}", err);
                time::sleep(Duration::from_secs(60)).await;
                continue;
            }
        };

        println!("---AI test parsed_response {:?}", parsed_response);
        {
        let mut risk_score_map = RISK_SCORE_MAP.write().unwrap();
        for entry in parsed_response {
            let wallet = entry.wallet;
            let risk_score = entry.risk_score as u32;
            println!("Wallet: {:?}, Risk Score: {:?}", wallet, risk_score);
            risk_score_map.insert(wallet, risk_score as u32);
            
        }
        println!("---AI test get risk_score {:?}", risk_score_map);
        
        drop(risk_score_map);
    }
/*         if parsed_response.is_array() {
            let mut risk_score_map = RISK_SCORE_MAP.lock().unwrap();
            for value in parsed_response.as_array().unwrap().iter() {
                if let (Some(account), Some(risk_score)) = (value["wallet"].as_str(), value["risk_score"].as_u64()) {
                    risk_score_map.insert(account.to_string(), risk_score as u32);
                }
            }

            println!("---AI test get risk_score {:?}", risk_score_map);
        } else {
            let mut risk_score_map = RISK_SCORE_MAP.lock().unwrap();
            if let (Some(account), Some(risk_score)) = (parsed_response["wallet"].as_str(), parsed_response["risk_score"].as_u64()) {
                risk_score_map.insert(account.to_string(), risk_score as u32);
            }

            println!("---AI test get risk_score {:?}", risk_score_map);
        } */
        // The lock is automatically released when risk_score_map goes out of scope.
        time::sleep(Duration::from_secs(60)).await;
        
    }
}



async fn send_http_request(url: &str) -> Result<String, reqwest::Error> {
    let client = reqwest::Client::new();
    let response = client.get(url).send().await?;
    response.text().await
}
