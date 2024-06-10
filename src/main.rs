#![warn(clippy::pedantic)]
mod cards;
mod search;

use crate::search::fuzzy_search;
use actix_cors::Cors;
use actix_web::{web, App, HttpResponse, HttpServer, Responder};
use cards::Card;
use search::QueryParams;
use serde::Deserialize;
use std::fs;
use std::sync::Arc;
use tokio::sync::RwLock;

struct AppState {
    cards: Arc<RwLock<Vec<Card>>>,
}

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

enum Restriction {
    Name(String),
    Kin(String),
    Cost(Comparison),
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
