use crate::scan_im::CLASSES_FILE;
use crate::scan_im::Image;

use actix_web::{HttpResponse, get, web};
use std::fs::File;
use std::io::{BufRead, BufReader};
use mongodb::{bson::doc, Client, Collection, options::FindOptions};
use futures::stream::TryStreamExt;

const DB_NAME: &str = "zyscan";
const COLL_NAME: &str = "images";

#[get("/api/iclasses")]
async fn get_classes() -> HttpResponse {
    // Open classes file and serve it
    let file = File::open(CLASSES_FILE).unwrap();
    let reader = BufReader::new(file);
    let mut classes = Vec::new();
    for line in reader.lines() {
        classes.push(line.unwrap());
    }
    HttpResponse::Ok().json(classes) 
}

#[get("/api/images/{datetime}")]
async fn get_images(client: web::Data<Client>, datetime: web::Path<String>) -> HttpResponse {

    println!("get_images called");
 
    let _datetime = datetime.into_inner();
    let collection: Collection<Image> = client.database(DB_NAME).collection(COLL_NAME);
    let options = FindOptions::builder()
        .limit(50)
//        .sort(doc! { "i_datetime": -1 })
        .build();

    //let stream = collection
    //    .find(doc! { "i_datetime": { "$lt": &datetime } }, options)
    //    .await
    //    .expect("find should succeed");
    //
    let stream = collection.find(doc! {}, options).await.expect("find should succeed");

    
    let mut images: Vec<Image> = Vec::new();
    let mut stream = stream.into_stream();

    while let Some(image) = stream.try_next().await.unwrap() {
        images.push(image);
    }

    HttpResponse::Ok().json(images)    
}
