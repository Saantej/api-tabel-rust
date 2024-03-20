use actix_web::{web, App, HttpResponse, HttpServer, Responder};
use serde::Deserialize;
use std::sync::Arc;
use tokio_postgres::{NoTls, Error};
use reqwest;
use std::time::SystemTime;
use chrono::{DateTime, Utc};
#[derive(Deserialize)]
struct IdUser {
    user_id: String,
}

async fn register_nfc_action(db_pool: web::Data<Arc<tokio_postgres::Client>>, info: web::Query<IdUser>) -> Result<impl Responder, actix_web::Error> {
    let user_id: i32 = info.user_id.parse().map_err(|_| actix_web::error::ErrorBadRequest("Invalid user_id"))?;

    // Отправляем запрос на внешний сервер после проверки/регистрации действия
    send_external_action(user_id, db_pool.clone()).await.map_err(|e| {
        eprintln!("Ошибка при отправке действия на внешний сервер: {}", e);
        actix_web::error::ErrorInternalServerError("Internal server error")
    })?;

    Ok(HttpResponse::Ok().body(format!("Действие для пользователя {} было успешно отправлено", user_id)))
}

async fn send_external_action(user_id: i32, db_pool: web::Data<Arc<tokio_postgres::Client>>) -> Result<(), Box<dyn std::error::Error>> {
    let (last_action, last_action_time) = db_pool.query_one(
        "SELECT action, timestamp FROM actions WHERE user_id = $1 ORDER BY timestamp DESC LIMIT 1",
        &[&user_id]
    ).await.map(|row| {
        let last_action: String = row.get(0);
        let last_action_time: SystemTime = row.get(1);
        let last_action_time = DateTime::<Utc>::from(last_action_time).naive_utc();
        (last_action, last_action_time)
    })?;
    let now = chrono::Utc::now().naive_utc();

    if now.signed_duration_since(last_action_time).num_seconds() < 10 {
        return Err(Box::new(std::io::Error::new(std::io::ErrorKind::Other, "Подождите 10 секунд перед следующим действием")));
    }

    let action = if last_action == "come" { "left" } else { "come" };

    let client = reqwest::Client::new();
    let url = format!("http://127.0.0.1:8000/api/users/{}/{}/", user_id, action);
    println!("{}", url);

    client.post(&url)
        .send()
        .await?
        .error_for_status()?;

    println!("Успешно отправлено действие '{}' для пользователя {}", action, user_id);
    update_last_action(&db_pool, user_id, &action).await?;

    Ok(())
}



async fn check_last_action(db_pool: &Arc<tokio_postgres::Client>, user_id: i32) -> Result<bool, Error> {
    let row = db_pool.query_one("SELECT action FROM actions WHERE user_id = $1 ORDER BY timestamp DESC LIMIT 1", &[&user_id]).await?;
    Ok(row.get::<_, String>(0) == "come")
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    let (client, connection) = tokio_postgres::connect("host=localhost user=pweb password=pweb dbname=tabel", NoTls).await.unwrap();
    let client = Arc::new(client);

    tokio::spawn(async move {
        if let Err(e) = connection.await {
            eprintln!("connection error: {}", e);
        }
    });

    HttpServer::new(move || {
        App::new()
            .app_data(web::Data::new(client.clone()))
            .service(web::resource("/nfc_action").route(web::get().to(register_nfc_action)))
    })
        .bind("127.0.0.1:8081")?
        .run()
        .await
}

async fn update_last_action(db_pool: &Arc<tokio_postgres::Client>, user_id: i32, action: &str) -> Result<(), Error> {
    db_pool.execute(
        "INSERT INTO actions (user_id, action, timestamp) VALUES ($1, $2, NOW())",
        &[&user_id, &action]
    ).await?;
    Ok(())
}
