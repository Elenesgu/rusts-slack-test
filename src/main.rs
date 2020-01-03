
use actix_web::http::{StatusCode};
use actix_web::{web, App, HttpRequest, HttpServer, HttpResponse, Result};
use serde_derive::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
struct MyObj {
    channel: String,
    text: String,
}


// async fn json_handler((params, req): (web::Json<MyObj>, HttpRequest)) -> Result<HttpResponse> {
//     println!("JSON: {:?}", params);
//     println!("REQ: {:?}", req);

//         // response
//     Ok(HttpResponse::build(StatusCode::OK)
//         .content_type("text/html; charset=utf-8")
//         .body("include_str!(\"../static/welcome.html\")"))
// }

async fn json_handler((params, req): (web::Json<MyObj>, HttpRequest)) -> Result<HttpResponse> {
    println!("JSON: {:?}", params);
    println!("REQ: {:?}", req);

        // response
    Ok(HttpResponse::build(StatusCode::OK)
        .content_type("text/html; charset=utf-8")
        .body("include_str!(\"../static/welcome.html\")"))
}

async fn normal_handler(req: HttpRequest) -> Result<HttpResponse> {
    println!("REQ: {:?}", req);
    let content_str;

    if let Some(i) = req.headers().get("content-type") {
        content_str = i.to_str().unwrap();
    } else {
        content_str = "";
    }

    println!("type: {:?}", content_str);

    if content_str.contains("json") {
        // response
        Ok(HttpResponse::build(StatusCode::OK)
            .content_type("text/html; charset=utf-8")
            .body("foo"))
    } else {
        // response
        Ok(HttpResponse::build(StatusCode::OK)
            .content_type("text/html; charset=utf-8")
            .body("bar"))
    }
}

#[actix_rt::main]
async fn main() -> std::io::Result<()> {
    HttpServer::new(|| {
        App::new()
            .route("/", web::post().to(normal_handler))
            .route("/fo", web::post().to(normal_handler))
    })
    .bind("127.0.0.1:8080")?
    .run()
    .await
}

// curl 'https://slack.com/api/chat.postMessage' -H 'Authorization: Bearer SECRET' -H 'Content-type: application/json; charset=utf-8' -d '{"channel": "CS2AVF83X", "text": "hello, world"}'