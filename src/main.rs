mod handlers;
mod server_utils;
use crate::handlers::*;
use crate::server_utils::*;
use tower_http::services::ServeDir;
use axum_extra::extract::cookie::Key;
use std::net::*;
use axum::{
    routing::{get, post},
    middleware,
    Router,
};
use std::env;


#[tokio::main]
async fn main() {
    if load_env() == ServerCode::Fail { return; }

    // connect postgres neon with url from .env
    let database_url = match get_db_url() {
        Ok(url) => url,
        Err(_) => { return },
    };

    // create postgresql connection pool
    let pool = match create_connection_pool(database_url).await {
        Ok(pool) => pool,
        Err(_) => { return },
    };

    // create app state containing the active db connection pool
    let state = AppState { db: pool, key: Key::generate() };

    // get IpV4 socket
    let socket = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 3000);

    let protected_routes = Router::new()
        .route("/dashboard", get(get_dashboard_handler))
        .route_layer(middleware::from_fn_with_state(state.clone(), auth_middleware));

    let app = Router::new()
        .route("/", get(get_landing_handler))
        .route("/login", post(login_handler))
        .route("/create_account_user", get(create_user_get).post(create_user_post))
        .merge(protected_routes)
        .nest_service("/frontend", ServeDir::new("frontend"))
        .with_state(state);


    // bind socket to tokio TcpListener
    // let listener = match tokio::net::TcpListener::bind(&socket).await {
    //     Ok(listener) => {
    //         println!("> server listening @ -> {}", &socket);
    //         listener
    //     },
    //     Err(e) => {
    //         eprintln!("> Failed to bind addr to listener -> {e}");
    //         return;
    //     },
    // };

    let port = env::var("PORT")
        .unwrap_or_else(|_| "8080".to_string())
        .parse::<u16>()
        .expect("PORT must be a valid number");

    // 3. Bind to 0.0.0.0 using the dynamic port
    let addr = SocketAddr::from(([0, 0, 0, 0], port));

    let listener = match tokio::net::TcpListener::bind(addr).await {
        Ok(listener) => {
            println!("> server running @ {}", addr);
            listener
        },
        Err(e) => {
            eprintln!("> Error binding TCP listener | {e}");
            return;
        },
    };

    axum::serve(listener, app).await.unwrap();
}





