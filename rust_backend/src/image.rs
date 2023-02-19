use crate::scan_im::CLASSES_FILE;
use crate::scan_im::Image;

use actix_web::{HttpResponse, get, web};
use std::fs::File;
use std::io::{BufRead, BufReader};
use mongodb::{bson::doc, Client, Collection, options::FindOptions};
use futures::stream::TryStreamExt;
use serde::{Deserialize, Deserializer, Serialize};
use chrono::{DateTime, Utc};
use std::io::Read;

const DB_NAME: &str = "zyscan";
const COLL_NAME: &str = "images";
const MAJORITY_CLASS_PROBABILITY: f64 = 0.08;

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

#[derive(Deserialize)]
struct ClassQuery {
    class: String,
    dt: UtcDateTime,
}

#[get("/api/images-by-class")]
async fn get_images_by_class(client: web::Data<Client>, params: web::Query<ClassQuery>) -> HttpResponse {
    
    let dt = mongodb::bson::DateTime::from_millis(params.dt.0.timestamp_millis());

    let collection: Collection<Image> = client.database(DB_NAME).collection(COLL_NAME);
    let options = FindOptions::builder()
        .limit(50)
        .sort(doc! { "i_datetime": -1 })
        .projection(
            doc! {
                "classification_result": 0,
            }
        ).build();

    let stream = collection.find(
        doc! {
            "i_datetime": { "$lt": dt },
            "classification_result": {
                "$elemMatch": {
                    "name": params.class.clone(),
                    "prob": { "$gt": MAJORITY_CLASS_PROBABILITY }
                }
            }
        },
        options,
        ).await
        .expect("find should succeed");
    
    let mut images: Vec<Image> = Vec::new();
    let mut stream = stream.into_stream();

    while let Some(image) = stream.try_next().await.unwrap() {
        images.push(image);
    }

    HttpResponse::Ok().json(images)    
}


#[derive(Deserialize, Serialize, Debug)]
struct ClassObj {
    name: String,
}

#[get("/images-by-class")]
async fn get_images_by_class_html() -> HttpResponse {
    let mut file = File::open(CLASSES_FILE).unwrap();
    let mut data = String::new();
    file.read_to_string(&mut data).unwrap();

    let classes: Vec<ClassObj> = serde_json::from_str(&data).unwrap();
    // Generate a html document contating the classes
    let mut html = String::from("<html><body><ul>");
    for class in classes {
        html.push_str(&format!("<li><a href=\"/images-by-class/{}\">{}</a></li>", class.name, class.name));
    }
    html.push_str("</ul></body></html>");

    HttpResponse::Ok().content_type("text/html").body(html)
}


#[get("/images-by-class/{class}")]
async fn get_images_belonging_to_a_class_html(client: web::Data<Client>, class: web::Path<String>) -> HttpResponse {
    // Generate a html document contating the images that belong to the class
    let collection: Collection<Image> = client.database(DB_NAME).collection(COLL_NAME);
    let options = FindOptions::builder()
        .limit(50)
        .sort(doc! { "i_datetime": -1 })
        .projection(
            doc! {
                "classification_result": 0,
            }
        ).build();

    let stream = collection.find(
        doc! {
            "classification_result": {
                "$elemMatch": {
                    "name": class.clone(),
                    "prob": { "$gt": MAJORITY_CLASS_PROBABILITY }
                }
            }
        },
        options,
        ).await
        .expect("find should succeed");
    
    let mut stream = stream.into_stream();
    // Do that in single line
    let mut html = format!("<html><body><h1>Images belonging to class {}</h1><ul>", class.clone());

    while let Some(image) = stream.try_next().await.unwrap() {
        let filename = image.i_path.split('/').last().unwrap();
        html.push_str(&format!("<li><a href=\"/images/{}\"><img src=\"/thumbnails/{}.jpg\"/></a></li>", filename, image._id));
    }

    HttpResponse::Ok().content_type("text/html").body(html)
}
