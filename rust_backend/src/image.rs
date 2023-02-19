use crate::scan_im::CLASSES_FILE;
use crate::scan_im::Image;

use actix_web::{HttpResponse, get, web};
use std::fs::File;
use std::io::{BufRead, BufReader};
use mongodb::{bson::doc, Client, Collection, options::FindOptions};
use futures::stream::TryStreamExt;
use serde::{Deserialize, Deserializer};
use chrono::{DateTime, Utc};

const DB_NAME: &str = "zyscan";
const COLL_NAME: &str = "images";

struct UtcDateTime(DateTime<Utc>);

#[get("/api/iclasses")]
async fn get_classes() -> HttpResponse {
    let file = File::open(CLASSES_FILE).unwrap();
    let reader = BufReader::new(file);
    let mut classes = Vec::new();
    for line in reader.lines() {
        classes.push(line.unwrap());
    }
    HttpResponse::Ok().json(classes) 
}

#[derive(Deserialize)]
struct DateTimeQuery {
    dt: UtcDateTime,
}

impl<'de> Deserialize<'de> for UtcDateTime {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        let dt = DateTime::parse_from_rfc3339(&s)
            .map_err(serde::de::Error::custom)?
            .with_timezone(&Utc);
        Ok(UtcDateTime(dt))
    }
}

#[get("/api/images")]
async fn get_images(client: web::Data<Client>, params: web::Query<DateTimeQuery>) -> HttpResponse {

    // convert params.dt to bson::DateTime
    let dt = mongodb::bson::DateTime::from_millis(params.dt.0.timestamp_millis());
  
    let collection: Collection<Image> = client.database(DB_NAME).collection(COLL_NAME);
    let options = FindOptions::builder()
        .limit(50)
        .sort(doc! { "i_datetime": -1 })
        .build();

    let stream = collection
        .find(doc! { "i_datetime": { "$lt": dt } }, options)
        .await
        .expect("find should succeed");
     
    let mut images: Vec<Image> = Vec::new();
    let mut stream = stream.into_stream();

    while let Some(image) = stream.try_next().await.unwrap() {
        images.push(image);
    }

    HttpResponse::Ok().json(images)    
}
