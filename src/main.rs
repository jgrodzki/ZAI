use axum::{
    extract::{Multipart, Path, Query, Request, State},
    http::{StatusCode, Uri},
    middleware::{from_fn, Next},
    response::{IntoResponse, Redirect},
    routing::{get, post},
    Form, Router,
};
use axum_htmx::{HxBoosted, HxCurrentUrl, HxLocation, HxPushUrl, HxReplaceUrl, HxRequest};
use axum_session::{Session, SessionLayer, SessionNullPool, SessionStore};
use dotenvy::dotenv;
use serde::Deserialize;
use sqlx::{migrate::MigrateDatabase, PgPool, Postgres};
use std::{collections::HashMap, env};
use tokio::{
    fs::{remove_file, rename, try_exists, File},
    io::AsyncWriteExt,
    net::TcpListener,
};
use tower_http::services::ServeDir;

mod database;
mod svg;
mod templates;

#[tokio::main]
async fn main() {
    dotenv().unwrap();
    let database_url = env::var("DATABASE_URL").unwrap();
    if !Postgres::database_exists(&database_url)
        .await
        .unwrap_or(false)
    {
        Postgres::create_database(&database_url).await.unwrap();
    }
    let pool = PgPool::connect_lazy(&database_url).unwrap();
    sqlx::migrate!().run(&pool).await.unwrap();
    let static_service = ServeDir::new("static");
    let session_store = SessionStore::<SessionNullPool>::new(None, Default::default())
        .await
        .unwrap();
    let app = Router::new()
        .route("/", get(index_handler))
        .route("/login", get(login_form_handler).post(login_handler))
        .route(
            "/register",
            get(register_form_handler).post(register_handler),
        )
        .route("/logout", post(logout_handler))
        .route("/search", get(search_handler))
        .route("/items", get(item_view_handler))
        .route(
            "/items/add",
            get(item_add_form_handler).post(item_add_handler),
        )
        .route("/items/:item", get(item_handler))
        .route(
            "/items/:item/edit",
            get(item_edit_form_handler).post(item_edit_handler),
        )
        .route(
            "/items/:item/remove",
            get(item_remove_form_handler).post(item_remove_handler),
        )
        .route(
            "/items/:item/rate",
            post(review_add_handler).delete(review_remove_handler),
        )
        .route("/users", get(user_view_handler))
        .route("/users/:user", get(user_handler))
        .route(
            "/users/:user/edit",
            get(user_edit_form_handler).post(user_edit_handler),
        )
        .route(
            "/users/:user/remove",
            get(user_remove_form_handler).post(user_remove_handler),
        )
        .nest_service("/static", static_service)
        .layer(SessionLayer::new(session_store))
        .layer(from_fn(strip_empty_query))
        .with_state(pool);
    let listener = TcpListener::bind("0.0.0.0:3000").await.unwrap();
    axum::serve(listener, app).await.unwrap();
}

async fn strip_empty_query(
    HxBoosted(boosted): HxBoosted,
    Query(mut query): Query<HashMap<String, String>>,
    mut request: Request,
    next: Next,
) -> impl IntoResponse {
    let initial_param_count = query.len();
    query.retain(|_, v| !v.is_empty() && v != "0");
    if initial_param_count != query.len() {
        let new_query_string = query
            .into_iter()
            .map(|(k, v)| format!("{}={}", k, v))
            .reduce(|acc, s| format!("{}&{}", acc, s));
        let new_pq_string = if let Some(query) = new_query_string {
            format!("{}?{}", request.uri().path(), query)
        } else {
            request.uri().path().to_owned()
        };
        let new_uri = {
            let mut parts = request.uri().clone().into_parts();
            parts.path_and_query = Some(new_pq_string.try_into().unwrap());
            Uri::from_parts(parts).unwrap()
        };
        if boosted {
            *request.uri_mut() = new_uri.clone();
        }
        let response = next.run(request).await;
        if boosted {
            (HxReplaceUrl(new_uri), response).into_response()
        } else {
            Redirect::to(&new_uri.to_string()).into_response()
        }
    } else {
        next.run(request).await
    }
}

