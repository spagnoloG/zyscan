mod model;
mod config;
mod scan_im;

use actix_web::{get, post, web, App, HttpResponse, HttpServer};
use actix_web::web::Json;
use model::User;
use mongodb::{bson::doc, options::IndexOptions, Client, Collection, IndexModel};
use std::thread;
use actix_rt::System;

const DB_NAME: &str = "zyscan";
const COLL_NAME: &str = "users";

/// Adds a new user to the "users" collection in the database.
#[post("/add_user")]
async fn add_user(client: web::Data<Client>, user_req: Json<User>) -> HttpResponse {
    let collection = client.database(DB_NAME).collection(COLL_NAME);
    let result = collection.insert_one(user_req, None).await; match result {
        Ok(_) => HttpResponse::Ok().body("user added"),
        Err(err) => HttpResponse::InternalServerError().body(err.to_string()),
    }
}

/// Gets the user with the supplied username.
#[get("/get_user/{username}")]
async fn get_user(client: web::Data<Client>, username: web::Path<String>) -> HttpResponse {
    let username = username.into_inner();
    let collection: Collection<User> = client.database(DB_NAME).collection(COLL_NAME);
    match collection
        .find_one(doc! { "username": &username }, None)
        .await
    {
        Ok(Some(user)) => HttpResponse::Ok().json(user),
        Ok(None) => {
            HttpResponse::NotFound().body(format!("No user found with username {username}"))
        }
        Err(err) => HttpResponse::InternalServerError().body(err.to_string()),
    }
}

/// Creates an index on the "username" field to force the values to be unique.
async fn create_username_index(client: &Client) {
    let options = IndexOptions::builder().unique(true).build();
    let model = IndexModel::builder()
        .keys(doc! { "username": 1 })
        .options(options)
        .build();
    client
        .database(DB_NAME)
        .collection::<User>(COLL_NAME)
        .create_index(model, None)
        .await
        .expect("creating an index should succeed");
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info"))
        .init();
    let cfg: config::AppConfig = confy::load("zyscan").unwrap();
    confy::store("zyscan", &cfg).unwrap(); // stores the config file to ~/.config/zyscan/zyscan.yml

    let client = Client::with_uri_str(cfg.db_connection).await.expect("failed to connect");
    create_username_index(&client).await;
    
    // Scan exif data in a seperate thread
    thread::spawn(move ||{
        let thread_cfg: config::AppConfig = confy::load("zyscan").unwrap();
        for folder in thread_cfg.scan_folders {
                let sys = System::new();
                sys.block_on(scan_im::load_images(&folder, &thread_cfg.db_connection)); 
                sys.run().unwrap();
        }
    });
    
    // This needs to run in the main thread
    HttpServer::new(move || {
        App::new()
            .app_data(web::Data::new(client.clone()))
            .service(add_user)
            .service(get_user)
    })
    .bind(("127.0.0.1", 8080))?
    .run()
    .await
}
