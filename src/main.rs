use std::collections::HashMap;

use axum::{
    extract::{Query, State},
    response::IntoResponse,
    routing::post,
    Json, Router,
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::json;
use sqlx::{mysql::MySqlPoolOptions, FromRow, MySql, Pool, QueryBuilder};
use tower_http::trace::{DefaultMakeSpan, DefaultOnRequest, DefaultOnResponse, TraceLayer};

#[derive(Debug, Clone)]
pub struct AppState {
    pub pool: Pool<MySql>,
}

impl AppState {
    pub fn new(pool: Pool<MySql>) -> Self {
        Self { pool }
    }
}

#[derive(Debug, Serialize, Deserialize, FromRow)]
struct SampleEntity {
    id: u64,
    text: String,
    created_at: DateTime<Utc>,
}

async fn create_entity(
    State(state): State<AppState>,
    Json(params): Json<serde_json::Value>,
) -> Result<impl IntoResponse, String> {
    // { "text": "ユーザー入力値" } のJsonを受け取る
    let text: Option<String> = params.get("text").map(|value| value.to_string());
    if let Some(text) = text {
        QueryBuilder::new("INSERT INTO sample_entity (text) VALUES (")
            .push_bind(text)
            .push(")")
            .build()
            .execute(&state.pool)
            .await
            .map_err(|e| e.to_string())?;
    }
    Ok(())
}

async fn entity_list(
    State(state): State<AppState>,
    Query(params): Query<HashMap<String, String>>,
) -> Result<impl IntoResponse, String> {
    // クエリパラメータを抽出
    let page: Option<i32> = params
        .get("page")
        .and_then(|value| value.to_string().parse::<i32>().ok());
    let count: Option<i32> = params
        .get("count")
        .and_then(|value| value.to_string().parse::<i32>().ok());


    let count = count.unwrap_or(10); // 指定されない場合は10件ページャ
    let offset = count * page.unwrap_or(0); // ページ指定がない場合は先頭ページ

    // DB問い合わせ
    let entities: Vec<SampleEntity> =
        QueryBuilder::new("SELECT * FROM sample_entity ORDER BY id DESC LIMIT ")
            .push_bind(count)
            .push(" OFFSET ")
            .push_bind(offset)
            .build_query_as()
            .fetch_all(&state.pool)
            .await
            .map_err(|e| e.to_string())?;

    // { "list": [{"id":1, "text":"ユーザー入力値", "created_at": <unixtimestamp> }] }
    // の形式でレスポンス
    Ok(Json(json!({
        "list": entities
    })))
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 環境変数初期化
    dotenvy::dotenv().unwrap();

    // logger初期化
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::DEBUG)
        .with_ansi(true)
        .init();

    // DBへの接続プール作成
    let uri = std::env::var("DATABASE_URL")?;
    let pool = MySqlPoolOptions::new()
        .max_connections(4)
        .connect(&uri)
        .await?;

    // Router定義
    let app = Router::new()
        .route("/sample_entity", post(create_entity).get(entity_list))
        .layer(
            TraceLayer::new_for_http()
                .make_span_with(DefaultMakeSpan::new().level(tracing::Level::DEBUG))
                .on_request(DefaultOnRequest::new().level(tracing::Level::DEBUG))
                .on_response(DefaultOnResponse::new().level(tracing::Level::DEBUG)),
        )
        .with_state(AppState::new(pool));

    // サーバー起動
    let listener = tokio::net::TcpListener::bind("localhost:8000").await?;
    axum::serve(listener, app).await?;

    Ok(())
}
