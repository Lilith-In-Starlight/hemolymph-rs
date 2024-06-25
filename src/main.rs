#![warn(clippy::pedantic)]

use actix_cors::Cors;
use actix_web::{web, App, HttpRequest, HttpResponse, HttpServer, Responder};
use hemoglobin::cards::Card;
use hemoglobin::search::query_parser::query_parser;
use hemolymph_frontend::ServerAppProps;
use notify::RecursiveMode;
use notify_debouncer_mini::new_debouncer;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::mpsc::TryRecvError;
use std::sync::Arc;
use std::time::Duration;
use std::{env, fs, io};
use tokio::sync::RwLock;
use tokio::task::{spawn_blocking, LocalSet};
use tokio::time::sleep;
use yew::ServerRenderer;

#[derive(Deserialize)]
struct QueryParams {
    query: Option<String>,
}

struct AppState {
    cards: Arc<RwLock<HashMap<String, Card>>>,
}

#[derive(Serialize)]
#[serde(tag = "type")]
pub enum QueryResult<'a> {
    CardList {
        query_text: String,
        content: Vec<&'a Card>,
    },
    Error {
        message: String,
    },
}

#[derive(Deserialize)]
struct IdViewParam {
    id: String,
}

async fn serve_index(data: web::Data<AppState>, req: HttpRequest) -> io::Result<HttpResponse> {
    let cards = data.cards.read().await;
    let path = req.path().to_string();
    let path = PathBuf::from(path);
    let card_details = path
        .iter()
        .nth(2)
        .map(|x| x.to_str().unwrap())
        .and_then(|x| cards.get(x).cloned());

    if path.extension().map_or(false, |x| x == "js") {
        let content = fs::read_to_string(format!("dist/{}", path.to_string_lossy()))?;
        Ok(HttpResponse::Ok()
            .content_type("application/javascript; charset=utf-8")
            .body(content))
    } else if path.extension().map_or(false, |x| x == "wasm") {
        let content = fs::read(format!("dist/{}", path.to_string_lossy()))?;
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
                let (description, name) = match card_details {
                    Some(card) => (card.description.clone(), get_filegarden_link(&card.name)),
                    None => (
                        "A search engine for Bloodless cards.".to_string(),
                        String::new(),
                    ),
                };
                let renderer =
                    ServerRenderer::<hemolymph_frontend::ServerApp>::with_props(move || {
                        ServerAppProps {
                            url: path.to_string_lossy().to_string().into(),
                            queries: HashMap::new(),
                        }
                    });
                content
                    .replace("{content}", &renderer.render().await)
                    .replace("{description}", &description)
                    .replace("{ogimage}", &name)
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
    let cards = create_card_map(cards);

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

    tokio::spawn(async move {
        let (tx, rx) = std::sync::mpsc::channel();
        let mut debouncer = new_debouncer(Duration::from_secs(1), tx).unwrap();
        debouncer
            .watcher()
            .watch(Path::new("cards.json"), RecursiveMode::Recursive)
            .unwrap();
        loop {
            match rx.try_recv() {
                Ok(_) => {
                    let data = fs::read_to_string("cards.json").expect("Unable to read file");
                    match serde_json::from_str::<Vec<Card>>(&data) {
                        Ok(data) => {
                            let mut cards = cards_pointer.write().await;
                            *cards = create_card_map(data);
                        }
                        Err(x) => eprintln!("{x:#?}"),
                    }
                }
                Err(TryRecvError::Empty) => (),
                Err(x) => eprintln!("{x:#?}"),
            }
            sleep(Duration::from_secs(0)).await;
        }
    });

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

fn create_card_map(vec: Vec<Card>) -> HashMap<String, Card> {
    vec.into_iter().map(|x| (x.id.clone(), x)).collect()
}

async fn search(data: web::Data<AppState>, query: web::Query<QueryParams>) -> impl Responder {
    let cards = data.cards.read().await;
    let cards = cards.values();

    match query_parser(&query.query.clone().unwrap_or_default()) {
        Ok(query_restrictions) => {
            let results = hemoglobin::search::search(&query_restrictions, cards);

            let results = QueryResult::CardList {
                content: results,
                query_text: format!("{query_restrictions}"),
            };

            HttpResponse::Ok().json(results)
        }
        Err(error) => {
            let error = QueryResult::Error {
                message: format!("Query couldn't be parsed: {error:#?}"),
            };
            HttpResponse::Ok().json(error)
        }
    }
}

async fn view_card(data: web::Data<AppState>, query: web::Query<IdViewParam>) -> impl Responder {
    let results = data.cards.read().await;

    let results: Option<&Card> = results.get(&query.id);

    match results {
        Some(results) => HttpResponse::Ok().json(results),
        None => HttpResponse::Ok().body("oops"),
    }
}

fn get_filegarden_link(name: &str) -> String {
    format!(
        "https://file.garden/ZJSEzoaUL3bz8vYK/bloodlesscards/{}.png",
        name.replace(' ', "").replace("ä", "a")
    )
}
