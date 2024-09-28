use std::collections::VecDeque;

use anyhow::anyhow;
use reqwest::{
    header::{HeaderMap, HeaderValue, AUTHORIZATION},
    StatusCode,
};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

use crate::{model::Model, BASE_API_URL};

/// Returns Ok if message was sent successfully and adds response to history as first element
pub async fn send_completion(
    api_key: &str,
    model: &Model,
    msg: Completion,
    params: &CompletionParams,
    history: &mut Option<VecDeque<Completion>>,
) -> anyhow::Result<()> {
    let client = reqwest::Client::new();
    let mut headers = HeaderMap::new();
    headers.insert(
        AUTHORIZATION,
        HeaderValue::from_str(&format!("Bearer {api_key}")).unwrap(),
    );

    add_history(history, msg.clone());

    let messages = if history.is_some() {
        // if im correct i should remove ai messages from history
        let mut history = history.as_ref().unwrap().clone();
        history.retain(|msg| msg.get_role() != CompletionRole::AI);
        json!(history)
    } else {
        let history: [Completion; 1] = [msg];
        json!(history)
    };

    let json = json!({
    "model": model.name,
    "max_tokens": params.max_tokens,
    "frequency_penalty": params.frequency_penalty,
    "top_p": params.top_p,
    "temperature": params.temperature,
    "stream": params.stream,
    "messages": messages,
    });
    let res = client
        .post(BASE_API_URL.to_string() + "/chat/completions")
        .headers(headers)
        .json(&json)
        .send()
        .await?;

    if res.status() != StatusCode::CREATED {
        return Err(anyhow!("request failed {}", res.status()));
    }

    let res = res.text().await?;
    let mut json: Value = serde_json::from_str(&res)?;
    // TODO: return every choice
    let message = json["choices"].take()[0].take()["message"].take()["content"].take();

    if !message.is_string() {
        return Err(anyhow!("message is not a string"));
    }

    add_history(
        history,
        Completion::new(CompletionRole::AI, message.as_str().unwrap()),
    );

    Ok(())
}

pub fn add_history(history: &mut Option<VecDeque<Completion>>, msg: Completion) {
    if history.is_none() {
        return;
    }

    history.as_mut().unwrap().push_front(msg);
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Completion {
    role: String,
    pub content: String,
}

impl Completion {
    pub fn new(role: CompletionRole, content: &str) -> Self {
        Self {
            role: role.into(),
            content: content.to_string(),
        }
    }

    pub fn get_role(&self) -> CompletionRole {
        CompletionRole::from(self.role.clone())
    }
}

// change f32 to f16 when it's available in stable release
#[derive(Serialize, Deserialize, Debug, Clone, Copy)]
pub struct CompletionParams {
    pub max_tokens: u32,
    pub frequency_penalty: f32,
    pub top_p: f32,
    pub temperature: f32,
    pub stream: bool,
}

impl CompletionParams {
    pub fn new(
        max_tokens: u32,
        frequency_penalty: f32,
        top_p: f32,
        temperature: f32,
        stream: bool,
    ) -> Self {
        Self {
            max_tokens,
            frequency_penalty,
            top_p,
            temperature,
            stream,
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub enum CompletionRole {
    USER,
    SYSTEM,
    AI,
}

impl From<String> for CompletionRole {
    fn from(value: String) -> Self {
        match value.as_str() {
            "user" => Self::USER,
            "system" => Self::SYSTEM,
            _ => Self::AI,
        }
    }
}

impl From<CompletionRole> for String {
    fn from(value: CompletionRole) -> Self {
        match value {
            CompletionRole::USER => "user".to_string(),
            CompletionRole::SYSTEM => "system".to_string(),
            CompletionRole::AI => "placeholder".to_string(),
        }
    }
}
