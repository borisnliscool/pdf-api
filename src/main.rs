use anyhow::Result;
use axum::extract::Query;
use axum::response::Response;
use axum::routing::post;
use axum::{Json, Router};
use headless_chrome::types::PrintToPdfOptions;
use headless_chrome::{Browser, LaunchOptionsBuilder};
use serde::Serialize;
use std::collections::HashMap;
use std::fs;
use tokio::net::TcpListener;
use tower_http::cors::CorsLayer;
use tower_http::trace::TraceLayer;

#[derive(Serialize)]
enum ErrorType {
    BadRequest,
    Internal,
    FileSystem,
    Unknown,
}

#[derive(Serialize)]
struct Error {
    message: String,
    error_type: ErrorType,
}

fn print_page(page_path: &str, dimensions: (Option<f64>, Option<f64>)) -> Result<Vec<u8>> {
    let browser = Browser::new(
        LaunchOptionsBuilder::default()
            .headless(true)
            .sandbox(false) // required when running in a container
            .build()
            .unwrap(),
    )?;

    let tab = browser.new_tab()?;

    let pdf_options: Option<PrintToPdfOptions> = Some(PrintToPdfOptions {
        landscape: None,
        display_header_footer: None,
        print_background: Some(true),
        scale: None,
        paper_width: dimensions.0,
        paper_height: dimensions.1,
        margin_top: Some(0f64),
        margin_bottom: Some(0f64),
        margin_left: Some(0f64),
        margin_right: Some(0f64),
        page_ranges: None,
        ignore_invalid_page_ranges: None,
        header_template: None,
        footer_template: None,
        prefer_css_page_size: None,
        transfer_mode: None,
        generate_document_outline: None,
        generate_tagged_pdf: None,
    });

    let local_pdf = tab
        .navigate_to(&format!("file://{page_path}"))?
        .wait_until_navigated()?
        .print_to_pdf(pdf_options)?;

    Ok(local_pdf)
}

async fn handle_post(
    Query(params): Query<HashMap<String, String>>,
    body: String,
) -> Result<Response, Json<Error>> {
    if body.is_empty() {
        return Err(Json(Error {
            message: "Couldn't generate PDF, body is empty".to_string(),
            error_type: ErrorType::BadRequest,
        }));
    }

    let cwd = std::env::current_dir().map_err(|err| {
        Json(Error {
            message: err.to_string(),
            error_type: ErrorType::Internal,
        })
    })?;

    let tmp_path = format!(
        "{}/tmp/{}",
        cwd.as_path().to_str().unwrap(),
        uuid::Uuid::new_v4().to_string()
    );

    fs::create_dir_all(&tmp_path).map_err(|err| {
        Json(Error {
            message: err.to_string(),
            error_type: ErrorType::FileSystem,
        })
    })?;

    fs::write(format!("{}/index.html", tmp_path), body).map_err(|err| {
        Json(Error {
            message: err.to_string(),
            error_type: ErrorType::FileSystem,
        })
    })?;

    let width = params.get("width").and_then(|s| s.parse::<f64>().ok());
    let height = params.get("height").and_then(|s| s.parse::<f64>().ok());

    let local_pdf =
        print_page(&format!("{}/index.html", tmp_path), (width, height)).map_err(|err| {
            Json(Error {
                message: err.to_string(),
                error_type: ErrorType::Unknown,
            })
        })?;

    let response = axum::response::Response::builder()
        .header("Content-Type", "application/pdf")
        .header("Content-Disposition", "attachment; filename=\"file.pdf\"")
        .body(local_pdf.into())
        .map_err(|err| {
            Json(Error {
                message: err.to_string(),
                error_type: ErrorType::Unknown,
            })
        })?;

    let _ = fs::remove_dir_all(&tmp_path);
    Ok(response)
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::DEBUG)
        .init();
    let addr = std::net::SocketAddr::from(([0, 0, 0, 0], 3000));

    let listener = TcpListener::bind(addr).await.expect("Failed to bind");
    tracing::info!("Webserver listening on {}", addr);

    let server = Router::new()
        .route("/create", post(handle_post))
        .layer(TraceLayer::new_for_http())
        .layer(CorsLayer::permissive());

    axum::serve(listener, server)
        .await
        .expect("Failed to serve");

    Ok(())
}
