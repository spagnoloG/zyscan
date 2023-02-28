use crate::config::AppConfig;
use log::{error, info, warn};
use mongodb::{bson::doc, Client, Collection};
use opencv::core::{Mat, Size, Vector};
use opencv::imgcodecs::{imread, imwrite, IMREAD_UNCHANGED};
use opencv::imgproc::{resize, INTER_AREA};
use opencv::prelude::*;
use serde::{Deserialize, Serialize};
use std::path::Path;
use std::process::Command;
use mongodb::bson::DateTime as MongoDateTime;

const DB_NAME: &str = "zyscan";
const COLL_NAME: &str = "images";
const THUMBNAIL_SIZE: u32 = 100;
const PYTHON_CLASSIFICATION_SCRIPT: &str = "./src_py/classify.py";
pub const THUMBNAIL_LOCATION: &str = "./thumbnails/";

#[derive(Clone, Debug, PartialEq, Deserialize, Serialize)]
pub struct ClassificationScriptResult {
    pub image_file: String,
    pub milvus_id: u64,
}

#[derive(Clone, Debug, PartialEq, Deserialize, Serialize)]
pub struct Image {
    pub _id: mongodb::bson::oid::ObjectId,
    pub milvus_id: u64,
    pub i_path: String,
    pub i_width: u32,
    pub i_height: u32,
    pub i_longitude: String,
    pub i_latitude: String,
    pub i_altitude: f64,
    pub i_datetime: MongoDateTime,
    pub c_lens_make: String,
    pub c_lens_model: String,
}

pub async fn load_images(config: AppConfig) {
    let client = Client::with_uri_str(config.db_connection.clone())
        .await
        .expect("failed to connect");
    let db = client.database(DB_NAME);
    let collection: Collection<Image> = db.collection(COLL_NAME);

    // check if thumbnail directory exists
    if !Path::new(THUMBNAIL_LOCATION).exists() {
        std::fs::create_dir(THUMBNAIL_LOCATION).unwrap();
    }

    for folder in &config.scan_folders {
        _load_images(config.clone(), &collection, folder).await;
    }
}

async fn _load_images(config: AppConfig, collection: &Collection<Image>, path: &str) -> () {
    // firstly list all the files in the path recursively
    // then check if the file is already in the database
    // if not, add it to the database

    let csr: Vec<ClassificationScriptResult> = classify_images(config, path);
    for entry in csr {
            // Firstly check if the image is already in the database
            if let Ok(Some(_)) = collection
                .find_one(doc! { "i_path": &entry.image_file }, None)
                .await
            {
                info!("Image {} already in database", entry.image_file);
                continue;
            }

            let file = std::fs::File::open(&entry.image_file).unwrap();
            let exif_data_result = exif::Reader::new()
                .read_from_container(&mut std::io::BufReader::new(file));
            
            let exif_data = match exif_data_result {
               Ok(exif_data) => exif_data,
               Err(exif::Error::InvalidFormat(_)) => {
                   warn!("Invalid exif format for file: {}", entry.image_file);
                   continue;
               },
               Err(_) => {
                   error!("Error while reading fxif data for file: {}", entry.image_file);
                   continue;
               }
            };

            let image = Image {
                _id: mongodb::bson::oid::ObjectId::new(),
                milvus_id: entry.milvus_id,
                i_path: entry.image_file.clone(),
                i_datetime: parse_exif_datetime(&exif_data),
                i_width: exif_data
                    .get_field(exif::Tag::ImageWidth, exif::In::PRIMARY)
                    .map(|f| f.display_value().to_string())
                    .unwrap_or_else(|| "0".to_string())
                    .parse::<u32>()
                    .unwrap_or(0),
                i_height: exif_data
                    .get_field(exif::Tag::ImageLength, exif::In::PRIMARY)
                    .map(|f| f.display_value().to_string())
                    .unwrap_or_else(|| "0".to_string())
                    .parse::<u32>()
                    .unwrap_or(0),
                i_longitude: exif_data
                    .get_field(exif::Tag::GPSLongitude, exif::In::PRIMARY)
                    .map(|f| f.display_value().to_string())
                    .unwrap_or_else(|| "0".to_string()),
                i_latitude: exif_data
                    .get_field(exif::Tag::GPSLatitude, exif::In::PRIMARY)
                    .map(|f| f.display_value().to_string())
                    .unwrap_or_else(|| "0".to_string()),
                i_altitude: exif_data
                    .get_field(exif::Tag::GPSAltitude, exif::In::PRIMARY)
                    .map(|f| f.display_value().to_string())
                    .unwrap_or_else(|| "0".to_string())
                    .parse::<f64>()
                    .unwrap_or(0.0),
                c_lens_make: exif_data
                    .get_field(exif::Tag::Make, exif::In::PRIMARY)
                    .map(|f| f.display_value().to_string())
                    .unwrap_or_else(|| "0".to_string()),
                c_lens_model: exif_data
                    .get_field(exif::Tag::Model, exif::In::PRIMARY)
                    .map(|f| f.display_value().to_string())
                    .unwrap_or_else(|| "0".to_string()),
            };

            // insert the image into the database and get the id
            let ins_result = collection.insert_one(image, None).await.unwrap();
            let i_id: String = ins_result.inserted_id.as_object_id().unwrap().to_hex();

            // Generate the thumbnail
            generate_thumbnail(&entry.image_file, &i_id);

            info!("Added image {} to database", entry.image_file);
    }
}

