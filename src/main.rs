#![warn(clippy::pedantic)]
mod cards;
mod search;

use actix_cors::Cors;
use actix_web::{web, App, HttpResponse, HttpServer, Responder};
use cards::Card;
use rust_fuzzy_search::fuzzy_compare;
use search::query_parser::query_parser;
use search::{QueryParams, QueryRestriction};
use serde::{Deserialize, Serialize};
use std::fs;
use std::sync::Arc;
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

async fn search(data: web::Data<AppState>, query: web::Query<QueryParams>) -> impl Responder {
    let results = data.cards.read().await;

    let Ok(query_restrictions) = query_parser(&query.query.clone().unwrap_or_default()) else {
        let results = QueryResult::Error {
            message: "Query couldn't be parsed".to_string(),
        };
        return HttpResponse::Ok().json(results);
    };

    let mut results: Vec<&Card> = results
        .iter()
        .filter(|card| {
            let mut filtered = true;
            for res in &query_restrictions {
                match res {
                    QueryRestriction::Fuzzy(x) => filtered = filtered && search::fuzzy(card, x),
                    QueryRestriction::Comparison(field, comparison) => {
                        filtered = filtered && comparison.compare(&field(card));
                    }
                    QueryRestriction::Contains(what, contains) => {
                        filtered = filtered
                            && what(card)
                                .to_lowercase()
                                .contains(contains.to_lowercase().as_str());
                    }
                    QueryRestriction::Has(fun, thing) => {
                        let x = fun(card);
                        filtered = filtered && x.iter().any(|x| x.contains(thing));
                    }
                    QueryRestriction::HasKw(fun, thing) => {
                        let x = fun(card);
                        filtered = filtered && x.iter().any(|x| x.name.contains(thing));
                    }
                }
            }
            filtered
        })
        .collect();

    let name = &query.query.clone().unwrap_or_default();

    results.sort_by(|a, b| {
        weighted_compare(b, name)
            .partial_cmp(&weighted_compare(a, name))
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    let results = QueryResult::CardList { content: results };

    HttpResponse::Ok().json(results)
}

async fn view_card(data: web::Data<AppState>, query: web::Query<IdViewParam>) -> impl Responder {
    let results = data.cards.read().await;

    let results: Vec<&Card> = results.iter().filter(|card| card.id == query.id).collect();

    HttpResponse::Ok().json(results.first().unwrap())
}

fn weighted_compare(a: &Card, b: &str) -> f32 {
    fuzzy_compare(&a.name, b) * 2.
        + fuzzy_compare(&a.r#type, b) * 1.8
        + fuzzy_compare(&a.description, b) * 1.6
        + a.kins
            .iter()
            .map(|x| fuzzy_compare(x, b))
            .max_by(|a, b| PartialOrd::partial_cmp(a, b).unwrap_or(std::cmp::Ordering::Less))
            .unwrap_or(0.0)
            * 1.5
        + a.keywords
            .iter()
            .map(|x| fuzzy_compare(&x.name, b))
            .max_by(|a, b| PartialOrd::partial_cmp(a, b).unwrap_or(std::cmp::Ordering::Less))
            .unwrap_or(0.0)
            * 1.2
}
