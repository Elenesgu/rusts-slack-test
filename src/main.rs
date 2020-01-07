
use actix_web::http::{StatusCode};
use actix_web::{web, App, HttpRequest, HttpServer, HttpResponse, Result};
use serde_derive::{Deserialize, Serialize};
use openssl::ssl::{SslAcceptor, SslFiletype, SslMethod};

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
    UrlVerification { token: String, challenge: String }
}

async fn normal_handler(req: HttpRequest, body: String) -> Result<HttpResponse> {
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
        println!("json body: {:?}", posted_event);

        match posted_event {
            SlackEvent::UrlVerification {
                ref challenge,
                ..
            } => {
                Ok(HttpResponse::build(StatusCode::OK)
                    .content_type("application/x-www-form-urlencoded")
                    .body(challenge)
                )
            },
        }
    } else {
        // response
        Ok(HttpResponse::build(StatusCode::OK)
            .content_type("text/html; charset=utf-8")
            .body(body))
    }
}

#[actix_rt::main]
async fn main() -> std::io::Result<()> {

    let mut ssl_builder = SslAcceptor::mozilla_intermediate(SslMethod::tls()).unwrap();
    
    ssl_builder
        .set_private_key_file("PRIVATE_KEY.pem", SslFiletype::PEM)
        .unwrap();
    
    ssl_builder
        .set_certificate_chain_file("PUBLIC_KEY.pem")
        .unwrap();

    let server = HttpServer::new(|| {
        App::new()
            .route("/", web::post().to(normal_handler))
            .route("/", web::get().to(normal_handler))
    });

    server.bind_openssl("0.0.0.0:14475", ssl_builder)?
          .run()
          .await
}

// curl 'https://slack.com/api/chat.postMessage' -H 'Authorization: Bearer SECRET' -H 'Content-type: application/json; charset=utf-8' -d '{"channel": "CS2AVF83X", "text": "hello, world"}'