fn parse_exif_datetime(exif_data: &exif::Exif) -> MongoDateTime {
    let mut timestamp = 0;
    if let Some(field) = exif_data.get_field(exif::Tag::DateTime, exif::In::PRIMARY) {
        match field.value {
            exif::Value::Ascii(ref vec) if !vec.is_empty() => {
                if let Ok(datetime) = exif::DateTime::from_ascii(&vec[0]) {
                    // create a new chrono DateTime
                    let datetime = chrono::NaiveDateTime::new(
                        chrono::NaiveDate::from_ymd_opt(
                            datetime.year as i32,
                            datetime.month as u32,
                            datetime.day as u32,
                            ).unwrap_or_else(
                                || chrono::NaiveDate::from_ymd_opt(1970, 1, 1).unwrap()
                                ),
                                chrono::NaiveTime::from_hms_opt(
                                    datetime.hour as u32,
                                    datetime.minute as u32,
                                    datetime.second as u32,
                                    ).unwrap_or_else(
                                        || chrono::NaiveTime::from_hms_opt(0, 0, 0).unwrap()
                                        )
                                );
                    let datetime = chrono::DateTime::<chrono::Utc>::from_utc(datetime, chrono::Utc);
                    timestamp = datetime.timestamp_millis();
                }
            },
            _ => {},
        }
    }
    MongoDateTime::from_millis((timestamp as u64).try_into().expect("Failed to convert timestamp to u64"))
}

fn generate_thumbnail(image_path: &str, i_id: &str) -> bool {
    let img = imread(image_path, IMREAD_UNCHANGED).unwrap();

    // get the aspect ratio
    let aspect_ratio = img.cols() as f32 / img.rows() as f32;

    // determine the new width and height
    let (new_width, new_height) = (
        THUMBNAIL_SIZE,
        (THUMBNAIL_SIZE as f32 / aspect_ratio) as u32,
    );

    // create a new image with the new width and height
    let mut resized = Mat::default();
    let params: Vector<i32> = Vector::new();
    resize(
        &img,
        &mut resized,
        Size::new(
            new_width.try_into().unwrap(),
            new_height.try_into().unwrap(),
        ),
        0.0,
        0.0,
        INTER_AREA,
    )
    .unwrap();

    let thumbnail_location: String = format!("{}{}{}", THUMBNAIL_LOCATION, i_id, ".jpg");

    imwrite(&thumbnail_location, &resized, &params).unwrap();

    true
}

fn classify_images(config: AppConfig, images_dir: &str) -> Vec<ClassificationScriptResult> {
    let output = Command::new(config.python_venv_path)
        .arg(PYTHON_CLASSIFICATION_SCRIPT)
        .arg("--images_dir")
        .arg(images_dir)
        .arg("--device")
        .arg("cuda")
        .output()
        .expect("failed to execute process");

    if !output.status.success() {
        error!("Error: {}", String::from_utf8_lossy(&output.stderr));
    }

    let output_string = String::from_utf8(output.stdout).unwrap();
    let classification_result: Vec<ClassificationScriptResult> =
        serde_json::from_str(&output_string).expect("Error while parsing classification result");

    classification_result
}
