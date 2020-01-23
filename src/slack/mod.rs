use serde_derive::{Serialize, Deserialize};

#[derive(Clone, Debug, Deserialize)]
#[serde(tag = "type")]
#[serde(rename_all = "snake_case")]
pub enum BlockElement {
    RichTextSection { elements: Vec<Box<BlockElement>> },
    Text { text: String },
}

#[derive(Clone, Debug, Deserialize)]
pub struct Block {
    pub block_id: String,
    pub elements: Vec<BlockElement>,
    #[serde(rename = "type")]
    pub ty: String,
}

#[derive(Clone, Debug, Deserialize)]
pub struct Reaction {
    pub count: u32,
    pub name: String,
    pub users: Vec<String>,
}

#[derive(Clone, Debug, Deserialize)]
pub struct Edited {
    pub ts: String,
    pub user: String,
}

#[derive(Clone, Debug, Deserialize)]
pub struct Message {
    pub blocks: Option<Vec<Block>>,
    pub channel: Option<String>,
    pub channel_type: Option<String>,
    pub client_msg_id: Option<String>,
    pub deleted_ts: Option<String>,
    pub edited: Option<Edited>,
    pub event_ts: Option<String>,
    pub hidden: Option<bool>,
    pub is_starred: Option<bool>,
    pub message: Option<Box<Message>>,
    pub pinned_to: Option<Vec<String>>,
    pub previous_message: Option<Box<Message>>,
    pub reactions: Option<Vec<Reaction>>,
    pub source_team: Option<String>,
    pub subtype: Option<String>,
    pub team: Option<String>,
    pub text: Option<String>,
    pub ts: String,
    pub user: Option<String>,
    pub user_team: Option<String>,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(tag = "type")]
#[serde(rename_all = "snake_case")]
pub enum InternalEvent {
    Message(Message),
}

#[derive(Clone, Debug, Deserialize)]
pub struct EventCallback {
    pub api_app_id: String,
    pub authed_users: Vec<String>,
    pub event: InternalEvent,
    pub event_id: String,
    pub event_time: u64,
    pub team_id: String,
    pub token: String,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(tag = "type")]
#[serde(rename_all = "snake_case")]
pub enum SlackEvent {
    /// https://api.slack.com/events/url_verification
    ///
    /// This event is sent from Slack when the url is first entered.
    /// {
    ///     "token": "TOKEN_VALUE",
    ///     "challenge": "SOME_VALUE",
    ///     "type": "url_verification"
    /// }
    ///
    /// You should reposnd with
    ///
    /// HTTP 200 OK
    /// Content-type: application/x-www-form-urlencoded
    /// challenge=SOME_VALUE
    EventCallback(EventCallback),
    UrlVerification { token: String, challenge: String },
}