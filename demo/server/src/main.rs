use actix_web::{get, App, HttpResponse, HttpServer, Responder};
use aetheris_lib::translate;
use serde::Deserialize;

#[derive(Deserialize)]
struct TranslateParams {
    text: String,
    target: String,
}

#[get("/translate")]
async fn translate_handler(query: actix_web::web::Query<TranslateParams>) -> impl Responder {
    match translate(&query.text, &query.target) {
        Ok(result) => HttpResponse::Ok().json(serde_json::json!({"translation": result})),
        Err(e) => HttpResponse::InternalServerError().body(format!("Error: {}", e)),
    }
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    println!("Starting ASP demo server on http://127.0.0.1:8080");
    HttpServer::new(|| App::new().service(translate_handler))
        .bind("127.0.0.1:8080")?
        .run()
        .await
}
