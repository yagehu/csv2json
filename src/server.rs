use std::net::TcpListener;

use actix_web::{dev::Server, web, HttpServer};
use sqlx::PgPool;
use tracing_actix_web::TracingLogger;

use crate::handler;

pub fn run(listener: TcpListener, db_conn_pool: PgPool) -> Result<Server, std::io::Error> {
    let db_conn_pool = web::Data::new(db_conn_pool);
    let server = HttpServer::new(move || {
        actix_web::App::new()
            .wrap(TracingLogger::default())
            .service(
                web::scope("/document")
                    .route("", web::post().to(handler::create_document))
                    .route("/{id}", web::get().to(handler::get_document)),
            )
            .app_data(db_conn_pool.clone())
    })
    .listen(listener)?
    .run();

    Ok(server)
}
