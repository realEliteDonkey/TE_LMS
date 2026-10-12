use sqlx::{ postgres::{PgPoolOptions, Postgres,}, pool::Pool };
use std::env;
use axum_extra::extract::cookie::SignedCookieJar;
use axum::{
    middleware,
    extract::Request,
    response::{IntoResponse, Redirect, Response},
};

#[derive(PartialEq)]
pub enum ServerCode {
    Success,
    Fail,
}

pub async fn auth_middleware(
    jar: SignedCookieJar,
    req: Request,
    next: middleware::Next,
) -> Response {
    // check if signed cookie exists and is valid
    if let Some(_cookie) = jar.get("auth_token") {
        // token is valid; continue to the handler
        next.run(req).await
    } else {
        // redirect to login or return unauthorized
        Redirect::to("/login").into_response()
    }
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