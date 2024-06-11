use std::collections::HashMap;

use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize, Debug)]
pub struct Card {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub img: Vec<String>,
    pub description: String,
    pub cost: usize,
    pub health: usize,
    pub defense: usize,
    pub power: usize,
    pub r#type: String,
    #[serde(default)]
    pub keywords: Vec<Keyword>,
    #[serde(default)]
    pub kins: Vec<String>,
    #[serde(default)]
    pub abilities: Vec<String>,
    #[serde(default)]
    pub artists: Vec<String>,
    pub set: String,
    pub legality: HashMap<String, String>,
    #[serde(default)]
    pub other: Vec<String>,
    #[serde(default)]
    pub functions: Vec<String>,
}

impl Card {
    pub fn get_cost(&self) -> usize {
        self.cost
    }
    pub fn get_name(&self) -> &str {
        &self.name
    }
    pub fn get_type(&self) -> &str {
        &self.r#type
    }
    pub fn get_kins(&self) -> &[String] {
        &self.kins
    }
    pub fn get_keywords(&self) -> &[Keyword] {
        &self.keywords
    }
    pub fn get_health(&self) -> usize {
        self.health
    }
    pub fn get_power(&self) -> usize {
        self.power
    }
    pub fn get_defense(&self) -> usize {
        self.defense
    }
}

#[derive(Deserialize, Serialize, Debug)]
pub struct CardID {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(default)]
    pub keywords: Option<Vec<Keyword>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub r#type: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(default)]
    pub kins: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub health: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub defense: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub power: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(default)]
    pub abilities: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(default)]
    pub functions: Option<Vec<String>>,
}

#[derive(Deserialize, Serialize, Debug)]
#[serde(tag = "type")]
pub enum KeywordData {
    CardID(CardID),
    String(String),
}

#[derive(Deserialize, Serialize, Debug)]
pub struct Keyword {
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<KeywordData>,
}
