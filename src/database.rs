use argon2::{
    password_hash::{rand_core::OsRng, SaltString},
    Argon2, PasswordHash, PasswordHasher, PasswordVerifier,
};
use passwords::{analyzer, scorer};
use regex::Regex;
use serde::{Deserialize, Serialize};
use sqlx::{query, query_as, query_scalar, types::chrono::NaiveDateTime, Decode, PgPool};
use std::{error::Error, fmt::Display, ops::Deref};

#[derive(Debug)]
pub enum DatabaseError {
    InternalError(Box<dyn Error>),
    IncorrectCredentials,
    EmptyFields,
    PasswordsDiffer,
    WeakPassword,
    DuplicateUser,
    DuplicateItem,
    IllegalUsername,
    NotValidImage,
    IllegalLocator
}

impl Display for DatabaseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DatabaseError::InternalError(_) => write!(f, "Internal server error!"),
            DatabaseError::IncorrectCredentials => write!(f, "Incorrect login credentials!"),
            DatabaseError::EmptyFields => write!(f, "Some fields are empty!"),
            DatabaseError::PasswordsDiffer => write!(f, "Passwords do not match!"),
            DatabaseError::DuplicateUser => write!(f, "User with this username already exists!"),
            DatabaseError::WeakPassword => write!(f, "Password is not strong enough!"),
            DatabaseError::IllegalUsername => write!(
                f,
                "Only alphanumerical characters and underscores are allowed in usernames!"
            ),
            DatabaseError::DuplicateItem => write!(f, "Item with this locator already exists!"),
            DatabaseError::NotValidImage => write!(f, "Uploaded file is not a valid image"),
            DatabaseError::IllegalLocator => write!(f,
                "Only alphanumerical characters and underscores are allowed in item locator!"
            ),
        }
    }
}

impl std::error::Error for DatabaseError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            DatabaseError::InternalError(e) => Some(e.deref()),
            _ => None,
        }
    }
}

pub async fn login_user(
    pool: &PgPool,
    username: &str,
    password: &str,
) -> Result<User, DatabaseError> {
    if username.trim().is_empty() || password.trim().is_empty() {
        return Err(DatabaseError::EmptyFields);
    }
    let result = query!(
        "SELECT password_hash, is_admin, avatar_hue, has_avatar FROM users WHERE username=$1 LIMIT 1",
        username
    )
    .fetch_one(pool)
    .await
    .map_err(|e| {
        if let sqlx::Error::RowNotFound = e {
            DatabaseError::IncorrectCredentials
        } else {
            DatabaseError::InternalError(Box::new(e))
        }
    })?;
    let password_hash = PasswordHash::new(&result.password_hash)
        .map_err(|e| DatabaseError::InternalError(Box::new(e)))?;
    Argon2::default()
        .verify_password(password.as_bytes(), &password_hash)
        .map_err(|e| {
            if let argon2::password_hash::Error::Password = e {
                DatabaseError::IncorrectCredentials
            } else {
                DatabaseError::InternalError(Box::new(e))
            }
        })?;
    Ok(User {
        username: username.to_owned(),
        is_admin: result.is_admin,
        avatar_hue: result.avatar_hue,
        has_avatar: result.has_avatar
    })
}

pub async fn register_user(
    pool: &PgPool,
    username: &str,
    password1: &str,
    password2: &str,
) -> Result<User, DatabaseError> {
    if username.trim().is_empty() || password1.trim().is_empty() || password2.trim().is_empty() {
        return Err(DatabaseError::EmptyFields);
    }
    if !Regex::new(r"^\w+$").unwrap().is_match(username) {
        return Err(DatabaseError::IllegalUsername);
    }
    if password1 != password2 {
        return Err(DatabaseError::PasswordsDiffer);
    }
    if scorer::score(&analyzer::analyze(password1)) < 80.0 {
        return Err(DatabaseError::WeakPassword);
    }
    let password_hash = Argon2::default()
        .hash_password(password1.as_bytes(), &SaltString::generate(&mut OsRng))
        .map_err(|e| DatabaseError::InternalError(Box::new(e)))?
        .to_string();
    query!(
        "INSERT INTO users (username, password_hash) VALUES ($1, $2)",
        username,
        password_hash
    )
    .execute(pool)
    .await
    .map_err(|e| {
        if let sqlx::Error::Database(e) = e {
            if e.is_unique_violation() {
                DatabaseError::DuplicateUser
            } else {
                DatabaseError::InternalError(Box::new(e))
            }
        } else {
            DatabaseError::InternalError(Box::new(e))
        }
    })?;
    login_user(pool, username, password1).await
}

