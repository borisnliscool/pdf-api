use anyhow::Result;
use std::{fs};
use axum::response::{Response};
use axum::Router;
use axum::routing::post;
use headless_chrome::{Browser, LaunchOptionsBuilder};
use headless_chrome::types::PrintToPdfOptions;
use tokio::net::TcpListener;
use tower_http::cors::CorsLayer;
use tower_http::trace::TraceLayer;

fn print_page(page_path: &str) -> Result<Vec<u8>> {
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
        paper_width: None,
        paper_height: None,
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

async fn handle_post(body: String) -> Response {
    let cwd = std::env::current_dir().unwrap();
    let tmp_path = format!("{}/tmp/{}", cwd.as_path().to_str().unwrap(), uuid::Uuid::new_v4().to_string());

    fs::create_dir_all(&tmp_path).unwrap();
    fs::write(format!("{}/index.html", tmp_path), body).unwrap();

    let local_pdf = print_page(&format!("{}/index.html", tmp_path)).unwrap();

    let response =  axum::response::Response::builder()
        .header("Content-Type", "application/pdf")
        .header("Content-Disposition", "attachment; filename=\"file.pdf\"")
        .body(local_pdf.into())
        .unwrap();

    fs::remove_dir_all(&tmp_path).unwrap();

    response
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