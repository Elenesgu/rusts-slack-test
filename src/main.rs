use actix_web::{web, App, HttpRequest, HttpServer};

async fn index(req: HttpRequest) -> &'static str {
    println!("REQ: {:?}", req);
    "Hello world!"
}

#[actix_rt::main]
async fn main() -> std::io::Result<()> {
    HttpServer::new(|| {
        App::new()
            .route("/", web::post().to(index))
    })
    .bind("127.0.0.1:8080")?
    .run()
    .await
}