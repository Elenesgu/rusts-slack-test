use actix::System;
use actix::prelude::*;
use actix_web::http::{StatusCode};
use actix_web::{web, App, HttpRequest, HttpServer, HttpResponse, Result};

use serde_derive::{Serialize};

use openssl::ssl::{SslAcceptor, SslFiletype, SslMethod};

use ctrlc;
use sha2::Sha256;
use hmac::{Hmac, Mac};

use futures::executor;

use std::env;

mod slack;

impl Message for slack::EventCallback {
    type Result = Result<(), std::io::Error>;
}

struct SlackEventActor {
    bot_token: String,
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
                        .header("Authorization", "Bearer ".to_string() + &self.bot_token)
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
    signing_secret: String,
}

#[derive(Clone, Debug, Serialize)]
struct PostMessage {
    channel: String,
    text: String,
}

struct ByteBuf<'a>(&'a [u8]);

impl<'a> std::fmt::LowerHex for ByteBuf<'a> {
    fn fmt(&self, fmtr: &mut std::fmt::Formatter) -> Result<(), std::fmt::Error> {
        for byte in self.0 {
            fmtr.write_fmt(format_args!("{:02x}", byte))?;
        }
        Ok(())
    }
}

async fn normal_handler(req: HttpRequest, body: String, state: web::Data<AppState>) -> Result<HttpResponse> {
    println!("REQ: {:?}", req);
    println!("Body: {:?}", body);


    let content_str =
        if let Some(i) = req.headers().get("content-type") {
            i.to_str().unwrap()
        } else {
            ""
        };

    let slack_signature: &str =
        if let Some(sig) = req.headers().get("X-Slack-Signature") {
            sig.to_str().unwrap()
        } else {
            return Ok(
                HttpResponse::Unauthorized().finish()
            );
        };

    let slack_timestamp: &str =
        if let Some(sig) = req.headers().get("X-Slack-Request-Timestamp") {
            sig.to_str().unwrap()
        } else {
            return Ok(
                HttpResponse::Unauthorized().finish()
            );
        };

    {
        let cur_timestamp = 
            std::time::SystemTime::now().duration_since(
                std::time::SystemTime::UNIX_EPOCH
            )
            .unwrap()
            .checked_sub(
                std::time::Duration::from_secs(slack_timestamp.parse::<u64>().unwrap())
            );
        println!("now: {:?}", cur_timestamp);
        //TODO: check replay attack
    }

    let signature_base_string: String =
        format!("v0:{}:{}", slack_timestamp, body);

    let mut mac = Hmac::<Sha256>::new_varkey(state.signing_secret.as_bytes()).expect("");
    mac.input(signature_base_string.as_bytes());

    let calculated_signature = 
        format!("v0={:02x}", ByteBuf(&mac.result().code().as_slice()));

    if slack_signature != calculated_signature {
        return Ok(
            HttpResponse::Unauthorized().finish()
        );
    }
    println!("Success to verify a slack's signature.");

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
    let bot_token = match env::var("SLACK_BOT_TOKEN") {
        Ok(val) => val,
        Err(_e) => panic!("Secret bot token is not given."),
    };
    println!("{:?}", bot_token);

    let signing_secret = match env::var("SLACK_SIGNING_SECRET") {
        Ok(val) => val,
        Err(_e) => panic!("Secret bot token is not given."),
    };
    
    let system = System::new("slack");

    let slack_event_actor = SyncArbiter::start(1, move || SlackEventActor {
        bot_token: bot_token.clone(),
        slack_client: reqwest::blocking::Client::new(),
    });

    let (tx, rx) = std::sync::mpsc::channel();

    let _ = std::thread::spawn(move || {
        let system = System::new("http");

        let mut ssl_builder = SslAcceptor::mozilla_intermediate(SslMethod::tls()).unwrap();

        ssl_builder
            .set_private_key_file("PRIVATE_KEY.pem", SslFiletype::PEM)
            .unwrap();
        
        ssl_builder
            .set_certificate_chain_file("PUBLIC_KEY.pem")
            .unwrap();

        let srv = HttpServer::new(move || { 
            App::new()
                .data(AppState {
                    sender: slack_event_actor.clone(),
                    signing_secret: signing_secret.clone(),
                })
                .route("/", web::post().to(normal_handler))
                .route("/", web::get().to(normal_handler))
            })
            .bind_openssl("0.0.0.0:14475", ssl_builder).unwrap()
            .run();

        let _ = tx.send(srv);

        system.run()
    });

    let srv = rx.recv().unwrap();
    let main_sys = System::current();
    let ctrl_c_pressed = std::sync::atomic::AtomicBool::new(false);

    ctrlc::set_handler(move || {
        if ctrl_c_pressed.load(std::sync::atomic::Ordering::SeqCst) {
            println!("Force to stop program.");
            std::process::exit(1);
        }
        ctrl_c_pressed.store(true, std::sync::atomic::Ordering::SeqCst);
        println!("Try to stop HttpServer.");
        executor::block_on(srv.stop(true));
        main_sys.stop();
        println!("Stopped.");
    }).expect("Fail to set Ctrl-C handler.");
        
    system.run()
}

// curl 'https://slack.com/api/chat.postMessage' -H 'Authorization: Bearer SECRET' -H 'Content-type: application/json; charset=utf-8' -d '{"channel": "CS2AVF83X", "text": "hello, world"}'