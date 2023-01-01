use crate::config::Config;
use crate::{get, sql};
use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::response::{Html, IntoResponse, Redirect};
use axum::routing::get;
use axum::Router;
use serde::{Deserialize, Serialize};
use std::net::SocketAddr;
use std::sync::Arc;

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Params {
    pub limit: Option<usize>,
    pub offset: Option<usize>,
    // TODO: this is a hack to allow for one PostgREST-style column filter
    pub table: Option<String>,
}

struct AppState {
    pub config: Config,
}

#[tokio::main]
pub async fn app(config: &Config) -> Result<String, String> {
    let shared_state = Arc::new(AppState {
        //TODO: use &config instead of config.clone()?
        config: config.clone(),
    });

    // build our application with a route
    let app = Router::new()
        // `GET /` goes to `root`
        .route("/", get(root))
        .route("/:table", get(table))
        .with_state(shared_state);

    // run our app with hyper
    // `axum::Server` is a re-export of `hyper::Server`
    let addr = SocketAddr::from(([127, 0, 0, 1], 3000));
    tracing::info!("listening on {}", addr);
    axum::Server::bind(&addr)
        .serve(app.into_make_service())
        .await
        .unwrap();

    let hello = String::from("Hello, world!");
    Ok(hello)
}

async fn root() -> impl IntoResponse {
    tracing::info!("request root");
    Redirect::permanent("/table")
}

async fn table(
    Path(path): Path<String>,
    params: Query<Params>,
    State(state): State<Arc<AppState>>,
) -> impl IntoResponse {
    tracing::info!("request table {:?} {:?}", path, params.0);
    let mut table = path.clone();
    let mut format = "html";
    if path.ends_with(".pretty.json") {
        table = path.replace(".pretty.json", "");
        format = "pretty.json";
    } else if path.ends_with(".json") {
        table = path.replace(".json", "");
        format = "json";
    }
    let select = sql::Select {
        table,
        limit: params.limit.unwrap_or_default(),
        offset: params.offset.unwrap_or_default(),
        // TODO: restore filters
        ..Default::default()
    };

    match get::get_rows(&state.config, &select, "page", &format).await {
        Ok(x) => match format {
            "html" => Html(x).into_response(),
            "json" => ([("content-type", "application/json; charset=utf-8")], x).into_response(),
            "pretty.json" => x.into_response(),
            _ => unreachable!("Unsupported format"),
        },
        Err(_) => (StatusCode::NOT_FOUND, Html("404 Not Found".to_string())).into_response(),
    }
}