async fn index_handler(HxBoosted(boosted): HxBoosted) -> impl IntoResponse {
    if boosted {
        (HxLocation::from_uri("/items".try_into().unwrap()), ()).into_response()
    } else {
        Redirect::to("/items").into_response()
    }
}

#[derive(Deserialize)]
struct Score {
    score: i16,
}

async fn review_add_handler(
    State(pool): State<PgPool>,
    session: Session<SessionNullPool>,
    Path(locator): Path<String>,
    HxRequest(is_htmx): HxRequest,
    HxCurrentUrl(current_url): HxCurrentUrl,
    score: Form<Score>,
) -> impl IntoResponse {
    if let Some(user) = session.get::<database::User>("user") {
        database::rate_item(&pool, &user.username, &locator, score.score)
            .await
            .unwrap();
        if is_htmx {
            (
                HxLocation {
                    uri: current_url.unwrap(),
                },
                (),
            )
                .into_response()
        } else {
            StatusCode::OK.into_response()
        }
    } else {
        StatusCode::UNAUTHORIZED.into_response()
    }
}

async fn review_remove_handler(
    State(pool): State<PgPool>,
    session: Session<SessionNullPool>,
    Path(locator): Path<String>,
    HxRequest(is_htmx): HxRequest,
    HxCurrentUrl(current_url): HxCurrentUrl,
) -> impl IntoResponse {
    let Some(user) = session.get::<database::User>("user") else {
        return StatusCode::UNAUTHORIZED.into_response();
    };
    if database::remove_review(&pool, &locator, &user.username)
        .await
        .is_ok()
    {
        if is_htmx {
            (
                HxLocation {
                    uri: current_url.unwrap(),
                },
                (),
            )
                .into_response()
        } else {
            StatusCode::OK.into_response()
        }
    } else {
        return StatusCode::INTERNAL_SERVER_ERROR.into_response();
    }
}

#[derive(Deserialize)]
struct Params {
    search: Option<String>,
    page: Option<i32>,
}

async fn item_handler(
    State(pool): State<PgPool>,
    session: Session<SessionNullPool>,
    Path(locator): Path<String>,
    query: Query<Params>,
    HxBoosted(boosted): HxBoosted,
) -> impl IntoResponse {
    if let Some(item) = database::get_item(&pool, &locator).await.unwrap() {
        if let Some(user) = session.get::<database::User>("user") {
            let item_page = templates::item_page(
                &item,
                database::get_item_ratings(&pool, query.page, &locator)
                    .await
                    .unwrap(),
                Some(&user),
                database::get_item_rating(&pool, &locator, &user.username)
                    .await
                    .unwrap(),
            );
            if boosted {
                item_page.into_response()
            } else {
                templates::index(item_page, "/items", Some(&user)).into_response()
            }
        } else {
            let item_page = templates::item_page(
                &item,
                database::get_item_ratings(&pool, query.page, &locator)
                    .await
                    .unwrap(),
                None,
                None,
            );
            if boosted {
                item_page.into_response()
            } else {
                templates::index(item_page, "/items", None).into_response()
            }
        }
    } else {
        StatusCode::NOT_FOUND.into_response()
    }
}

async fn item_remove_form_handler(
    Path(locator): Path<String>,
    HxRequest(is_htmx): HxRequest,
) -> impl IntoResponse {
    if is_htmx {
        templates::remove_form(
            &("/items/".to_owned() + &locator + "/remove"),
            "Remove item",
            &locator,
        )
        .into_response()
    } else {
        StatusCode::NOT_FOUND.into_response()
    }
}

