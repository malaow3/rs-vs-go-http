use std::env;

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

#[tokio::main(flavor = "multi_thread", worker_threads = 4)]
async fn main() {
    let args = Args::parse();
    let key = get_limitless_key();
    let mut headers = HeaderMap::new();
    let header_value = match HeaderValue::from_str(&key) {
        Ok(header_value) => header_value,
        Err(e) => panic!("Error creating header value - {}", e),
    };
    headers.insert("X-Access-Key", header_value);
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

    let start = std::time::Instant::now();
    match args.mode {
        Mode::Formats => {
            get_games(&client).await;
        }
        Mode::Tours { format } => {
            get_tours(&client, &format)
                .await
                .expect("Error getting tours");
        }
    }
    let end = std::time::Instant::now();
    println!("Time taken={:?}", end.duration_since(start));
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
    let json = resp.json::<serde_json::Value>().await?;
    let entries = match json.as_array() {
        Some(entries) => entries,
        None => return Err(anyhow::anyhow!("Error parsing json as array")),
    };
    println!("total_entries={}", entries.len());
    // Make parallel requests for each entry.
    let mut handles = vec![];
    for entry in entries {
        let entry = entry.clone();
        let task_client = client.clone();
        handles.push(tokio::spawn(async move {
            let id = match entry["id"].as_str() {
                Some(id) => id,
                None => {
                    println!("Error parsing tour id");
                    return;
                }
            };

            let url = format!(
                "https://play.limitlesstcg.com/api/tournaments/{}/standings",
                id
            );
            let resp = task_client.get(&url).send().await;
            let resp = match resp {
                Ok(resp) => resp,
                Err(e) => {
                    println!("Error getting games - {}", e);
                    return;
                }
            };
            let _body = resp.text().await.unwrap();
        }));
    }
    let results = join_all(handles).await;
    for result in results {
        match result {
            Ok(_) => {}
            Err(e) => println!("Error getting tour - {}", e),
        }
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
