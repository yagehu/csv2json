use std::{net::TcpListener, time::Duration};

use sqlx::postgres::{PgConnectOptions, PgPoolOptions, PgSslMode};

use csv2json::{logging, server};

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    logging::init(logging::get_subscriber(
        "csv2json".into(),
        "info".into(),
        std::io::stdout,
    ));

    let listener = TcpListener::bind("localhost:8000")?;
    let connect_options = PgConnectOptions::new()
        .host("localhost")
        .username("csv2json")
        .password("password")
        .port(5432)
        .ssl_mode(PgSslMode::Disable)
        .database("csv2json");
    let db_conn_pool = PgPoolOptions::new()
        .acquire_timeout(Duration::from_secs(2))
        .connect_with(connect_options)
        .await
        .expect("Failed to connect to Postgres.");

    server::run(listener, db_conn_pool)?.await
}