async fn item_remove_handler(
    State(pool): State<PgPool>,
    session: Session<SessionNullPool>,
    Path(locator): Path<String>,
    HxRequest(is_htmx): HxRequest,
) -> impl IntoResponse {
    if let Some(user) = session.get::<database::User>("user") {
        if !user.is_admin {
            return StatusCode::FORBIDDEN.into_response();
        }
    } else {
        return StatusCode::FORBIDDEN.into_response();
    }
    if database::remove_item(&pool, &locator).await.is_ok() {
        remove_file("static/images/items/".to_owned() + &locator)
            .await
            .unwrap();
        if is_htmx {
            (
                HxLocation {
                    uri: "/items".try_into().unwrap(),
                },
                (),
            )
                .into_response()
        } else {
            StatusCode::OK.into_response()
        }
    } else {
        return StatusCode::INTERNAL_SERVER_ERROR.into_response();
    }
}

async fn item_view_handler(
    State(pool): State<PgPool>,
    session: Session<SessionNullPool>,
    query: Query<Params>,
    HxBoosted(boosted): HxBoosted,
) -> impl IntoResponse {
    let content = templates::item_view(
        database::get_items(&pool, query.page, query.search.as_deref())
            .await
            .unwrap(),
        session.get("user").as_ref(),
    );
    if boosted {
        content
    } else {
        templates::index(content, "/items", session.get("user").as_ref())
    }
}

async fn user_remove_form_handler(
    Path(username): Path<String>,
    HxRequest(is_htmx): HxRequest,
) -> impl IntoResponse {
    if is_htmx {
        templates::remove_form(
            &("/users/".to_owned() + &username + "/remove"),
            "Remove user",
            &username,
        )
        .into_response()
    } else {
        StatusCode::NOT_FOUND.into_response()
    }
}

async fn user_remove_handler(
    State(pool): State<PgPool>,
    session: Session<SessionNullPool>,
    Path(username): Path<String>,
    HxRequest(is_htmx): HxRequest,
) -> impl IntoResponse {
    let Some(user) = session.get::<database::User>("user") else {
        return StatusCode::FORBIDDEN.into_response();
    };
    if !user.is_admin && user.username != username {
        return StatusCode::FORBIDDEN.into_response();
    }
    let Ok(Some(page_user)) = database::get_user(&pool, &username).await else {
        return StatusCode::INTERNAL_SERVER_ERROR.into_response();
    };
    if page_user.is_admin {
        return StatusCode::FORBIDDEN.into_response();
    }
    if database::remove_user(&pool, &username).await.is_ok() {
        if user.username == page_user.username {
            session.destroy();
        }
        if try_exists("static/images/avatars/".to_owned() + &username)
            .await
            .unwrap_or(false)
        {
            remove_file("static/images/avatars/".to_owned() + &username)
                .await
                .unwrap();
        }
        if is_htmx {
            (
                HxLocation {
                    uri: "/users".try_into().unwrap(),
                },
                (),
            )
                .into_response()
        } else {
            StatusCode::OK.into_response()
        }
    } else {
        return StatusCode::INTERNAL_SERVER_ERROR.into_response();
    }
}

async fn user_handler(
    State(pool): State<PgPool>,
    session: Session<SessionNullPool>,
    query: Query<Params>,
    Path(username): Path<String>,
    HxBoosted(boosted): HxBoosted,
) -> impl IntoResponse {
    if let Some(page_user) = database::get_user(&pool, &username).await.unwrap() {
        let user = session.get::<database::User>("user");
        let user_page = templates::user_page(
            &page_user,
            database::get_user_ratings(&pool, query.page, &username)
                .await
                .unwrap(),
            user.as_ref(),
        );
        if boosted {
            user_page.into_response()
        } else {
            templates::index(user_page, "/users", user.as_ref()).into_response()
        }
    } else {
        StatusCode::NOT_FOUND.into_response()
    }
}

