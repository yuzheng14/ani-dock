use ani_dock_server::router::get_router;
use axum::serve;
use tracing_subscriber::EnvFilter;

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")),
        )
        .with_target(true)
        .with_line_number(true)
        .init();

    let app = get_router();

    let listener = tokio::net::TcpListener::bind("127.0.0.1:6789")
        .await
        .expect("could not start server");
    tracing::info!("server started, listener at: 127.0.0.1:6789");

    serve(listener, app).await.expect("server serve error");
}
