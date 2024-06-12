#![warn(clippy::pedantic)]

use actix_cors::Cors;
use actix_files::{Files, NamedFile};
use actix_web::{web, App, HttpRequest, HttpResponse, HttpServer, Responder};
use hemoglobin::cards::Card;
use hemoglobin::search::query_parser::query_parser;
use hemoglobin::search::QueryParams;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::{fs, io};
use tokio::sync::RwLock;

struct AppState {
    cards: Arc<RwLock<Vec<Card>>>,
}

#[derive(Serialize)]
#[serde(tag = "type")]
pub enum QueryResult<'a> {
    CardList { content: Vec<&'a Card> },
    Error { message: String },
}

#[derive(Deserialize)]
struct IdViewParam {
    id: String,
}

async fn serve_index(req: HttpRequest) -> io::Result<HttpResponse> {
    // Here, we ignore the request path and always serve index.html
    let file = NamedFile::open("./dist/index.html")?.use_last_modified(true);
    let response = file.into_response(&req);
    Ok(response)
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    let data = fs::read_to_string("cards.json").expect("Unable to read file");
    let cards: Vec<Card> = serde_json::from_str(&data).expect("Unable to parse JSON");

    let app_state = web::Data::new(AppState {
        cards: Arc::new(RwLock::new(cards)),
    });

    HttpServer::new(move || {
        let cors = Cors::default().allow_any_origin();
        App::new()
            .wrap(cors)
            .app_data(app_state.clone())
            .route("/api/search", web::get().to(search))
            .route("/api/card", web::get().to(view_card))
            .service(Files::new("/", "dist").index_file("index.html"))
            .default_service(web::route().to(serve_index))
    })
    .bind("104.248.54.50:80")?
    .run()
    .await
}

async fn search(data: web::Data<AppState>, query: web::Query<QueryParams>) -> impl Responder {
    let results = data.cards.read().await;

    let Ok(query_restrictions) = query_parser(&query.query.clone().unwrap_or_default()) else {
        let results = QueryResult::Error {
            message: "Query couldn't be parsed".to_string(),
        };
        return HttpResponse::Ok().json(results);
    };

    let results = hemoglobin::apply_restrictions(query_restrictions.as_slice(), &results);

    let results = QueryResult::CardList { content: results };

    HttpResponse::Ok().json(results)
}

async fn view_card(data: web::Data<AppState>, query: web::Query<IdViewParam>) -> impl Responder {
    let results = data.cards.read().await;

    let results: Vec<&Card> = results.iter().filter(|card| card.id == query.id).collect();

    HttpResponse::Ok().json(results.first().unwrap())
}