async fn user_view_handler(
    State(pool): State<PgPool>,
    session: Session<SessionNullPool>,
    query: Query<Params>,
    HxBoosted(boosted): HxBoosted,
) -> impl IntoResponse {
    let content = templates::user_view(
        database::get_users(&pool, query.page, query.search.as_deref())
            .await
            .unwrap(),
    );
    if boosted {
        content
    } else {
        templates::index(content, "/users", session.get("user").as_ref())
    }
}

#[derive(Deserialize)]
#[serde(tag = "target", rename_all = "lowercase")]
enum SearchTarget {
    Items,
    Users,
}

async fn search_handler(
    State(pool): State<PgPool>,
    session: Session<SessionNullPool>,
    Query(target): Query<SearchTarget>,
    HxRequest(is_htmx): HxRequest,
) -> impl IntoResponse {
    if is_htmx {
        match target {
            SearchTarget::Items => {
                let content = templates::item_view(
                    database::get_items(&pool, None, None).await.unwrap(),
                    session.get("user").as_ref(),
                );
                (
                    HxPushUrl("/items".try_into().unwrap()),
                    templates::search("/items", Some(content)),
                )
            }
            SearchTarget::Users => {
                let content =
                    templates::user_view(database::get_users(&pool, None, None).await.unwrap());
                (
                    HxPushUrl("/users".try_into().unwrap()),
                    templates::search("/users", Some(content)),
                )
            }
        }
        .into_response()
    } else {
        StatusCode::NOT_FOUND.into_response()
    }
}

async fn user_edit_form_handler(
    Path(username): Path<String>,
    HxRequest(is_htmx): HxRequest,
) -> impl IntoResponse {
    if is_htmx {
        templates::user_edit_form(None, &username).into_response()
    } else {
        StatusCode::NOT_FOUND.into_response()
    }
}

async fn user_edit_handler(
    session: Session<SessionNullPool>,
    Path(username): Path<String>,
    State(pool): State<PgPool>,
    HxRequest(is_htmx): HxRequest,
    mut multipart: Multipart,
) -> impl IntoResponse {
    let Some(user) = session.get::<database::User>("user") else {
        return StatusCode::FORBIDDEN.into_response();
    };
    if !user.is_admin && user.username != username {
        return StatusCode::FORBIDDEN.into_response();
    }
    let mut new_username = None;
    let mut new_avatar = None;
    let mut new_password1 = None;
    let mut new_password2 = None;
    let mut clear_avatar = false;
    while let Some(field) = multipart.next_field().await.unwrap() {
        if let Some(field_name) = field.name() {
            if field_name == "avatar" {
                if let Some(content_type) = field.content_type() {
                    if !content_type.starts_with("image/") {
                        return if is_htmx {
                            templates::user_edit_form(
                                Some(&database::DatabaseError::NotValidImage.to_string()),
                                &username,
                            )
                            .into_response()
                        } else {
                            StatusCode::UNPROCESSABLE_ENTITY.into_response()
                        };
                    }
                    if let Ok(bytes) = field.bytes().await {
                        new_avatar = Some(bytes);
                    }
                }
            } else if field_name == "username" {
                if let Ok(text) = field.text().await {
                    new_username = Some(text);
                }
            } else if field_name == "password1" {
                if let Ok(text) = field.text().await {
                    new_password1 = Some(text);
                }
            } else if field_name == "password2" {
                if let Ok(text) = field.text().await {
                    new_password2 = Some(text);
                }
            } else if field_name == "clear_avatar" {
                clear_avatar = true;
            }
        }
    }
    if new_username.is_none() {
        return if is_htmx {
            templates::user_edit_form(
                Some(&database::DatabaseError::EmptyFields.to_string()),
                &username,
            )
            .into_response()
        } else {
            StatusCode::UNPROCESSABLE_ENTITY.into_response()
        };
    }
    if let Err(err) = database::edit_user(
        &pool,
        &username,
        new_username.as_deref(),
        if new_avatar.is_none() && clear_avatar {
            Some(false)
        } else {
            new_avatar.as_ref().map(|_| true)
        },
        new_password1.as_deref(),
        new_password2.as_deref(),
    )
    .await
    {
        return if is_htmx {
            templates::user_edit_form(Some(&err.to_string()), &username).into_response()
        } else {
            StatusCode::UNAUTHORIZED.into_response()
        };
    };
    if clear_avatar {
        if try_exists("static/images/avatars/".to_owned() + &username)
            .await
            .unwrap_or(false)
        {
            remove_file("static/images/avatars/".to_owned() + &username)
                .await
                .unwrap()
        }
    }
    if let Some(new_username) = &new_username {
        if try_exists("static/images/avatars/".to_owned() + &username)
            .await
            .unwrap_or(false)
        {
            rename(
                "static/images/avatars/".to_owned() + &username,
                "static/images/avatars/".to_owned() + &new_username,
            )
            .await
            .unwrap();
        }
    }
    if let Some(new_avatar) = new_avatar {
        let mut file = File::create(
            "static/images/avatars/".to_owned() + new_username.as_ref().unwrap_or(&username),
        )
        .await
        .unwrap();
        file.write_all(&new_avatar).await.unwrap();
    }
    if user.username == username {
        session.set(
            "user",
            database::get_user(&pool, &new_username.as_ref().unwrap_or(&username))
                .await
                .unwrap(),
        )
    }
    if is_htmx {
        (
            HxLocation {
                uri: ("/users/".to_owned() + &new_username.unwrap_or(username))
                    .try_into()
                    .unwrap(),
            },
            (),
        )
            .into_response()
    } else {
        StatusCode::OK.into_response()
    }
}

