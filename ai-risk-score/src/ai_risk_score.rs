use lazy_static::lazy_static;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use serde_json::Value;
use std::time::Duration;
use tokio::time;

lazy_static! {
    pub static ref RISK_SCORE_MAP: Arc<Mutex<HashMap<String, u32>>> = Arc::new(Mutex::new(HashMap::new()));
}

pub async fn get_risk_score() {
    let url = "http://127.0.0.1:5000/retrieve_risk_score_by_timestamp";

    println!("AI Risk Score Test ");
    eprintln!("AI Risk Score Test");
    
    loop {
        // Use the tokio runtime to run the asynchronous function and get the JSON response.
        {
        let json_response = match send_http_request(url).await {
            Ok(json) => json,
            Err(err) => {
                println!("Error sending the request: {:?}", err);
                time::sleep(Duration::from_secs(600)).await;
                continue;
            }
        };

        // Parse the JSON response into a serde_json::Value
        let parsed_response: Value = match serde_json::from_str(&json_response) {
            Ok(parsed) => parsed,
            Err(err) => {
                println!("Error parsing JSON response: {:?}", err);
                time::sleep(Duration::from_secs(600)).await;
                continue;
            }
        };

        // Lock the map and insert values if the response is an array
        let mut risk_score_map = RISK_SCORE_MAP.lock().unwrap();
        if parsed_response.is_array() {
            for value in parsed_response.as_array().unwrap().iter() {
                if let (Some(account), Some(risk_score)) = (value["account"].as_str(), value["risk_score"].as_u64()) {
                    risk_score_map.insert(account.to_string(), risk_score as u32);
                }
            }
        }
    }
        // The lock is automatically released when risk_score_map goes out of scope.

        time::sleep(Duration::from_secs(600)).await;
    }
}



async fn send_http_request(url: &str) -> Result<String, reqwest::Error> {
    let client = reqwest::Client::new();
    let response = client.get(url).send().await?;
    response.text().await
}
