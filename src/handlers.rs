use axum::{
    extract::{Form, State, FromRef},
    http::{StatusCode},
    response::{Html, IntoResponse, Redirect},
};
use sqlx::{PgPool};
use serde::*;
use axum_extra::extract::cookie::{SignedCookieJar, Key};

#[derive(Clone)]
pub struct AppState {
    pub db: PgPool,
    pub key: Key,
}

impl FromRef<AppState> for Key {
    fn from_ref(state: &AppState) -> Self {
        state.key.clone()
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub struct User {
    first_name: String,
    last_name: String,
    email: String,
    password: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct LoginUser {
    email: String,
    password: String,
}

async fn verify_email_exists(
    State(state): State<AppState>, 
    Form(payload): Form<LoginUser>
) -> (LoginUser, bool) {
    println!("payload: {:?}", &payload);
    match sqlx::query_scalar::<_, i64>(
        r#"
        SELECT COUNT(*) 
        FROM users 
        WHERE email = $1
        "#
    ).bind(&payload.email)
    .fetch_one(&state.db)
    .await {
        Ok(count) => {
            return (payload, (count > 0 && count < 2));
        },
        Err(e) => {
            eprintln!("> Failed to check for valid user: {e}");
            return (payload, false);
        }
    };
}

async fn validate_email_to_password(
    State(state): State<AppState>,
    email: String,
    password: String,
) -> bool {
    match sqlx::query_scalar::<_, String>(
        r#"
        SELECT password_hash
        FROM users
        WHERE email = $1
        "#
    )
    .bind(&email)
    .fetch_optional(&state.db)
    .await {
        Ok(Some(stored)) => stored == password,
        Ok(None) => false,
        Err(e) => {
            eprintln!("> Failed to validate password: {e}");
            false
        }
    }
}

pub async fn login_handler(
    jar: SignedCookieJar, 
    State(state): State<AppState>, 
    Form(payload): Form<LoginUser>
) -> impl IntoResponse {
    let verified = verify_email_exists(State(state.clone()), Form(payload)).await;
    if !verified.1 {
        eprintln!("> Failed to find valid email.");
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            "Unable to search email in db",
        ).into_response();
    }

    // verify password matches the form using sqlx query
    let password = verified.0.password;
    let email = verified.0.email;

    let valid_password = validate_email_to_password(State(state), email, password).await;
    if !valid_password {
        eprintln!("> Password incorrect.");
        return (
            StatusCode::FORBIDDEN,
            "Unable to load landing page",
        ).into_response();
    }

    let cookie = axum_extra::extract::cookie::Cookie::build(("auth_token", "user_session_123"))
        .http_only(true)
        .path("/")
        .build();

    return (jar.add(cookie), Redirect::permanent("/dashboard")).into_response();
}

pub async fn get_landing_handler() -> impl IntoResponse {
    match tokio::fs::read_to_string("frontend/index.html").await {
        Ok(html) => Html(html).into_response(),
        Err(error) => {
            eprintln!("> Failed to read landing page: {error}");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                "Unable to load landing page",
            ).into_response()
        }
    }
}

fn escape_html(input: &str) -> String {
    input
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#39;")
}

pub async fn create_user_get() -> impl IntoResponse {
    println!("> Create account called");
    match tokio::fs::read_to_string("frontend/create_account.html").await {
        Ok(html) => Html(html).into_response(),
        Err(error) => {
            eprintln!("> Failed to read landing page: {error}");
            (   
                StatusCode::INTERNAL_SERVER_ERROR,
                "Unable to load landing page",
            ).into_response()
        }
    }
}

pub async fn create_user_post(
    State(state): State<AppState>,
    Form(payload): Form<User>,
) -> impl IntoResponse {
    println!("> Create account post called");
    println!("payload: {:?}", &payload);

    println!("> Checking for non-unique user email.");
    // check email doesnt exist in DB
    let count: i64 = match sqlx::query_scalar(
        r#"
        SELECT COUNT(*) 
        FROM users 
        WHERE email = $1
        "#
    ).bind(&payload.email)
    .fetch_one(&state.db)
    .await {
        Ok(count) => count,
        Err(e) => {
            eprintln!("> Failed to check for existing user: {e}");
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                "Unable to check for existing user",
            ).into_response();
        }
    };

    println!("> Verifying count.");
    println!("> COUNT IS {count}");
    if count > 0 {
        let mut contents = match tokio::fs::read_to_string("frontend/create_account.html").await {
            Ok(contents) => contents,
            Err(error) => {
                eprintln!("> Failed to read create account page: {error}");
                return (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "Unable to load create account page",
                )
                    .into_response();
            }
        };
        let alert = format!(
            r#"<div class="alert" role="alert">
                <p><strong>Account already exists</strong></p>
                <p>An account with <span class="alert-email">{}</span> is already registered.</p>
                <a class="alert-button" href="/">Log in</a>
            </div>"#,
            escape_html(&payload.email),
        );
        contents = contents.replace(r#"<div id="error_msg"></div>"#, &alert);
        return (StatusCode::CONFLICT, Html(contents)).into_response();
    }

    println!("> Updating db...");
    let fullname = format!("{} {}", payload.first_name.to_lowercase().trim(), payload.last_name.to_lowercase().trim());

    // else update db with new user and redirect to dashboard
    match sqlx::query(
        r#"
        INSERT INTO users (fullname, email, password_hash)
        VALUES ($1, $2, $3)
        "#
    )
    .bind(fullname)
    .bind(&payload.email)
    .bind(&payload.password)
    .execute(&state.db)
    .await {
        Ok(_) => println!("> Successfully updated db."),
        Err(_) => { 
            eprintln!("> Failed to update db.");
            return (   
                StatusCode::INTERNAL_SERVER_ERROR,
                "Unable to insert user into db.",
            ).into_response();
        },
    };

    println!("> Redirecting to /dashboard...");
    Redirect::permanent("/dashboard").into_response()

}

pub async fn get_dashboard_handler() -> impl IntoResponse {
    println!("> Create account called");
    match tokio::fs::read_to_string("frontend/dashboard.html").await {
        Ok(html) => Html(html).into_response(),
        Err(error) => {
            eprintln!("> Failed to read dashboard page: {error}");
            (   
                StatusCode::INTERNAL_SERVER_ERROR,
                "Unable to load dashboard page",
            ).into_response()
        }
    }
}