async fn item_edit_form_handler(
    State(pool): State<PgPool>,
    Path(locator): Path<String>,
    HxRequest(is_htmx): HxRequest,
) -> impl IntoResponse {
    if is_htmx {
        if let Ok(Some(item)) = database::get_item(&pool, &locator).await {
            templates::item_form(
                &("/items/".to_owned() + &locator + "/edit"),
                "Edit item",
                None,
                Some(&item.title),
                Some(&item.locator),
                Some(&item.description),
            )
            .into_response()
        } else {
            StatusCode::NOT_FOUND.into_response()
        }
    } else {
        StatusCode::NOT_FOUND.into_response()
    }
}

async fn item_edit_handler(
    session: Session<SessionNullPool>,
    Path(locator): Path<String>,
    State(pool): State<PgPool>,
    HxRequest(is_htmx): HxRequest,
    mut multipart: Multipart,
) -> impl IntoResponse {
    if let Some(user) = session.get::<database::User>("user") {
        if !user.is_admin {
            return StatusCode::FORBIDDEN.into_response();
        }
    } else {
        return StatusCode::FORBIDDEN.into_response();
    }
    let mut new_title = None;
    let mut new_locator = None;
    let mut new_description = None;
    let mut new_image = None;
    while let Some(field) = multipart.next_field().await.unwrap() {
        if let Some(field_name) = field.name() {
            if field_name == "image" {
                if let Some(content_type) = field.content_type() {
                    if !content_type.starts_with("image/") {
                        return if is_htmx {
                            templates::item_form(
                                &("/items/".to_owned() + &locator + "/edit"),
                                "Edit item",
                                Some(&database::DatabaseError::NotValidImage.to_string()),
                                None,
                                None,
                                None,
                            )
                            .into_response()
                        } else {
                            StatusCode::UNPROCESSABLE_ENTITY.into_response()
                        };
                    }
                    if let Ok(bytes) = field.bytes().await {
                        new_image = Some(bytes);
                    }
                }
            } else if field_name == "title" {
                if let Ok(text) = field.text().await {
                    new_title = Some(text);
                }
            } else if field_name == "description" {
                if let Ok(text) = field.text().await {
                    new_description = Some(text);
                }
            } else if field_name == "locator" {
                if let Ok(text) = field.text().await {
                    new_locator = Some(text);
                }
            }
        }
    }
    if new_locator.is_none() || new_title.is_none() || new_description.is_none() {
        return if is_htmx {
            templates::item_form(
                &("/items/".to_owned() + &locator + "/edit"),
                "Edit item",
                Some(&database::DatabaseError::EmptyFields.to_string()),
                None,
                None,
                None,
            )
            .into_response()
        } else {
            StatusCode::UNPROCESSABLE_ENTITY.into_response()
        };
    }
    if let Err(err) = database::edit_item(
        &pool,
        &locator,
        new_locator.as_deref(),
        new_title.as_deref(),
        new_description.as_deref(),
    )
    .await
    {
        return if is_htmx {
            templates::item_form(
                &("/items/".to_owned() + &locator + "/edit"),
                "Edit item",
                Some(&err.to_string()),
                None,
                None,
                None,
            )
            .into_response()
        } else {
            StatusCode::UNAUTHORIZED.into_response()
        };
    };
    if let Some(new_locator) = &new_locator {
        rename(
            "static/images/items/".to_owned() + &locator,
            "static/images/items/".to_owned() + &new_locator,
        )
        .await
        .unwrap();
    }
    if let Some(new_image) = new_image {
        let mut file = File::create(
            "static/images/items/".to_owned() + new_locator.as_ref().unwrap_or(&locator),
        )
        .await
        .unwrap();
        file.write_all(&new_image).await.unwrap();
    }
    if is_htmx {
        (
            HxLocation {
                uri: ("/items/".to_owned() + &new_locator.unwrap_or(locator))
                    .try_into()
                    .unwrap(),
            },
            (),
        )
            .into_response()
    } else {
        StatusCode::OK.into_response()
    }
}

