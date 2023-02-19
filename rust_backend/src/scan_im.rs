use crate::config::AppConfig;
use async_recursion::async_recursion;
use log::{error, info, warn};
use mongodb::{bson::doc, Client, Collection};
use opencv::core::{Mat, Size, Vector};
use opencv::imgcodecs::{imread, imwrite, IMREAD_UNCHANGED};
use opencv::imgproc::{resize, INTER_AREA};
use opencv::prelude::*;
use serde::{Deserialize, Serialize};
use std::path::Path;
use std::process::Command;

const DB_NAME: &str = "zyscan";
const COLL_NAME: &str = "images";
const THUMBNAIL_LOCATION: &str = "./thumbnails/";
const THUMBNAIL_SIZE: u32 = 100;
const PYTHON_CLASSIFICATION_SCRIPT: &str = "./src_py/classify.py";
const CLASSES_FILE: &str = "/home/gasperspagnolo/Documents/faks_git/diplomska-cv/rust_backend/assests/annotations/classes.json";

#[derive(Clone, Debug, PartialEq, Deserialize, Serialize)]
pub struct ClassificationResult {
    pub name: String,
    pub prob: f64,
}

#[derive(Clone, Debug, PartialEq, Deserialize, Serialize)]
pub struct Image {
    pub i_path: String,
    pub i_width: u32,
    pub i_height: u32,
    pub i_longitude: String,
    pub i_latitude: String,
    pub i_altitude: f64,
    pub i_datetime: String,
    pub c_lens_make: String,
    pub c_lens_model: String,
    pub classification_result: Vec<ClassificationResult>,
}

pub async fn load_images(config: AppConfig, path: &str) {
    let client = Client::with_uri_str(config.db_connection.clone())
        .await
        .expect("failed to connect");
    let db = client.database(DB_NAME);
    let collection: Collection<Image> = db.collection(COLL_NAME);

    // check if thumbnail directory exists
    if !Path::new(THUMBNAIL_LOCATION).exists() {
        std::fs::create_dir(THUMBNAIL_LOCATION).unwrap();
    }

    info!("Loading images from {}", path);

    _load_images(config, &collection, path).await;
}

#[async_recursion]
async fn _load_images(config: AppConfig, collection: &Collection<Image>, path: &str) {
    // firstly list all the files in the path recursively
    // then check if the file is already in the database
    // if not, add it to the database

    for entry in std::fs::read_dir(path).unwrap() {
        let entry = entry.unwrap();
        let entry_path = entry.path().clone();
        // print entry_path
        if entry_path.is_file() {
            // check if the file is an image jpg, jpeg, png
            if !entry_path.to_str().unwrap().ends_with(".jpg")
                && !entry_path.to_str().unwrap().ends_with(".jpeg")
                && !entry_path.to_str().unwrap().ends_with(".png")
            {
                continue;
            }

            // Firstly check if the image is already in the database
            if let Ok(Some(_)) = collection
                .find_one(doc! { "i_path": entry_path.to_str().unwrap() }, None)
                .await
            {
                info!("Image {} already in database", entry_path.to_str().unwrap());
                continue;
            }

            let file = std::fs::File::open(&entry_path).unwrap();
            let exif_data = exif::Reader::new()
                .read_from_container(&mut std::io::BufReader::new(file))
                .unwrap();

            let classification_result =
                classify_image(config.clone(), entry_path.to_str().unwrap());

            let image = Image {
                i_path: entry_path.to_str().unwrap().to_string(),
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
                i_datetime: exif_data
                    .get_field(exif::Tag::DateTime, exif::In::PRIMARY)
                    .map(|f| f.display_value().to_string())
                    .unwrap_or_else(|| "0".to_string()),
                c_lens_make: exif_data
                    .get_field(exif::Tag::Make, exif::In::PRIMARY)
                    .map(|f| f.display_value().to_string())
                    .unwrap_or_else(|| "0".to_string()),
                c_lens_model: exif_data
                    .get_field(exif::Tag::Model, exif::In::PRIMARY)
                    .map(|f| f.display_value().to_string())
                    .unwrap_or_else(|| "0".to_string()),
                classification_result: classification_result.clone(),
            };

            // insert the image into the database and get the id
            let ins_result = collection.insert_one(image, None).await.unwrap();
            let i_id: String = ins_result.inserted_id.as_object_id().unwrap().to_hex();

            // Generate the thumbnail
            generate_thumbnail(entry_path.to_str().unwrap(), &i_id);

            info!("Added image {} to database", entry_path.to_str().unwrap());
        } else if entry_path.is_dir() {
            // recursively call this function untill we reach files
            _load_images(config.clone(), collection, entry_path.to_str().unwrap()).await;
        } else {
            warn!(
                "Not sure what you provied me here {}",
                entry_path.to_str().unwrap()
            );
        }
    }
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

fn classify_image(config: AppConfig, image_path: &str) -> Vec<ClassificationResult> {
    let output = Command::new(config.python_venv_path)
        .arg(PYTHON_CLASSIFICATION_SCRIPT)
        .arg("--image_file")
        .arg(image_path)
        .arg("--classes_file")
        .arg(CLASSES_FILE)
        .arg("--device")
        .arg("cuda")
        .output()
        .expect("failed to execute process");

    if !output.status.success() {
        error!("Error while classifying image {}", image_path);
        error!("Error: {}", String::from_utf8_lossy(&output.stderr));
    }

    let output_string = String::from_utf8(output.stdout).unwrap();
    let classification_result: Vec<ClassificationResult> =
        serde_json::from_str(&output_string).expect("Error while parsing classification result");

    classification_result
}
