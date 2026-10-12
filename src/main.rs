mod handlers;
mod server_utils;
use crate::handlers::*;
use crate::server_utils::*;
use tower_http::services::ServeDir;
use axum_extra::extract::cookie::{SignedCookieJar, Key};
use std::net::*;
use axum::{
    routing::{get, post},
    middleware,
    Router,
    extract::Request,
    response::{IntoResponse, Redirect, Response},
};


async fn auth_middleware(
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

    // // set up axum router
    // let router: Router = Router::new()
    //     .route("/", get(get_landing_handler))
    //     .route("/create_account_user", get(create_user_get).post(create_user_post))
    //     .route("/dashboard", get(get_dashboard_handler))
    //     .nest_service("/frontend", ServeDir::new("frontend"))
    //     .with_state(state);

    let app = Router::new()
        .route("/", get(get_landing_handler))
        .route("/login", post(login_handler))
        .route("/create_account_user", get(create_user_get).post(create_user_post))
        .merge(protected_routes)
        .nest_service("/frontend", ServeDir::new("frontend"))
        .with_state(state);


    // bind socket to tokio TcpListener
    let listener = match tokio::net::TcpListener::bind(&socket).await {
        Ok(listener) => {
            println!("> server listening @ -> {}", &socket);
            listener
        },
        Err(e) => {
            eprintln!("> Failed to bind addr to listener -> {e}");
            return;
        },
    };

    axum::serve(listener, app).await.unwrap();
}





