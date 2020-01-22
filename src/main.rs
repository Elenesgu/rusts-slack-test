use actix::System;
use actix::prelude::*;
use actix_web::http::{StatusCode};
use actix_web::{web, App, HttpRequest, HttpServer, HttpResponse, Result};
use ctrlc;
use serde_derive::{Serialize};
use openssl::ssl::{SslAcceptor, SslFiletype, SslMethod};

use std::env;

mod slack;

impl Message for slack::EventCallback {
    type Result = Result<(), std::io::Error>;
}

struct SlackEventActor {
    secret_token: String,
    slack_client: reqwest::blocking::Client,
}

impl Actor for SlackEventActor {
    type Context = SyncContext<Self>;
}

impl Handler<slack::EventCallback> for SlackEventActor {
    type Result = Result<(), std::io::Error>;

    fn handle(&mut self, msg: slack::EventCallback, _: &mut Self::Context) -> Self::Result {
        //TODO: remove hardcoded value
        let bot_id = "URS3HL8SD".to_string();

        match msg.event {
            slack::InternalEvent::Message(slack_msg) => {
                if slack_msg.subtype == None {
                    match &slack_msg.user {
                        Some(ref user_list) => {
                            if user_list.contains(&bot_id) {
                                return Ok(());
                            }
                        }
                        None => {
                            return Ok(());
                        }
                    }

                    let reply = PostMessage {
                        channel: slack_msg.channel.unwrap(),
                        text: "hello, world".to_string(),
                    };

                    let request = self.slack_client
                        .post("https://slack.com/api/chat.postMessage")
                        .header("Content-type", "application/json; charset=utf-8")
                        .header("Authorization", "Bearer ".to_string() + &self.secret_token)
                        .json(&reply);

                    request.send();
                }
            },
        }
        Ok(())
    }
}

struct AppState {
    sender: Addr<SlackEventActor>,
}

#[derive(Clone, Debug, Serialize)]
struct PostMessage {
    channel: String,
    text: String,
}

async fn normal_handler(req: HttpRequest, body: String, state: web::Data<AppState>) -> Result<HttpResponse> {
    let content_str =
        if let Some(i) = req.headers().get("content-type") {
            i.to_str().unwrap()
        } else {
            ""
        };

    if content_str.contains("json") {

        let posted_event: slack::SlackEvent = serde_json::from_str(&body)?;

        match posted_event {
            slack::SlackEvent::UrlVerification {
                ref challenge,
                ..
            } => {
                Ok(
                    HttpResponse::build(StatusCode::OK)
                        .content_type("application/x-www-form-urlencoded")
                        .body(challenge)
                )
            },
            slack::SlackEvent::EventCallback(event_callback) => {
                state.sender.do_send(event_callback);
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

fn main() -> std::io::Result<()> {
    let secret_token = match env::var("SLACK_BOT_TOKEN") {
        Ok(val) => val,
        Err(_e) => panic!("Secret bot token is not given."),
    };
    println!("{:?}", secret_token);
    
    let system = System::new("slack");

    let slack_event_actor = SyncArbiter::start(1, move || SlackEventActor {
        secret_token: secret_token.clone(),
        slack_client: reqwest::blocking::Client::new(),
    });


    let _ = std::thread::spawn(move || {
        let system = System::new("http");

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
                    sender: slack_event_actor.clone(),
                })
                .route("/", web::post().to(normal_handler))
                .route("/", web::get().to(normal_handler))
            })
            .bind_openssl("0.0.0.0:14475", ssl_builder).unwrap()
            .run();

        system.run()
    });

    ctrlc::set_handler(move || {
        println!("Try to stop HttpServer.");
        std::process::exit(1);
    }).expect("Fail to set Ctrl-C handler.");
        
    system.run()
}

// curl 'https://slack.com/api/chat.postMessage' -H 'Authorization: Bearer SECRET' -H 'Content-type: application/json; charset=utf-8' -d '{"channel": "CS2AVF83X", "text": "hello, world"}'