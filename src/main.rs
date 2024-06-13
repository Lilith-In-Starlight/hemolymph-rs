#![warn(clippy::pedantic)]

use actix_cors::Cors;
use actix_files::{Files, NamedFile};
use actix_web::{web, App, HttpRequest, HttpResponse, HttpServer, Responder};
use hemoglobin::cards::Card;
use hemoglobin::search::query_parser::query_parser;
use hemoglobin::search::QueryParams;
use hemolymph_frontend::ServerAppProps;
use notify::RecursiveMode;
use notify_debouncer_mini::new_debouncer;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;
use std::sync::Arc;
use std::time::Duration;
use std::{env, fs, io};
use tokio::runtime::Runtime;
use tokio::sync::RwLock;
use tokio::task::{spawn_blocking, LocalSet};
use yew::ServerRenderer;

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
    let path = req.path().trim().to_string();
    if path.ends_with(".js") {
        let content = fs::read_to_string(format!("dist/{}", path))?;
        Ok(HttpResponse::Ok()
            .content_type("application/javascript; charset=utf-8")
            .body(content))
    } else if path.ends_with(".wasm") {
        let content = fs::read(format!("dist/{}", path))?;
        Ok(HttpResponse::Ok()
            .content_type("application/wasm")
            .body(content))
    } else {
        let content = fs::read_to_string("dist/index.html")?;
        let path = path.clone();
        let content = spawn_blocking(move || {
            use tokio::runtime::Builder;
            let set = LocalSet::new();
            let rt = Builder::new_current_thread().enable_all().build().unwrap();
            set.block_on(&rt, async {
                let renderer =
                    ServerRenderer::<hemolymph_frontend::ServerApp>::with_props(move || {
                        ServerAppProps {
                            url: path.into(),
                            queries: HashMap::new(),
                        }
                    });
                content.replace("{content}", &renderer.render().await)
            })
        })
        .await
        .unwrap();
        Ok(HttpResponse::Ok().content_type("text/html").body(content))
    }
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    let data = fs::read_to_string("cards.json").expect("Unable to read file");
    let cards: Vec<Card> = serde_json::from_str(&data).expect("Unable to parse JSON");

    let app_state = web::Data::new(AppState {
        cards: Arc::new(RwLock::new(cards)),
    });

    let environment = env::var("RUST_ENV").unwrap_or_else(|_| "development".to_string());
    let env_file = match environment.as_str() {
        "production" => ".env.production",
        _ => ".env",
    };
    dotenv::from_filename(env_file).ok();

    // Read the HOST and PORT variables
    let host = env::var("HOST").unwrap_or_else(|_| "127.0.0.1".to_string());
    let port = env::var("PORT").unwrap_or_else(|_| "8080".to_string());

    let cards_pointer = Arc::clone(&app_state.cards);

    if env::var("WATCH").is_ok_and(|x| &x == "YEP") {
        tokio::spawn(async move {
            let (tx, rx) = std::sync::mpsc::channel();
            let mut debouncer = new_debouncer(Duration::from_secs(1), tx).unwrap();
            debouncer
                .watcher()
                .watch(Path::new("cards.json"), RecursiveMode::Recursive)
                .unwrap();

            loop {
                match rx.recv() {
                    Ok(_) => {
                        let data = fs::read_to_string("cards.json").expect("Unable to read file");
                        match serde_json::from_str::<Vec<Card>>(&data) {
                            Ok(data) => {
                                let mut cards = cards_pointer.write().await;
                                *cards = data;
                            }
                            Err(x) => eprintln!("{x:#?}"),
                        }
                    }
                    Err(_) => println!("boowomp"),
                }
            }
        });
    }
    HttpServer::new(move || {
        let cors = Cors::default().allow_any_origin();
        App::new()
            .wrap(cors)
            .app_data(app_state.clone())
            .route("/api/search", web::get().to(search))
            .route("/api/card", web::get().to(view_card))
            .default_service(web::route().to(serve_index))
    })
    .bind(format!("{host}:{port}"))?
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
