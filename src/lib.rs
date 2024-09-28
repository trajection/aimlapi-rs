pub mod chat;
pub mod model;

#[cfg(feature = "managers")]
pub mod managers;

pub const BASE_API_URL: &str = "https://api.aimlapi.com";

/*
API terms:
    - Completion (or msg): https://docs.aimlapi.com/api-overview/text-models-llm/completion-or-chat-models#what-is-a-completion
*/
