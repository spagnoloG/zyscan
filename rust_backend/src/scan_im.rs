use mongodb::{bson::doc, Client, Collection};
use serde::{Deserialize, Serialize};
use async_recursion::async_recursion;
use log::{warn, info};
use opencv::prelude::*;
use opencv::core::{Mat, Size, Vector};
use opencv::imgcodecs::{IMREAD_UNCHANGED, imread, imwrite};
use opencv::imgproc::{resize, INTER_LINEAR};
use std::path::Path;

const DB_NAME: &str = "zyscan";
const COLL_NAME: &str = "images";
const THUMBNAIL_LOCATION: &str = "./thumbnails/";
const THUMBNAIL_SIZE: u32 = 100;

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
}

pub async fn load_images(path: &str, db_connection_url: &str) {
    let client = Client::with_uri_str(db_connection_url).await.expect("failed to connect");
    let db = client.database(DB_NAME);
    let collection: Collection<Image> = db.collection(COLL_NAME);
    
    // check if thumbnail directory exists
    if !Path::new(THUMBNAIL_LOCATION).exists() {
        std::fs::create_dir(THUMBNAIL_LOCATION).unwrap();
    }
    
    info!("Loading images from {}", path);

    _load_images(path, &collection).await;
}

#[async_recursion]
async fn _load_images(path: &str, collection: &Collection<Image>) {
    // firstly list all the files in the path recursively
    // then check if the file is already in the database
    // if not, add it to the database
    
    for entry in std::fs::read_dir(path).unwrap() {
        let entry = entry.unwrap();
        let entry_path = entry.path().clone();
        // print entry_path
        if entry_path.is_file() {

            // Firstly check if the image is already in the database
            if let Ok(Some(_)) = collection.find_one(doc! { "i_path": entry_path.to_str().unwrap() }, None).await {
                info!("Image {} already in database", entry_path.to_str().unwrap());
                continue;
            }

            let file = std::fs::File::open(&entry_path).unwrap();
            let exif_data = exif::Reader::new()
                .read_from_container(&mut std::io::BufReader::new(file))
                .unwrap();

            let image = Image {
                i_path: entry_path.to_str().unwrap().to_string(),
                i_width: exif_data.get_field(exif::Tag::ImageWidth, exif::In::PRIMARY).map(
                    |f| f.display_value().to_string()).unwrap_or_else(|| "0".to_string()).parse::<u32>().unwrap_or(0),
                i_height: exif_data.get_field(exif::Tag::ImageLength, exif::In::PRIMARY).map(
                    |f| f.display_value().to_string()).unwrap_or_else(|| "0".to_string()).parse::<u32>().unwrap_or(0),
                i_longitude: exif_data.get_field(exif::Tag::GPSLongitude, exif::In::PRIMARY).map(
                    |f| f.display_value().to_string()).unwrap_or_else(|| "0".to_string()),
                i_latitude: exif_data.get_field(exif::Tag::GPSLatitude, exif::In::PRIMARY).map(
                    |f| f.display_value().to_string()).unwrap_or_else(|| "0".to_string()),
                i_altitude: exif_data.get_field(exif::Tag::GPSAltitude, exif::In::PRIMARY).map(
                    |f| f.display_value().to_string()).unwrap_or_else(|| "0".to_string()).parse::<f64>().unwrap_or(0.0),
                i_datetime: exif_data.get_field(exif::Tag::DateTime, exif::In::PRIMARY).map(
                    |f| f.display_value().to_string()).unwrap_or_else(|| "0".to_string()), 
                c_lens_make: exif_data.get_field(exif::Tag::Make, exif::In::PRIMARY).map(
                    |f| f.display_value().to_string()).unwrap_or_else(|| "0".to_string()),
                c_lens_model: exif_data.get_field(exif::Tag::Model, exif::In::PRIMARY).map(
                    |f| f.display_value().to_string()).unwrap_or_else(|| "0".to_string()), 
            };
  
            // insert the image into the database
            collection.insert_one(image, None).await.unwrap();

            // Resize the image to the thumbnail size but keep the aspect ratio
            let img = imread(entry_path.to_str().unwrap(), IMREAD_UNCHANGED).unwrap();

            // get the aspect ratio
            let aspect_ratio = img.cols() as f32 / img.rows() as f32;
            
            // determine the new width and height
            let (new_width, new_height) = if img.cols() > img.rows() {
                (THUMBNAIL_SIZE, (THUMBNAIL_SIZE as f32 / aspect_ratio) as u32)
            } else {
                ((THUMBNAIL_SIZE as f32 * aspect_ratio) as u32, THUMBNAIL_SIZE)
            };

            // create a new image with the new width and height
            let mut resized = Mat::default();
            let params: Vector<i32> = Vector::new();
            resize(&img, &mut resized, Size::new(new_width.try_into().unwrap(), new_height.try_into().unwrap()), 0.0, 0.0, INTER_LINEAR).unwrap();
            
            let thumbnail_location: String = format!("{}{}", THUMBNAIL_LOCATION, entry_path.file_name().unwrap().to_str().unwrap());

            imwrite(&thumbnail_location, &resized, &params).unwrap();

            info!("Added image {} to database", entry_path.to_str().unwrap());

        } else if entry_path.is_dir() {
            // recursively call this function untill we reach files
            _load_images(entry_path.to_str().unwrap(), collection).await;
        } else {
            warn!("Not sure what you provied me here {}", entry_path.to_str().unwrap());
        }
    }  
}
