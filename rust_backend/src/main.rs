mod config;
mod scan_im;
mod image;

use scan_im::THUMBNAIL_LOCATION;

use actix_rt::System;
use actix_web::{web, App, HttpServer};
use actix_files as fs;
use std::thread;
use mongodb::Client;

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();

    let cfg: config::AppConfig = confy::load("zyscan").unwrap();
    confy::store("zyscan", &cfg).unwrap(); // stores the config file to ~/.config/zyscan/zyscan.yml

    let client = Client::with_uri_str(cfg.db_connection)
        .await
        .expect("failed to connect");

    // Scan exif data in a seperate thread
    thread::spawn(move || {
        let thread_cfg: config::AppConfig = confy::load("zyscan").unwrap();
        let sys = System::new();
        sys.block_on(scan_im::load_images(thread_cfg.clone()));
        sys.run().unwrap();
    });

    // This needs to run in the main thread
    HttpServer::new(move || {
        let mut app = App::new()
            .app_data(web::Data::new(client.clone()))
            .service(
                fs::Files::new("/thumbnails", THUMBNAIL_LOCATION)
                    .show_files_listing()
                    .use_last_modified(true),
            )
           .service(image::get_images);
        
        // Serve all the images from the config file
        for folder in &cfg.scan_folders {
            app = app.service(
                fs::Files::new("/images", folder)
                    .show_files_listing()
                    .use_last_modified(true),
            );
        }

        app
    })

    .bind(("127.0.0.1", 8080))?
    .run()
    .await
}