pub struct Page<T> {
    pub target: String,
    pub items: Vec<T>,
    pub current_page: i32,
    pub number_of_pages: i32,
    pub query: Option<String>,
}

#[derive(Decode)]
pub struct Item {
    pub locator: String,
    pub title: String,
    pub description: String,
    pub score: f32,
    pub review_count: i64,
    pub rank: i64,
    pub popularity: i64
}

pub async fn get_item(pool: &PgPool, locator: &str) -> Result<Option<Item>, DatabaseError> {
    match query_as!(
        Item,
        r#"SELECT locator AS "locator!", title AS "title!", description AS "description!", score AS "score!", review_count AS "review_count!", rank AS "rank!", popularity AS "popularity!" FROM items_score WHERE locator = $1 LIMIT 1"#,
        locator
    )
    .fetch_one(pool)
    .await
    {
        Ok(i) => Ok(Some(i)),
        Err(e) => match e {
            sqlx::Error::RowNotFound => Ok(None),
            _ => Err(DatabaseError::InternalError(Box::new(e))),
        },
    }
}

pub async fn get_items(
    pool: &PgPool,
    page_number: Option<i32>,
    query: Option<&str>,
) -> Result<Option<Page<Item>>, DatabaseError> {
    let page_number = page_number.unwrap_or(0);
    let number_of_pages = if let Some(query) = query {
        (query_scalar!("SELECT COUNT(*) FROM items WHERE title % $1", query)
            .fetch_one(pool)
            .await
            .map_err(|e| DatabaseError::InternalError(Box::new(e)))?
            .unwrap_or_default() as usize)
            .div_ceil(12) as i32
    } else {
        (query_scalar!("SELECT COUNT(*) FROM items")
            .fetch_one(pool)
            .await
            .map_err(|e| DatabaseError::InternalError(Box::new(e)))?
            .unwrap_or_default() as usize)
            .div_ceil(12) as i32
    };
    if (0..number_of_pages).contains(&page_number) {
        let page = if let Some(query) = query {
            query_as!(
            Item,
            r#"SELECT locator AS "locator!", title AS "title!", description AS "description!", score AS "score!", review_count AS "review_count!", rank AS "rank!", popularity AS "popularity!" FROM items_score WHERE title % $1 ORDER BY SIMILARITY(title,$1) DESC, score DESC LIMIT 12 OFFSET 12 * $2"#,
            query,
            page_number
            )
            .fetch_all(pool)
            .await
            .map_err(|e| DatabaseError::InternalError(Box::new(e)))?
        } else {
            query_as!(
                Item,
                r#"SELECT locator AS "locator!", title AS "title!", description AS "description!", score AS "score!", review_count AS "review_count!", rank AS "rank!", popularity AS "popularity!" FROM items_score ORDER BY score DESC LIMIT 12 OFFSET 12 * $1"#,
                page_number
            )
            .fetch_all(pool)
            .await
            .map_err(|e| DatabaseError::InternalError(Box::new(e)))?
        };
        Ok(Some(Page {
            target: "/items".to_owned(),
            items: page,
            current_page: page_number,
            number_of_pages,
            query: query.map(str::to_owned),
        }))
    } else {
        Ok(None)
    }
}

#[derive(Serialize, Deserialize, Decode)]
pub struct User {
    pub username: String,
    pub is_admin: bool,
    pub avatar_hue: i16,
    pub has_avatar: bool
}