async fn item_add_form_handler(HxRequest(is_htmx): HxRequest) -> impl IntoResponse {
    if is_htmx {
        templates::item_form("/items/add", "Add item", None, None, None, None).into_response()
    } else {
        StatusCode::NOT_FOUND.into_response()
    }
}

async fn item_add_handler(
    session: Session<SessionNullPool>,
    State(pool): State<PgPool>,
    HxRequest(is_htmx): HxRequest,
    HxCurrentUrl(current_url): HxCurrentUrl,
    mut multipart: Multipart,
) -> impl IntoResponse {
    if let Some(user) = session.get::<database::User>("user") {
        if !user.is_admin {
            return StatusCode::FORBIDDEN.into_response();
        }
    } else {
        return StatusCode::FORBIDDEN.into_response();
    }
    let mut title = None;
    let mut locator = None;
    let mut description = None;
    let mut image = None;
    while let Some(field) = multipart.next_field().await.unwrap() {
        if let Some(field_name) = field.name() {
            if field_name == "image" {
                if let Some(content_type) = field.content_type() {
                    if !content_type.starts_with("image/") {
                        return if is_htmx {
                            templates::item_form(
                                "/items/add",
                                "Add item",
                                Some(&database::DatabaseError::NotValidImage.to_string()),
                                None,
                                None,
                                None,
                            )
                            .into_response()
                        } else {
                            StatusCode::UNPROCESSABLE_ENTITY.into_response()
                        };
                    }
                    if let Ok(bytes) = field.bytes().await {
                        image = Some(bytes);
                    }
                }
            } else if field_name == "title" {
                if let Ok(text) = field.text().await {
                    title = Some(text);
                }
            } else if field_name == "description" {
                if let Ok(text) = field.text().await {
                    description = Some(text);
                }
            } else if field_name == "locator" {
                if let Ok(text) = field.text().await {
                    locator = Some(text);
                }
            }
        }
    }
    if locator.is_none() || image.is_none() || title.is_none() || description.is_none() {
        return if is_htmx {
            templates::item_form(
                "/items/add",
                "Add item",
                Some(&database::DatabaseError::EmptyFields.to_string()),
                None,
                None,
                None,
            )
            .into_response()
        } else {
            StatusCode::UNPROCESSABLE_ENTITY.into_response()
        };
    }
    let locator = locator.unwrap();
    let image = image.unwrap();
    let title = title.unwrap();
    let description = description.unwrap();
    if let Err(err) = database::add_item(&pool, &locator, &title, &description).await {
        return if is_htmx {
            templates::item_form(
                "/items/add",
                "Add item",
                Some(&err.to_string()),
                None,
                None,
                None,
            )
            .into_response()
        } else {
            StatusCode::UNAUTHORIZED.into_response()
        };
    };
    let mut file = File::create("static/images/items/".to_owned() + &locator)
        .await
        .unwrap();
    file.write_all(&image).await.unwrap();
    if is_htmx {
        (
            HxLocation {
                uri: current_url.unwrap(),
            },
            (),
        )
            .into_response()
    } else {
        StatusCode::OK.into_response()
    }
}

