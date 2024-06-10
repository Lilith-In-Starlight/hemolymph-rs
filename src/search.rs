use actix_web::web;
use serde::Deserialize;

use crate::cards::Card;

#[derive(Deserialize)]
pub struct QueryParams {
    pub query: Option<String>,
    pub cost: Option<String>,
}

pub fn fuzzy_search(card: &Card, query: &web::Query<QueryParams>) -> bool {
    query.query.as_ref().map_or(false, |query| {
        card.description
            .to_lowercase()
            .as_str()
            .contains(&query.to_lowercase())
            || card.name.to_lowercase().contains(&query.to_lowercase())
    })
}
