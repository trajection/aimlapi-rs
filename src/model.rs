use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct Model {
    pub name: String,
}

impl From<String> for Model {
    fn from(value: String) -> Self {
        Self { name: value }
    }
}
