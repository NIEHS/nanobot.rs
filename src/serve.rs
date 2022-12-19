use crate::{get, sql};
use axum::extract::{Path, RawQuery};
use axum::http::StatusCode;
use axum::response::{Html, IntoResponse, Redirect};
use axum::routing::get;
use axum::Router;
use std::net::SocketAddr;

#[tokio::main]
pub async fn main() -> Result<String, String> {
    // build our application with a route
    let app = Router::new()
        // `GET /` goes to `root`
        .route("/", get(root))
        .route("/:table", get(table));

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

async fn table(Path(path): Path<String>, RawQuery(query): RawQuery) -> impl IntoResponse {
    tracing::info!("request table {:?} {:?}", path, query);
    let mut table = path.clone();
    let mut format = "html";
    if path.ends_with(".pretty.json") {
        table = path.replace(".pretty.json", "");
        format = "pretty.json";
    } else if path.ends_with(".json") {
        table = path.replace(".json", "");
        format = "json";
    }
    let url = match query {
        Some(q) => format!("{}?{}", table, q),
        None => table.clone(),
    };
    tracing::info!("URL: {}", url);
    let select = sql::parse(&url);
    tracing::info!("select {:?}", select);
    match get::get_rows(".nanobot.db", &select, "page", &format).await {
        Ok(x) => match format {
            "html" => Html(x).into_response(),
            "json" => ([("content-type", "application/json; charset=utf-8")], x).into_response(),
            "pretty.json" => x.into_response(),
            _ => unreachable!("Unsupported format"),
        },
        Err(x) => {
            tracing::info!("Get Error: {:?}", x);
            (StatusCode::NOT_FOUND, Html("404 Not Found".to_string())).into_response()
        }
    }
}
