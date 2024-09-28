use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use crate::BASE_API_URL;

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, Hash)]
pub struct Model {
    pub name: String,
}

impl From<String> for Model {
    fn from(value: String) -> Self {
        Self { name: value }
    }
}

/**
Retrieves all AIMLAPI models

Will return an error if request fails
*/
pub async fn get_models() -> anyhow::Result<Vec<Model>> {
    let res = reqwest::get(BASE_API_URL.to_string() + "/models")
        .await?
        .text()
        .await?;

    let model_map: HashMap<String, String> = serde_json::from_str(&res)?;
    let models = model_map
        .keys()
        .map(|key| Model::from(key.to_owned()))
        .collect();
    Ok(models)
}
