use sqlx::{ postgres::{PgPoolOptions, Postgres,}, pool::Pool, };
use std::env;

#[derive(PartialEq)]
pub enum ServerCode {
    Success,
    Fail,
}

pub fn load_env() -> ServerCode {
    match dotenvy::dotenv() {
        Ok(_) => {
            println!("> Successfully loaded .env vars.");
            ServerCode::Success
        },
        Err(e) => {
            eprintln!("Failed to load .env vars | {e}");
            ServerCode::Fail
        }
    }
}

pub fn get_db_url() -> Result<String, ServerCode> {
    println!("> fetching postgres db url.");
    match env::var("DATABASE_URL") {
        Ok(url) => {
            println!("> url found: {}", url);
            Ok(url)
        },
        Err(e) => {
            eprintln!("> db url env var not found | {e}");
            Err(ServerCode::Fail)
        },
    }
}

pub async fn create_connection_pool(db_url: String) -> Result<Pool<Postgres>, ServerCode>{
    println!("> creating connection pool.");
    return match PgPoolOptions::new()
        .max_connections(5)
        .connect(&db_url)
        .await {
            Ok(pool) => Ok(pool),
            Err(_) => { 
                eprintln!("> Failed to initiate db connection pool.");
                Err(ServerCode::Fail)
            }
        }
}