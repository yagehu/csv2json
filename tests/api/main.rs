use std::net::TcpListener;

use once_cell::sync::Lazy;
use serde::Deserialize;
use sqlx::{
    postgres::{PgConnectOptions, PgConnection, PgPool, PgSslMode},
    Connection as _, Executor as _,
};
use uuid::Uuid;

use csv2json::{logging, server};

mod create_document;
mod get_document;

static TRACING: Lazy<()> = Lazy::new(|| {
    let default_filter_level = "info".to_owned();
    let subscriber_name = "test".to_string();

    if std::env::var("C2J_TEST_LOG").is_ok() {
        let subscriber =
            logging::get_subscriber(subscriber_name, default_filter_level, std::io::stdout);
        logging::init(subscriber);
    } else {
        let subscriber =
            logging::get_subscriber(subscriber_name, default_filter_level, std::io::sink);
        logging::init(subscriber);
    }
});

pub struct TestApp {
    pub address: String,
    pub client: reqwest::Client,
}

impl TestApp {
    pub async fn spawn() -> Self {
        Lazy::force(&TRACING);

        let database_name = Uuid::new_v4().to_string();
        let listener = TcpListener::bind("127.0.0.1:0").expect("Failed to bind random port.");
        let port = listener.local_addr().unwrap().port();
        let address = format!("http://127.0.0.1:{}", port);

        let connect_options = PgConnectOptions::new()
            .host("localhost")
            .username("csv2json")
            .password("password")
            .port(5432)
            .ssl_mode(PgSslMode::Disable);
        let mut conn = PgConnection::connect_with(&connect_options)
            .await
            .expect("Failed to connect to Postgres");

        conn.execute(format!(r#"CREATE DATABASE "{}";"#, database_name).as_str())
            .await
            .expect("Failed to create database.");

        let pool = PgPool::connect_with(connect_options.database(&database_name))
            .await
            .expect("Failed to connect to Postgres.");

        sqlx::migrate!()
            .run(&pool)
            .await
            .expect("Failed to migrate the database.");

        let server = server::run(listener, pool).expect("Failed to run server.");
        let _ = tokio::spawn(server);

        Self {
            address,
            client: reqwest::Client::default(),
        }
    }
}

#[derive(Deserialize, Debug)]
pub struct DocumentResponse {
    pub id: Uuid,
    pub content: Vec<Vec<String>>,
}
