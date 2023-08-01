use std::{collections::HashMap, env};

use clap::{Parser, Subcommand};
use futures_util::future::join_all;
use http_cache_reqwest::{CACacheManager, Cache, CacheMode, HttpCache, HttpCacheOptions};
use reqwest::header::{HeaderMap, HeaderValue};

#[derive(Debug, Parser)]
struct Args {
    #[command(subcommand)]
    mode: Mode,
}

#[derive(Subcommand, Debug)]
enum Mode {
    Tours { format: String },
    Formats,
}

#[derive(serde::Deserialize, serde::Serialize, Debug)]
struct Tournaments {
    pairings: Vec<Standing>,
}

#[derive(serde::Deserialize, serde::Serialize, Debug)]
struct Record {
    wins: u32,
    losses: u32,
    ties: u32,
}

#[derive(serde::Deserialize, serde::Serialize, Debug)]
struct Standing {
    decklist: Vec<Mon>,
    record: Record,
}

#[derive(serde::Deserialize, serde::Serialize, Debug)]
struct Mon {
    id: String,
    name: String,
    item: String,
    tera: Option<String>,
    ability: String,
    attacks: Vec<String>,
}

#[derive(serde::Deserialize, serde::Serialize, Debug)]
struct TourData {
    id: String,
}

#[tokio::main(flavor = "multi_thread")]
async fn main() {
    let args = Args::parse();
    let key = get_limitless_key();

    let mut headers = HeaderMap::new();
    let header_value = match HeaderValue::from_str(&key) {
        Ok(header_value) => header_value,
        Err(e) => panic!("Error creating header value - {}", e),
    };
    headers.insert("X-Access-Key", header_value);

    let header_value = match HeaderValue::from_str("application/json") {
        Ok(header_value) => header_value,
        Err(e) => panic!("Error creating header value - {}", e),
    };
    headers.insert("Content-type", header_value);

    let client = reqwest_middleware::ClientBuilder::new(
        reqwest::ClientBuilder::new()
            .default_headers(headers)
            .build()
            .expect("Error building client"),
    )
    .with(Cache(HttpCache {
        mode: CacheMode::Default,
        manager: CACacheManager::default(),
        options: HttpCacheOptions::default(),
    }))
    .build();

    match args.mode {
        Mode::Formats => {
            get_games(&client).await;
        }
        Mode::Tours { format } => {
            let start = std::time::Instant::now();
            get_tours(&client, &format)
                .await
                .expect("Error getting tours");
            println!("Time taken={:?}", start.elapsed());
        }
    }
}

async fn get_tours(
    client: &reqwest_middleware::ClientWithMiddleware,
    format: &str,
) -> Result<(), anyhow::Error> {
    let max_tours_count = u128::MAX;
    let url = format!(
        "https://play.limitlesstcg.com/api/tournaments?format={}&limit={}",
        format, max_tours_count
    );

    let resp = client.get(&url).send().await?;
    let entries = resp.json::<Vec<TourData>>().await?;
    println!("total_entries={}", entries.len());
    // Make parallel requests for each entry.
    let mut handles = vec![];
    for entry in entries {
        let task_client = client.clone();
        let url = format!(
            "https://play.limitlesstcg.com/api/tournaments/{}/standings",
            entry.id
        );
        handles.push(tokio::spawn(async move {
            let resp = task_client.get(&url).send().await;
            let resp = match resp {
                Ok(resp) => resp,
                Err(e) => {
                    println!("Error getting games - {}", e);
                    return Err(anyhow::anyhow!("Error getting games"));
                }
            };
            // let text = resp.text().await.expect("Error getting text");
            let body = resp.json::<Vec<Standing>>().await;
            // let body = resp.json::<serde_json::Value>().await;
            // let body = serde_json::from_str::<Vec<Standing>>(&text);
            match body {
                Ok(body) => Ok(body),
                Err(_e) => Err(anyhow::anyhow!("Error parsing json")),
            }
        }));
    }
    let results = join_all(handles).await;
    // let mons = HashMap::new();
    for result in results.into_iter().flatten().flatten() {
        // println!(
        //     "{}",
        //     serde_json::to_string_pretty(&resp).expect("Error serializing json")
        // )
        // println!("{:?}", resp)
    }
    Ok(())
}

async fn get_games(client: &reqwest_middleware::ClientWithMiddleware) {
    let url = "https://play.limitlesstcg.com/api/games";
    let resp = client.get(url).send().await;
    let resp = match resp {
        Ok(resp) => resp,
        Err(e) => panic!("Error getting games - {}", e),
    };
    let body: Result<serde_json::Value, reqwest::Error> = resp.json().await;
    let json = match body {
        Ok(json) => json,
        Err(e) => panic!("Error parsing json - {}", e),
    };
    let games_array = match json.as_array() {
        Some(games_array) => games_array,
        None => panic!("Error parsing json as array"),
    };
    for game in games_array {
        if game["id"] == "VGC" {
            println!(
                "{}",
                serde_json::to_string_pretty(&game).expect("Error serializing json")
            );
        }
    }
}

fn get_limitless_key() -> std::string::String {
    match env::var("LIMITLESS_API_KEY") {
        Ok(var) => var,
        Err(e) => panic!("LIMITLESS_API_KEY not found in environment - {}", e),
    }
}
