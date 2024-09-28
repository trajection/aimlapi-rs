use std::collections::{HashMap, VecDeque};

use anyhow::anyhow;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{
    chat::{add_history, send_completion, Completion, CompletionParams, CompletionRole},
    model::Model,
};

#[derive(Serialize, Deserialize, Debug, Default, Clone)]
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
        let chat = Chat::new(model);
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

    /// Sends a completion in the current chat
    pub async fn send_current_chat_completion(
        &mut self,
        api_key: &str,
        msg: Completion,
    ) -> anyhow::Result<()> {
        let (_, current_chat) = match self.get_current_chat() {
            Some(chat) => chat,
            None => {
                return Err(anyhow!(
                    "failed to send message, current chat does not exist"
                ))
            }
        };

        current_chat.send_completion(api_key, msg).await?;
        Ok(())
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Chat {
    pub title: Option<String>,
    pub model: Model,
    pub global_params: CompletionParams,
    pub history: Option<VecDeque<Completion>>,
}

impl Chat {
    pub fn new(model: Model) -> Self {
        Self {
            title: None,
            model,
            global_params: CompletionParams::new(512, 0.7, 0.7, 0.7, false),
            history: None,
        }
    }

    pub fn with_title(&mut self, title: String) -> &mut Self {
        self.title = Some(title);
        self
    }

    pub fn with_history(&mut self) -> &mut Self {
        self.history = Some(VecDeque::new());
        self
    }

    /**
    Sends a completion and adds it to history as first element

    Returns Ok if message was sent successfully and adds response to history as first element

    Fails if sending a message returned an error and adds error message to history as first element
    */
    pub async fn send_completion(&mut self, api_key: &str, msg: Completion) -> anyhow::Result<()> {
        let res = send_completion(
            api_key,
            &self.model,
            msg,
            &self.global_params,
            &mut self.history,
        )
        .await;
        if res.is_err() {
            add_history(
                &mut self.history,
                Completion::new(
                    CompletionRole::AI,
                    "An error occured while sending the message",
                ),
            );
        }
        res
    }
}