pub async fn get_user(pool: &PgPool, username: &str) -> Result<Option<User>, DatabaseError> {
    match query_as!(
        User,
        "SELECT username, is_admin, avatar_hue, has_avatar FROM users WHERE username = $1 LIMIT 1",
        username
    )
    .fetch_one(pool)
    .await
    {
        Ok(u) => Ok(Some(u)),
        Err(e) => match e {
            sqlx::Error::RowNotFound => Ok(None),
            _ => Err(DatabaseError::InternalError(Box::new(e))),
        },
    }
}

pub async fn get_users(
    pool: &PgPool,
    page_number: Option<i32>,
    query: Option<&str>,
) -> Result<Option<Page<User>>, DatabaseError> {
    let page_number = page_number.unwrap_or(0);
    let number_of_pages = if let Some(query) = query {
        (query_scalar!(
            "SELECT COALESCE(COUNT(*), 0) FROM users WHERE username % $1",
            query
        )
        .fetch_one(pool)
        .await
        .map_err(|e| DatabaseError::InternalError(Box::new(e)))?
        .unwrap_or_default() as usize)
            .div_ceil(12) as i32
    } else {
        (query_scalar!("SELECT COUNT(*) FROM users")
            .fetch_one(pool)
            .await
            .map_err(|e| DatabaseError::InternalError(Box::new(e)))?
            .unwrap_or_default() as usize)
            .div_ceil(12) as i32
    };
    if (0..number_of_pages).contains(&page_number) {
        let page = if let Some(query) = query {
            query_as!(
            User,
            "SELECT username, is_admin, avatar_hue, has_avatar FROM users WHERE username % $1 ORDER BY SIMILARITY(username,$1) DESC LIMIT 12 OFFSET 12 * $2",
            query,
            page_number
            )
            .fetch_all(pool)
            .await
            .map_err(|e| DatabaseError::InternalError(Box::new(e)))?
        } else {
            query_as!(
                User,
                "SELECT username, is_admin, avatar_hue, has_avatar FROM users LIMIT 12 OFFSET 12 * $1",
                page_number
            )
            .fetch_all(pool)
            .await
            .map_err(|e| DatabaseError::InternalError(Box::new(e)))?
        };
        Ok(Some(Page {
            target: "/users".to_owned(),
            items: page,
            current_page: page_number,
            number_of_pages,
            query: query.map(str::to_owned),
        }))
    } else {
        Ok(None)
    }
}

pub async fn rate_item(
    pool: &PgPool,
    username: &str,
    item_locator: &str,
    rating: i16,
) -> Result<(), DatabaseError> {
    let rating = rating.max(1).min(10);
    if let Err(e)=query!("INSERT INTO reviews(item_id, user_id, rating) VALUES((SELECT id FROM items WHERE locator=$1 LIMIT 1), (SELECT id FROM users WHERE username=$2 LIMIT 1), $3)",item_locator,username,rating).execute(pool).await {
        match e {
            sqlx::Error::Database(e) => if e.is_unique_violation(){ 
                query!("UPDATE reviews SET rating=$3, date=now() WHERE item_id=(SELECT id FROM items WHERE locator=$1 LIMIT 1) AND user_id=(SELECT id FROM users WHERE username=$2 LIMIT 1)",item_locator,username,rating).execute(pool).await.map(|_|()) .map_err(|e| DatabaseError::InternalError(Box::new(e)))
            } else {
                Err(DatabaseError::InternalError(Box::new(e)))
            },
            _ => Err(DatabaseError::InternalError(Box::new(e)))
        }
    } else {
        Ok(())
    }
}

pub async fn remove_review(pool: &PgPool, locator:&str, username: &str) ->Result<(), DatabaseError>{
    query!("DELETE FROM reviews WHERE item_id=(SELECT id FROM items WHERE locator=$1 LIMIT 1) AND user_id=(SELECT id FROM users WHERE username=$2)",locator, username).execute(pool).await.map(|_|()).map_err(|e|DatabaseError::InternalError(Box::new(e)))
}