async fn login_form_handler(HxRequest(is_htmx): HxRequest) -> impl IntoResponse {
    if is_htmx {
        templates::login_form(None).into_response()
    } else {
        StatusCode::NOT_FOUND.into_response()
    }
}

async fn register_form_handler(HxRequest(is_htmx): HxRequest) -> impl IntoResponse {
    if is_htmx {
        templates::register_form(None).into_response()
    } else {
        StatusCode::NOT_FOUND.into_response()
    }
}

#[derive(Deserialize)]
struct Login {
    username: String,
    password: String,
}

async fn login_handler(
    State(pool): State<PgPool>,
    session: Session<SessionNullPool>,
    HxRequest(is_htmx): HxRequest,
    HxCurrentUrl(current_url): HxCurrentUrl,
    form: Form<Login>,
) -> impl IntoResponse {
    match database::login_user(&pool, &form.username, &form.password).await {
        Ok(user) => {
            session.set("user", &user);
            if is_htmx {
                (
                    HxLocation {
                        uri: current_url.unwrap(),
                    },
                    templates::logged_in(&user),
                )
                    .into_response()
            } else {
                StatusCode::OK.into_response()
            }
        }
        Err(e) => {
            if is_htmx {
                templates::login_form(Some(&e.to_string())).into_response()
            } else {
                StatusCode::UNAUTHORIZED.into_response()
            }
        }
    }
}

#[derive(Deserialize)]
struct Register {
    username: String,
    password1: String,
    password2: String,
}

async fn register_handler(
    State(pool): State<PgPool>,
    session: Session<SessionNullPool>,
    HxRequest(is_htmx): HxRequest,
    HxCurrentUrl(current_url): HxCurrentUrl,
    form: Form<Register>,
) -> impl IntoResponse {
    match database::register_user(&pool, &form.username, &form.password1, &form.password2).await {
        Ok(user) => {
            session.set("user", &user);
            if is_htmx {
                (
                    HxLocation {
                        uri: current_url.unwrap(),
                    },
                    templates::logged_in(&user),
                )
                    .into_response()
            } else {
                StatusCode::OK.into_response()
            }
        }
        Err(e) => {
            if is_htmx {
                templates::register_form(Some(&e.to_string())).into_response()
            } else {
                StatusCode::UNAUTHORIZED.into_response()
            }
        }
    }
}

async fn logout_handler(
    session: Session<SessionNullPool>,
    HxCurrentUrl(current_url): HxCurrentUrl,
    HxRequest(is_htmx): HxRequest,
) -> impl IntoResponse {
    session.destroy();
    if is_htmx {
        (
            HxLocation {
                uri: current_url.unwrap(),
            },
            templates::login_button(),
        )
            .into_response()
    } else {
        StatusCode::OK.into_response()
    }
}
