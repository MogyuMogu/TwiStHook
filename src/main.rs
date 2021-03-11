use dotenv::dotenv;
use reqwest::header::*;
use futures_util::StreamExt;
use chrono::Utc;
use json_flex;
use std::env;
use regex::Regex;

fn from_env(name: &str) -> String {
    match std::env::var(name) {
        Ok(val) => val,
        Err(err) => {
            println!("{}: {}", err, name);
            std::process::exit(1);
        }
    }
}

async fn run() {
    del_all().await;
    post_rules().await;
    filtered_stream().await;
}

async fn post_rules() {
    let client = reqwest::Client::new();
    let endpoint = "https://api.twitter.com/2/tweets/search/stream/rules";
    let mut map = HeaderMap::new();
    map.insert(
        CONTENT_TYPE,
        HeaderValue::from_static("application/json"),
    );
    let response = client.post(endpoint)
        .bearer_auth(from_env("BEARER_TOKEN"))
        .headers(map)
        .body(from_env("TRACK"))
        .send()
        .await
        .unwrap()
        .text()
        .await
        .unwrap();
    println!("{}", response);
}

async fn get_rules() -> String {
    let endpoint = "https://api.twitter.com/2/tweets/search/stream/rules";
    let mut map = HeaderMap::new();
    map.insert(
        CONTENT_TYPE,
        HeaderValue::from_static("application/json"),
    );
    let response = reqwest::Client::new()
        .request(reqwest::Method::GET, endpoint)
        .bearer_auth(from_env("BEARER_TOKEN"))
        .headers(map)
        .send()
        .await
        .unwrap()
        .text()
        .await
        .unwrap();
    println!("{}", response);
    response
}

async fn del_rules(delete_body: String) {
    let client = reqwest::Client::new();
    let endpoint = "https://api.twitter.com/2/tweets/search/stream/rules";
    let mut map = HeaderMap::new();
    map.insert(
        CONTENT_TYPE,
        HeaderValue::from_static("application/json"),
    );
    let response = client.post(endpoint)
        .bearer_auth(from_env("BEARER_TOKEN"))
        .headers(map)
        .body(delete_body)
        .send()
        .await
        .unwrap()
        .text()
        .await
        .unwrap();
    println!("{}", response);
}

async fn del_all() {
    let re = Regex::new(r#""id":("\d{19}",)"#).unwrap();
    let current_rules = get_rules().await;
    let regids = re.captures_iter(&current_rules);
    let mut delete_body = r#"{"delete":{"ids":["#.to_string();
    for id in regids {
        delete_body.push_str(&id[1]);
    }
    delete_body.pop();
    delete_body.push_str("]}}");
    del_rules(delete_body).await;
}

async fn filtered_stream() {
    let endpoint = "https://api.twitter.com/2/tweets/search/stream";
    let mut map = HeaderMap::new();
    map.insert(
        CONTENT_TYPE,
        HeaderValue::from_static("application/json"),
    );
    let mut stream = reqwest::Client::new()
        .request(reqwest::Method::GET, endpoint)
        .bearer_auth(from_env("BEARER_TOKEN"))
        .headers(map)
        .query(&[("expansions", "author_id"), ("user.fields", "username")])
        .send()
        .await
        .unwrap()
        .bytes_stream();
    while let Some(item) = stream.next().await {
        if let Ok(i) = item {
            let converted: String = String::from_utf8(i.to_vec()).unwrap();
            println!("Chunk {}: {}", Utc::now().format("%H:%M:%S%.9f").to_string(), converted);
            if converted != "\r\n" {
                let jf = json_flex::decode(converted);
                webhook(format!("https://twitter.com/{}/status/{}"
                , jf["includes"]["users"][0]["username"].unwrap_string().to_string()
                , jf["data"]["id"].unwrap_string().to_string())
                , from_env("WEBHOOK"))
                .await;
            }
        }
    }
}

async fn webhook(content: String, endpoint: String) {
    println!("sending... {}", endpoint);
    let client = reqwest::Client::new();
    let mut header = HeaderMap::new();
    header.insert(
        CONTENT_TYPE,
        HeaderValue::from_static("application/json"),
    );
    let body = from_env("FORMAT").replace("twitterURL", &content);
    let response = client.post(&endpoint)
        .headers(header)
        .query(&[("wait", "true")])
        .body(body)
        .send()
        .await
        .unwrap()
        .text()
        .await
        .unwrap();
    println!("{}", response);
}

fn main() {
    dotenv().ok();
    let args: Vec<String> = env::args().collect();
    let rt = tokio::runtime::Runtime::new().unwrap();
    let mut map = HeaderMap::new();
    map.insert(
        CONTENT_TYPE,
        HeaderValue::from_static("application/json"),
    );
    let _task = async {
        let mode = args[1].to_string();
        if mode == "run".to_string() {
            run().await;
        } else if mode == "post".to_string() {
            post_rules().await; 
        } else if mode == "get".to_string() {
            get_rules().await; 
        } else if mode == "delete".to_string() {
            del_rules(from_env("DELETE")).await;
        } else if mode == "delall".to_string() {
            del_all().await;
        } else if mode == "stream".to_string() {
            filtered_stream().await;
        } else if mode == "test".to_string() {
            webhook("https://twitter.com".to_string(), from_env("TEST_WEBHOOK")).await;
        }
    };
    rt.block_on(_task);
}
