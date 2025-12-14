/*
input:
    https://matgomes.com/path1.jpg
    path2.png
    ..
    path012931023.svg


-> download them to a directory
-> output json with info
{
    "uuid-qwe123-qwe123123.jpg": {
        "link": "https://matgomes.com/path1.jpg",
        "alt": "whatever text we have"
    },

    ...
}
*/

use anyhow::{anyhow, bail, Result};
use std::collections::HashMap;
use std::path::Path;
use std::time::Duration;

use log2::*;
use reqwest::{Client, Response};
use tokio::fs::{create_dir, File};
use tokio::io::AsyncWriteExt;
use tokio_stream::StreamExt;
use tokio::time::sleep;
use url::Url;
use uuid::Uuid;

use crate::model::{Image, LinkGraph};

/// Convert all the images in the found scraped
/// links to the (Uuid name, image) format
pub fn convert_links_to_images(links: &LinkGraph) -> HashMap<String, Image> {
    links
        .into_iter()
        .flat_map(|(_, link)| link.images.clone())
        .map(|img| (Uuid::new_v4().to_string(), img))
        .collect()
}

async fn download_image(link: &str, destination: &str, client: &Client) -> Result<()> {
    const MAX_RETRIES: u32 = 3;
    let mut last_error = None;

    for attempt in 0..MAX_RETRIES {
        match try_download_image(link, destination, client).await {
            Ok(()) => return Ok(()),
            Err(e) => {
                last_error = Some(e);
                if attempt < MAX_RETRIES - 1 {
                    sleep(Duration::from_millis(500 * (attempt + 1) as u64)).await;
                }
            }
        }
    }

    Err(last_error.unwrap_or_else(|| anyhow!("download failed after {} attempts", MAX_RETRIES)))
}

async fn try_download_image(link: &str, destination: &str, client: &Client) -> Result<()> {
    let res = client.get(link).send().await?;
    let extension = get_extension(&res)?;
    let mut file = File::create(format!("{}.{}", destination, extension)).await?;
    let mut stream = res.bytes_stream();

    while let Some(item) = stream.next().await {
        file.write_all(&item?).await?;
    }

    Ok(())
}

fn get_extension(res: &Response) -> Result<String> {
    if let Some(content_type) = res.headers().get("content-type").and_then(|h| h.to_str().ok()) {
        if let Some(ext) = match content_type {
            "image/gif" => Some("gif"),
            "image/jpeg" | "image/jpg" => Some("jpg"),
            "image/png" => Some("png"),
            "image/svg+xml" => Some("svg"),
            "image/webp" => Some("webp"),
            "image/tiff" | "image/tif" => Some("tif"),
            "image/avif" => Some("avif"),
            "image/bmp" => Some("bmp"),
            _ => None,
        } {
            return Ok(ext.to_string());
        }
    }

    if let Ok(url) = res.url().as_str().parse::<Url>() {
        if let Some(path) = url.path_segments() {
            if let Some(last) = path.last() {
                if let Some(dot_idx) = last.rfind('.') {
                    let ext = &last[dot_idx + 1..];
                    if !ext.is_empty() && ext.len() <= 5 {
                        return Ok(ext.to_lowercase());
                    }
                }
            }
        }
    }

    bail!("could not determine image extension")
}

/// Takes in the hashmap (image name, image info), downloads the images
/// and saves them to disk.
pub async fn download_images(
    images: &HashMap<String, Image>,
    save_directory: &str,
    max_links: u64,
) -> Result<()> {
    let directory_path = Path::new(&save_directory);
    if !directory_path.is_dir() {
        // bail!("given save directory is invalid");
        create_dir(directory_path).await?;
    }

    let client = reqwest::Client::new();
    for (name, image) in images.iter().take(max_links as usize) {
        // directory + name + extension
        let destination_path = directory_path.join(name);
        let destination = destination_path
            .to_str()
            .ok_or_else(|| anyhow!("could not get destination path"))?;

        if let Err(e) = download_image(&image.link, destination, &client).await {
            error!("Could not download image {}, error: {}", image.link, e);
        }
    }

    Ok(())
}

// #[cfg(test)]
// mod tests {
//     // use crate::model::Image;

//     #[test]
//     fn a_unit_test() {
//         assert!(true);
//     }
// }
