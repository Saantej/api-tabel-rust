use std::error::Error;
use serde::Deserialize;
use actix_web::{web, App, HttpServer, Responder, Result};

#[derive(Debug, Deserialize)]
struct Event {
    time0nd: Option<String>,
}

async fn handle_user_action() -> Result<impl Responder, Box<dyn std::error::Error>> {
    let user_id = 52;
    let event = get_data(user_id).await?;
    let url_base = "http://212.109.221.149:8002/api/users/";
    let action = if event == 0 {"come"} else {"left"};
    let url = format!("{url_base}{user_id}/{action}/");
    let client = reqwest::Client::new();
    client.post(&url).send().await?;
    println!("{}", url);

    if event == 0 {
        Ok("Пришел")
    } else {
        Ok("Ушел")
    }
}

async fn get_data(user_id: i32) -> Result<i32, Box<dyn Error>> {
    let url = format!("http://212.109.221.149:8002/api/users/{}/get_data/", user_id);
    let client = reqwest::Client::new();
    let response = client.get(&url).send().await?;

    if response.status().is_success() {
        let events: Vec<Event> = response.json().await?;

        if let Some(first_event) = events.into_iter().nth(0) {
            if first_event.time0nd.as_ref().map_or(true, |t| t.trim().is_empty()) {
                Ok(1)
            } else {
                Ok(0)
            }
        } else {
            Ok(3)
        }
    } else {
        Err(format!("Ошибка при получении данных: {}", response.status()).into())
    }
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    HttpServer::new(|| {
        App::new().route("/handle_user", web::get().to(handle_user_action))
    })
        .bind("0.0.0.0:4444")?
        .run()
        .await
}



