use std::collections::{HashMap, VecDeque};

use anyhow::anyhow;
use reqwest::{
    header::{HeaderMap, HeaderValue, AUTHORIZATION},
    StatusCode,
};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use uuid::Uuid;

use crate::{model::Model, BASE_API_URL};

#[derive(Serialize, Deserialize)]
pub struct ChatManager {
    current_chat: Uuid,
    chats: HashMap<Uuid, Chat>,
}

impl ChatManager {
    pub fn new() -> Self {
        Self {
            current_chat: Uuid::nil(),
            chats: HashMap::new(),
        }
    }

    /// Returns if chat specified by provided uuid exists
    pub fn chat_exists(&self, chat_uuid: Uuid) -> bool {
        self.chats.contains_key(&chat_uuid)
    }

    /**
    Creates a new chat and sets the current chat to it if current chat is equal to nil

    Returns created chat's uuid
    */
    pub fn create_new_chat(&mut self, model: Model) -> Uuid {
        let chat_uuid = Uuid::new_v4();
        let chat = Chat::new(chat_uuid, model);
        self.chats.insert(chat_uuid, chat);
        if self.current_chat.is_nil() {
            self.current_chat = chat_uuid;
        }
        chat_uuid
    }

    /**
    Removes a chat by uuid and sets current chat to nil if it's uuid is equal to the provided one

    Fails if chat does not exist
    */
    pub fn remove_chat(&mut self, chat_uuid: Uuid) -> anyhow::Result<()> {
        if !self.chat_exists(chat_uuid) {
            return Err(anyhow!("chat does not exist"));
        }
        self.chats.remove(&chat_uuid);
        if self.current_chat == chat_uuid {
            self.current_chat = Uuid::nil();
        }
        Ok(())
    }

    /**
    Gets chat by uuid

    Returns None if chat does not exist
    */
    pub fn get_chat(&mut self, chat_uuid: Uuid) -> Option<&mut Chat> {
        if !self.chat_exists(chat_uuid) {
            return None;
        }

        Some(self.chats.get_mut(&chat_uuid).unwrap())
    }

    /**
    Sets current chat and assures that the chat exists

    Fails if chat does not exist (which should never happen, if it does report it)
    */
    pub fn set_current_chat(&mut self, chat_uuid: Uuid) -> anyhow::Result<()> {
        if !self.chat_exists(chat_uuid) {
            return Err(anyhow!("chat does not exist"));
        }

        self.current_chat = chat_uuid;
        Ok(())
    }

    /**
    Gets current chat & uuid and assures that the chat exists

    Returns current chat and it's uuid

    Returns None if current chat does not exist
    */
    pub fn get_current_chat(&mut self) -> Option<(Uuid, &mut Chat)> {
        if !self.chat_exists(self.current_chat) {
            return None;
        }

        Some((
            self.current_chat,
            self.chats.get_mut(&self.current_chat).unwrap(),
        ))
    }
}

// change f32 to f16 when it's available in stable release
#[derive(Serialize, Deserialize)]
pub struct Chat {
    pub title: String,
    pub model: Model,
    pub max_tokens: u32,
    pub stream: bool,
    pub frequency_penalty: f32,
    pub top_p: f32,
    pub temperature: f32,
    pub history: VecDeque<Message>,
}

impl Chat {
    pub fn new(uuid: Uuid, model: Model) -> Self {
        Self {
            title: format!("Chat {uuid}"),
            model,
            max_tokens: 512,
            stream: false,
            frequency_penalty: 0.7,
            top_p: 0.7,
            temperature: 0.7,
            history: VecDeque::new(),
        }
    }

    /**
    Sends an message and adds it to history as first element

    Returns Ok if message was sent successfully and adds response to history as first element

    Fails if sending a message returned an error and adds error message to history as first element
    */
    pub async fn send_message(&mut self, api_key: &str, msg: Message) -> anyhow::Result<()> {
        let res = self.internal_send_message(api_key, msg).await;
        if res.is_err() {
            self.history.push_front(Message::new(
                MessageRole::AI,
                "An error occured while sending the message",
            ));
        }
        res
    }

    /// Returns Ok if message was sent successfully and adds response to history as first element
    async fn internal_send_message(&mut self, api_key: &str, msg: Message) -> anyhow::Result<()> {
        let client = reqwest::Client::new();
        let mut headers = HeaderMap::new();
        headers.insert(
            AUTHORIZATION,
            HeaderValue::from_str(&format!("Bearer {api_key}")).unwrap(),
        );

        self.history.push_front(msg);

        let mut history = self.history.clone();
        history.retain(|message| message.get_role() != MessageRole::AI);
        let messages = json!(history);

        let json = json!({
        "model": self.model.name,
        "max_tokens": self.max_tokens,
        "stream": self.stream,
        "frequency_penalty": self.frequency_penalty,
        "top_p": self.top_p,
        "temperature": self.temperature,
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
        let message = json["choices"].take()[0].take()["message"].take()["content"].take();

        if !message.is_string() {
            return Err(anyhow!("message is not a string"));
        }

        self.history
            .push_front(Message::new(MessageRole::AI, message.as_str().unwrap()));

        Ok(())
    }
}

#[derive(Serialize, Deserialize, Clone)]
pub struct Message {
    role: String,
    pub content: String,
}

impl Message {
    pub fn new(role: MessageRole, content: &str) -> Self {
        Self {
            role: role.into(),
            content: content.to_string(),
        }
    }

    pub fn get_role(&self) -> MessageRole {
        MessageRole::from(&self.role)
    }
}

#[derive(Serialize, Deserialize, Clone, PartialEq, Eq)]
pub enum MessageRole {
    USER,
    SYSTEM,
    AI,
}

impl From<&String> for MessageRole {
    fn from(value: &String) -> Self {
        match value.as_str() {
            "user" => Self::USER,
            "system" => Self::SYSTEM,
            _ => Self::AI,
        }
    }
}

impl Into<String> for MessageRole {
    fn into(self) -> String {
        match self {
            Self::USER => "user".to_string(),
            Self::SYSTEM => "system".to_string(),
            Self::AI => "placeholder".to_string(),
        }
    }
}
