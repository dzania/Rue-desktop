#![cfg_attr(
    all(not(debug_assertions), target_os = "windows"),
    windows_subsystem = "windows"
)]

use futures::{pin_mut, stream, StreamExt};
use mdns;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::{
    collections::HashMap,
    fs,
    io::{BufReader, Write},
    net::IpAddr,
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

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Bridge {
    internalipaddress: String,
}
/// Find bridges using mdns method
/// https://developers.meethue.com/develop/application-design-guidance/hue-bridge-discovery/#mDNS
#[tauri::command]
async fn mdns_discovery() -> Result<Vec<Bridge>, String> {
    const SERVICE_NAME: &'static str = "_hue._tcp.local";
    let stream = mdns::discover::all(SERVICE_NAME, Duration::from_millis(10))
        .map_err(|e| e.to_string())?
        .listen();
    pin_mut!(stream);
    let mut bridges = vec![];
    while let Some(Ok(response)) = stream.next().await {
        println!("{:#?}", response);
        let addr = response.records().filter_map(to_ip_addr).next();

        if let Some(addr) = addr {
            println!("found cast device at {}", addr);
            bridges.push(Bridge {
                internalipaddress: addr.to_string(),
            });
            break;
        } else {
            println!("cast device does not advertise address");
        }
    }
    Ok(bridges)
}
fn to_ip_addr(record: &mdns::Record) -> Option<IpAddr> {
    match record.kind {
        mdns::RecordKind::A(addr) => Some(addr.into()),
        mdns::RecordKind::AAAA(addr) => Some(addr.into()),
        _ => None,
    }
}
// find bridges using discovery url
#[tauri::command]
async fn find_bridges() -> Result<Vec<Bridge>, String> {
    let request: Vec<Bridge> = reqwest::get("https://discovery.meethue.com/")
        .await
        .map_err(|e| e.to_string())?
        .json()
        .await
        .map_err(|e| e.to_string())?;
    if request.is_empty() {
        return Err("No bridges found".to_string());
    }
    Ok(request)
}

// Send parallel requests to all bridges found
pub async fn create_user(bridges: Vec<Bridge>) -> Result<(), String> {
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
        .invoke_handler(tauri::generate_handler![
            save,
            load,
            find_bridges,
            mdns_discovery
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
