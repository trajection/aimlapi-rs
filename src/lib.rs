use std::{
    collections::HashMap,
    fs::File,
    io::{Read, Write},
    path::Path,
};

use anyhow::anyhow;
use chat::{ChatManager, Message};
use model::Model;
use serde::{Deserialize, Serialize};

pub mod chat;
pub mod model;

pub const BASE_API_URL: &str = "https://api.aimlapi.com";

#[derive(Serialize, Deserialize)]
pub struct AIMLAPI {
    local_save: bool,
    api_key: Option<String>,
    pub models: Vec<Model>,
    pub chat_manager: ChatManager,
}

impl AIMLAPI {
    /**
    Creates AIMLAPI instance and obtains AIMLAPI models

    Will return an error if AIMLAPI::get_models() fails or save.json file exists but is not a proper save file
     */
    pub async fn start() -> anyhow::Result<Self> {
        let models = AIMLAPI::get_models().await?;
        let mut instance: Self;

        if Path::new("save.json").exists() {
            let mut buf = String::new();
            let mut save = File::open("save.json").unwrap();
            save.read_to_string(&mut buf).unwrap();
            instance = serde_json::from_str(&buf).unwrap();
            instance.models = models;
        } else {
            instance = Self {
                local_save: true,
                api_key: None,
                models,
                chat_manager: ChatManager::new(),
            }
        }

        Ok(instance)
    }

    /// Change local save state
    pub fn local_save(&mut self, save: bool) -> &mut Self {
        self.local_save = save;
        self
    }

    /// Sets api key to be used by all requests
    pub fn set_api_key(&mut self, api_key: &str) {
        self.api_key = Some(api_key.to_string());
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

    /// Sends message in the current chat
    pub async fn send_message(&mut self, msg: Message) -> anyhow::Result<()> {
        if self.api_key.is_none() {
            return Err(anyhow!("api key not provided"));
        }

        let (_, current_chat) = match self.chat_manager.get_current_chat() {
            Some(chat) => chat,
            None => {
                return Err(anyhow!(
                    "failed to send message, current chat does not exist"
                ))
            }
        };

        let api_key = self.api_key.as_ref().unwrap();
        current_chat.send_message(api_key, msg).await?;
        Ok(())
    }

    /**
    Saves the instance if local_save is true

    Currently there is no other way to save the config than to use this function manually
    */
    pub fn save(&self) -> anyhow::Result<()> {
        if !self.local_save {
            return Ok(());
        }

        let mut save = File::create("save.json")?;
        let json = serde_json::to_string_pretty(self)?;
        save.write_all(json.as_bytes())?;
        Ok(())
    }
}
