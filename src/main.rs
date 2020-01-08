use actix_rt::System;
use actix_web::http::{StatusCode};
use actix_web::{web, App, HttpRequest, HttpServer, HttpResponse, Result};
use serde_derive::{Serialize, Deserialize};
use openssl::ssl::{SslAcceptor, SslFiletype, SslMethod};
use std::sync::mpsc;
use std::thread;
use std::env;

#[derive(Clone, Debug, Deserialize)]
#[serde(tag = "type")]
#[serde(rename_all = "snake_case")]
pub enum BlockElement {
    RichTextSection { elements: Vec<Box<BlockElement>> },
    Text { text: String },
}

#[derive(Clone, Debug, Deserialize)]
pub struct Block {
    block_id: String,
    elements: Vec<BlockElement>,
    #[serde(rename = "type")]
    ty: String,
}

#[derive(Clone, Debug, Deserialize)]
pub struct Reaction {
    count: u32,
    name: String,
    users: Vec<String>,
}

#[derive(Clone, Debug, Deserialize)]
pub struct Edited {
    ts: String,
    user: String,
}

#[derive(Clone, Debug, Deserialize)]
pub struct Message {
    blocks: Option<Vec<Block>>,
    channel: Option<String>,
    channel_type: Option<String>,
    client_msg_id: Option<String>,
    deleted_ts: Option<String>,
    edited: Option<Edited>,
    event_ts: Option<String>,
    hidden: Option<bool>,
    is_starred: Option<bool>,
    message: Option<Box<Message>>,
    pinned_to: Option<Vec<String>>,
    previous_message: Option<Box<Message>>,
    reactions: Option<Vec<Reaction>>,
    source_team: Option<String>,
    subtype: Option<String>,
    team: Option<String>,
    text: Option<String>,
    ts: String,
    user: Option<String>,
    user_team: Option<String>,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(tag = "type")]
#[serde(rename_all = "snake_case")]
pub enum InternalEvent {
    Message(Message),
}

#[derive(Clone, Debug, Deserialize)]
pub struct EventCallback {
    api_app_id: String,
    authed_users: Vec<String>,
    event: InternalEvent,
    event_id: String,
    event_time: u64,
    team_id: String,
    token: String,
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

struct AppState {
    sender: mpsc::Sender::<EventCallback>,
}

#[derive(Clone, Debug, Serialize)]
struct PostMessage {
    channel: String,
    text: String,
}

async fn normal_handler(req: HttpRequest, body: String, state: web::Data<AppState>) -> Result<HttpResponse> {
    println!("REQ: {:?}", req);

    let content_str =
        if let Some(i) = req.headers().get("content-type") {
            i.to_str().unwrap()
        } else {
            ""
        };

    println!("type: {:?}", content_str);
    println!("body: {:?}", body);

    if content_str.contains("json") {

        let posted_event: SlackEvent = serde_json::from_str(&body)?;

        match posted_event {
            SlackEvent::UrlVerification {
                ref challenge,
                ..
            } => {
                Ok(
                    HttpResponse::build(StatusCode::OK)
                        .content_type("application/x-www-form-urlencoded")
                        .body(challenge)
                )
            },
            SlackEvent::EventCallback(event_callback) => {
                state.sender.send(event_callback).unwrap();
                Ok(
                    HttpResponse::build(StatusCode::OK)
                        .content_type("text/html; charset=utf-8")
                        .body(body)
                )
            },
        }
    } else {
        // response
        Ok(HttpResponse::build(StatusCode::BAD_REQUEST)
            .content_type("text/html; charset=utf-8")
            .body(body))
    }
}

#[actix_rt::main]
async fn main() {
    let secret_token = match env::var("SLACK_BOT_TOKEN") {
        Ok(val) => val,
        Err(_e) => panic!("Secret bot token is not given."),
    };

    let (slack_event_sender, slack_event_reciever) = mpsc::channel::<EventCallback>();

    let slack_server_thrd = thread::spawn(move || {
        let sys = System::new("slack-listner");
        
        let mut ssl_builder = SslAcceptor::mozilla_intermediate(SslMethod::tls()).unwrap();
    
        ssl_builder
            .set_private_key_file("PRIVATE_KEY.pem", SslFiletype::PEM)
            .unwrap();
        
        ssl_builder
            .set_certificate_chain_file("PUBLIC_KEY.pem")
            .unwrap();

        HttpServer::new(move || {
            App::new()
                .data(AppState {
                    sender: slack_event_sender.clone(),
                })
                .route("/", web::post().to(normal_handler))
                .route("/", web::get().to(normal_handler))
        })
        .bind_openssl("0.0.0.0:14475", ssl_builder)?
        .run();

        sys.run()
    });

    println!("{:?}", secret_token);

    let slack_event_handle_thrd = thread::spawn(move || {
        let event_listner = slack_event_reciever;
        //TODO: async?
        loop {
            let slack_event_wrap: EventCallback = event_listner.recv().unwrap();
            println!("{:#?}", slack_event_wrap);

            //TODO: remove hardcoded value
            let bot_id = "URS3HL8SD".to_string();

            match slack_event_wrap.event {
                InternalEvent::Message(slack_msg) => {
                    if slack_msg.subtype == None {
                        match &slack_msg.user {
                            Some(ref user_list) => {
                                if user_list.contains(&bot_id) {
                                    continue;
                                }
                            }
                            None => {
                                continue;
                            }
                        }

                        let reply = PostMessage {
                            channel: slack_msg.channel.unwrap(),
                            text: "hello, world".to_string(),
                        };
                        println!("To send struct : {:#?}", reply);

                        let request = reqwest::blocking::Client::new()
                            .post("https://slack.com/api/chat.postMessage")
                            .header("Content-type", "application/json; charset=utf-8")
                            .header("Authorization", "Bearer ".to_string() + &secret_token)
                            .json(&reply);

                        println!("To send request: {:#?}", request);
                        let res = request.send();
                        println!("respond: {:#?}", res);
                    }
                },
            }
        }
    });

    let _res = slack_server_thrd.join();
    let _res2 = slack_event_handle_thrd.join();
}

// curl 'https://slack.com/api/chat.postMessage' -H 'Authorization: Bearer SECRET' -H 'Content-type: application/json; charset=utf-8' -d '{"channel": "CS2AVF83X", "text": "hello, world"}'