pub async fn get_item_rating(pool: &PgPool, locator:&str, username: &str) -> Result<Option<i16>, DatabaseError> {
    match query_scalar!("SELECT rating FROM reviews WHERE item_id=(SELECT id FROM items WHERE locator=$1 LIMIT 1) AND user_id=(SELECT id FROM users WHERE username=$2) LIMIT 1",locator,username).fetch_one(pool).await {
        Ok(r) => Ok(Some(r)),
        Err(e) => match e {
            sqlx::Error::RowNotFound => Ok(None),
            _ => Err(DatabaseError::InternalError(Box::new(e))),
        },
    }
}

pub struct RatingItem
{
    pub user: User,
    pub rating: i16,
    pub date: NaiveDateTime
}

pub async fn get_item_ratings(pool: &PgPool, page_number: Option<i32>, locator: &str)
 -> Result<Option<Page<RatingItem>>, DatabaseError> {
    let page_number = page_number.unwrap_or(0);
    let number_of_pages = 
        (query_scalar!("SELECT COUNT(*) FROM reviews WHERE item_id = (SELECT id FROM items WHERE locator = $1 LIMIT 1)", locator)
            .fetch_one(pool)
            .await
            .map_err(|e| DatabaseError::InternalError(Box::new(e)))?
            .unwrap_or_default() as usize)
            .div_ceil(3) as i32;
    if (0..number_of_pages).contains(&page_number) {
        let page = 
    query_as!(RatingItem, r#"SELECT (u.username, u.is_admin, u.avatar_hue, u.has_avatar) AS "user!: User", rating, date FROM reviews r JOIN users u ON r.user_id = u.id WHERE r.item_id = (SELECT id FROM items WHERE locator = $1 LIMIT 1) ORDER BY date DESC LIMIT 3 OFFSET 3 * $2"#,locator,page_number).fetch_all(pool).await.map_err(|e|DatabaseError::InternalError(Box::new(e)))?;
        Ok(Some(Page {
            target: "/items/".to_owned() + &locator,
            items: page,
            current_page: page_number,
            number_of_pages,
            query: None,
        }))
    } else {
        Ok(None)
    }
}

pub struct RatingUser
{
    pub item: Item,
    pub rating: i16,
    pub date: NaiveDateTime
}

pub async fn get_user_ratings(pool: &PgPool, page_number: Option<i32>, username: &str)
 -> Result<Option<Page<RatingUser>>, DatabaseError> {
    let page_number = page_number.unwrap_or(0);
    let number_of_pages = 
        (query_scalar!("SELECT COUNT(*) FROM reviews WHERE user_id = (SELECT id FROM users WHERE username = $1 LIMIT 1)", username)
            .fetch_one(pool)
            .await
            .map_err(|e| DatabaseError::InternalError(Box::new(e)))?
            .unwrap_or_default() as usize)
            .div_ceil(3) as i32;
    if (0..number_of_pages).contains(&page_number) {
        let page = 
    query_as!(RatingUser, r#"SELECT (i.locator, i.title, i.description, i.score, i.review_count, i.rank, i.popularity) AS "item!: Item", rating, date FROM reviews r JOIN items_score i ON r.item_id = i.id WHERE r.user_id = (SELECT id FROM users WHERE username = $1 LIMIT 1) ORDER BY date DESC LIMIT 3 OFFSET 3 * $2"#,username,page_number).fetch_all(pool).await.map_err(|e|DatabaseError::InternalError(Box::new(e)))?;
        Ok(Some(Page {
            target: "/users/".to_owned() + &username,
            items: page,
            current_page: page_number,
            number_of_pages,
            query: None,
        }))
    } else {
        Ok(None)
    }
}

pub async fn add_item(pool: &PgPool, locator:&str, title:&str, description: &str) -> Result<(),DatabaseError>{
    if locator.trim().is_empty() || title.trim().is_empty() || description.trim().is_empty() {
        return Err(DatabaseError::EmptyFields);
    }
    if !Regex::new(r"^\w+$").unwrap().is_match(locator) {
        return Err(DatabaseError::IllegalLocator);
    }
    query!("INSERT INTO items(locator, title, description) VALUES($1, $2, $3)", locator, title, description).execute(pool).await.map(|_|()).map_err(|e|match e{
        sqlx::Error::Database(e) => if e.is_unique_violation() {
            DatabaseError::DuplicateItem
        } else {
            DatabaseError::InternalError(Box::new(e))
        },
        _ => DatabaseError::InternalError(Box::new(e)),
    })
}

pub async fn remove_item(pool: &PgPool, locator:&str) ->Result<(), DatabaseError>{
    query!("DELETE FROM items WHERE locator=$1",locator).execute(pool).await.map(|_|()).map_err(|e|DatabaseError::InternalError(Box::new(e)))
}

pub async fn edit_item(pool: &PgPool,locator: &str, new_locator:Option<&str>, new_title:Option<&str>, new_description: Option<&str>) -> Result<(),DatabaseError>{
    if new_locator.is_some_and(|l|l.trim().is_empty()) || new_title.is_some_and(|t| t.trim().is_empty()) || new_description.is_some_and(|d|d.trim().is_empty()) {
        return Err(DatabaseError::EmptyFields);
    }
    if new_locator.is_some_and(|l|!Regex::new(r"^\w+$").unwrap().is_match(l)) {
        return Err(DatabaseError::IllegalLocator);
    }
    query!("UPDATE items SET locator = COALESCE($1,locator), title = COALESCE($2,title), description = COALESCE($3, description) WHERE locator=$4",new_locator,new_title,new_description,locator).execute(pool).await.map(|_|()).map_err(|e|match e{
        sqlx::Error::Database(e) => if e.is_unique_violation() {
            DatabaseError::DuplicateItem
        } else {
            DatabaseError::InternalError(Box::new(e))
        },
        _ => DatabaseError::InternalError(Box::new(e)),
    }
    )
}

pub async fn remove_user(pool: &PgPool, username:&str) ->Result<(), DatabaseError>{
    query!("DELETE FROM users WHERE username=$1", username).execute(pool).await.map(|_|()).map_err(|e|DatabaseError::InternalError(Box::new(e)))
}

pub async fn edit_user(pool: &PgPool, username: &str, new_username:Option<&str>,has_avatar:Option<bool>, new_password1:Option<&str>, new_password2:Option<&str>) -> Result<(),DatabaseError>{
    if new_username.is_some_and(|u|u.trim().is_empty()) {
        return Err(DatabaseError::EmptyFields);
    }
    if new_username.is_some_and(|u|!Regex::new(r"^\w+$").unwrap().is_match(u)) {
        return Err(DatabaseError::IllegalUsername);
    }
    let password_hash = if let Some(password1) = new_password1 {
        if let Some(password2) = new_password2 {
            if !password1.trim().is_empty() || !password2.trim().is_empty()
            {
                if password1 != password2 {
                    return Err(DatabaseError::PasswordsDiffer);
                }
                if scorer::score(&analyzer::analyze(password1)) < 80.0 {
                    return Err(DatabaseError::WeakPassword);
                }
                Some(Argon2::default() .hash_password(password1.as_bytes(), &SaltString::generate(&mut OsRng)) .map_err(|e| DatabaseError::InternalError(Box::new(e)))? .to_string())
            } else {
                None
            }
        } else {
            None
        }
    } else {
        None
    };
    query!("UPDATE users SET username = COALESCE($1, username), has_avatar = COALESCE($2, has_avatar), password_hash = COALESCE($3, password_hash) WHERE username = $4", new_username, has_avatar, password_hash, username).execute(pool).await.map(|_|()).map_err(|e|match e{
        sqlx::Error::Database(e) => if e.is_unique_violation() {
            DatabaseError::DuplicateItem
        } else {
            DatabaseError::InternalError(Box::new(e))
        },
        _ => DatabaseError::InternalError(Box::new(e)),
    }
    )
}
