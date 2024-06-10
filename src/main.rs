#![warn(clippy::pedantic)]
use actix_cors::Cors;
use actix_web::{web, App, HttpResponse, HttpServer, Responder};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::sync::Arc;
use tokio::sync::RwLock;

enum Errors {
    InvalidComparisonString,
}

#[derive(Deserialize)]
enum Comparison {
    GreaterThan,
    Equal,
    LowerThan,
    NotEqual,
}

#[derive(Deserialize)]
struct QueryParams {
    query: Option<String>,
    cost: Option<String>,
}

#[derive(Deserialize)]
struct IdViewParam {
    id: String,
}

fn compare<T: PartialOrd>(a: &T, b: &T, comparison: &Comparison) -> bool {
    match comparison {
        Comparison::GreaterThan => a.gt(b),
        Comparison::Equal => a.eq(b),
        Comparison::LowerThan => a.lt(b),
        Comparison::NotEqual => a.ne(b),
    }
}

fn text_comparison_parser(s: &str) -> Result<(Comparison, usize), Errors> {
    match s.parse::<usize>() {
        Ok(x) => Ok((Comparison::Equal, x)),
        Err(_) => {
            if let Some(end) = s.strip_prefix('>') {
                end.parse::<usize>()
                    .map(|x| (Comparison::GreaterThan, x))
                    .map_err(|_| Errors::InvalidComparisonString)
            } else if let Some(end) = s.strip_prefix('<') {
                end.parse::<usize>()
                    .map(|x| (Comparison::LowerThan, x))
                    .map_err(|_| Errors::InvalidComparisonString)
            } else if let Some(end) = s.strip_prefix('=') {
                end.parse::<usize>()
                    .map(|x| (Comparison::Equal, x))
                    .map_err(|_| Errors::InvalidComparisonString)
            } else if let Some(end) = s.strip_prefix("!=") {
                end.parse::<usize>()
                    .map(|x| (Comparison::NotEqual, x))
                    .map_err(|_| Errors::InvalidComparisonString)
            } else {
                Err(Errors::InvalidComparisonString)
            }
        }
    }
}

fn filter_comparison<V: Fn(&Card) -> usize>(
    card: &Card,
    property: V,
    comparison: &Option<(Comparison, usize)>,
) -> bool {
    if let Some(comparison) = comparison {
        compare(&property(card), &comparison.1, &comparison.0)
    } else {
        true
    }
}

fn fuzzy_search(card: &Card, query: &web::Query<QueryParams>) -> bool {
    query.query.as_ref().map_or(false, |query| {
        card.description
            .to_lowercase()
            .as_str()
            .contains(&query.to_lowercase())
            || card.name.to_lowercase().contains(&query.to_lowercase())
    })
}

async fn search(data: web::Data<AppState>, query: web::Query<QueryParams>) -> impl Responder {
    let results = data.cards.read().await;

    let results: Vec<&Card> = results
        .iter()
        .filter(|card| fuzzy_search(card, &query))
        .collect();

    HttpResponse::Ok().json(results)
}

async fn view_card(data: web::Data<AppState>, query: web::Query<IdViewParam>) -> impl Responder {
    let results = data.cards.read().await;

    let results: Vec<&Card> = results.iter().filter(|card| card.id == query.id).collect();

    HttpResponse::Ok().json(results.first().unwrap())
}

struct AppState {
    cards: Arc<RwLock<Vec<Card>>>,
}

#[derive(Deserialize, Serialize, Debug)]
#[serde(tag = "type")]
enum KeywordData {
    CardID(CardID),
    String(String),
}

#[derive(Deserialize, Serialize, Debug)]
struct Keyword {
    name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    data: Option<KeywordData>,
}

#[derive(Deserialize, Serialize, Debug)]
struct Card {
    id: String,
    name: String,
    description: String,
    cost: usize,
    health: usize,
    defense: usize,
    power: usize,
    r#type: String,
    #[serde(default)]
    keywords: Vec<Keyword>,
    #[serde(default)]
    kins: Vec<String>,
    #[serde(default)]
    abilities: Vec<String>,
    #[serde(default)]
    artists: Vec<String>,
    set: String,
    legality: HashMap<String, String>,
    #[serde(default)]
    other: Vec<String>,
    #[serde(default)]
    functions: Vec<String>,
}

#[derive(Deserialize, Serialize, Debug)]
struct CardID {
    #[serde(skip_serializing_if = "Option::is_none")]
    name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(default)]
    keywords: Option<Vec<Keyword>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    r#type: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(default)]
    kins: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    health: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    defense: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    power: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(default)]
    abilities: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(default)]
    functions: Option<Vec<String>>,
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
            .route("/search", web::get().to(search))
            .route("/card", web::get().to(view_card))
    })
    .bind("127.0.0.1:3000")?
    .run()
    .await
}

impl Card {
    fn get_cost(&self) -> usize {
        self.cost
    }
}
