#![cfg_attr(
    all(not(debug_assertions), target_os = "windows"),
    windows_subsystem = "windows"
)]

use futures::{stream, StreamExt};
use reqwest::{Client, Error};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::{
    collections::HashMap,
    fs,
    io::{BufReader, Write},
    path::{Path, PathBuf},
    thread,
    time::Duration,
};
use tokio::sync::mpsc;

const CONFIG_DIR: &str = ".config/rue";
const CONFIG_NAME: &str = "rue.json";

#[derive(Serialize, Deserialize, Debug)]
pub struct User {
    pub username: String,
    pub bridge_address: String,
}

fn get_config_path() -> Result<PathBuf, String> {
    match dirs::home_dir() {
        Some(home) => {
            let config_dir_path = Path::new(&home.join(CONFIG_DIR)).to_owned();
            if !config_dir_path.exists() {
                fs::create_dir(&config_dir_path).map_err(|e| e.to_string())?;
            };
            Ok(config_dir_path.join(CONFIG_NAME))
        }
        None => Err("No $HOME directory found for config".into()),
    }
}

// Store User used for api calls
#[tauri::command]
async fn save(user: User) -> Result<(), String> {
    let config_file_path = get_config_path()?;
    let mut file = fs::File::create(&config_file_path).map_err(|e| e.to_string())?;

    let json: String = serde_json::to_string(&user).map_err(|e| e.to_string())?;

    file.write_all(json.as_bytes()).map_err(|e| e.to_string())?;

    Ok(())
}
// Load User used for api calls
#[tauri::command]
fn load() -> Result<User, String> {
    let file_path = get_config_path()?;
    let file = fs::File::open(file_path).map_err(|e| e.to_string())?;
    let reader = BufReader::new(file);
    let user = serde_json::from_reader(reader).map_err(|e| e.to_string())?;

    Ok(user)
}

#[derive(Deserialize, Debug, Clone)]
pub struct Bridge {
    internalipaddress: String,
}
// find bridges using discovery url
pub async fn find_bridges() -> Result<Vec<Bridge>, String> {
    let request: Vec<Bridge> = reqwest::get("https://discovery.meethue.com/")
        .await
        .map_err(|e| e.to_string())?
        .json()
        .await
        .map_err(|e| e.to_string())?;
    if request.is_empty() {
        panic!("No bridges found");
    }
    Ok(request)
}

// Send parallel requests to all bridges found
pub async fn create_user() -> Result<(), String> {
    let bridges: Vec<Bridge> = find_bridges()
        .await
        .expect("No bridges found")
        .into_iter()
        .collect();

    // Poll bridge for minute
    for _ in 1..25 {
        let (tx, mut rx) = mpsc::channel(4);
        let requests = stream::iter(bridges.clone())
            .map(|bridge| {
                tokio::spawn(async move { authorize_user_request(&bridge.internalipaddress).await })
            })
            .buffer_unordered(bridges.len());

        requests
            .for_each(|b| async {
                match b {
                    Ok(Ok(b)) => {
                        let _ = tx.send(b).await;
                    }
                    // FIXME: Shouldn't print to std
                    Ok(Err(e)) => println!("Got a reqwest::Error: {:?}", e),
                    Err(e) => println!("Error: {}", e),
                }
            })
            .await;

        if let Some(user) = rx.recv().await {
            save(user).await?;
            break;
        };
        thread::sleep(Duration::from_secs(5));
    }
    Ok(())
}

/// Send request to bridge to get User
pub async fn authorize_user_request(ip: &str) -> Result<User, ()> {
    let address = format!("http://{}/api", ip);
    let client = Client::new();
    let mut body = HashMap::new();
    body.insert("devicetype", "rue_pc_app");
    let resp = client
        .post(&address)
        .json(&body)
        .send()
        .await
        .map_err(|_| ())?;
    let data = resp.text().await.map_err(|_| ())?;
    let value: Value = serde_json::from_str(&data).unwrap();

    match value[0].get("success") {
        Some(message) => {
            let username: String = serde_json::from_value(message.to_owned()).unwrap();
            let user = User {
                username,
                bridge_address: ip.into(),
            };
            Ok(user)
        }
        None => Err(()),
    }
}

fn main() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![save, load])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
