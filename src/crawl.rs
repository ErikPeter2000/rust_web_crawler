use log::{error, info};
use reqwest::Client;
use rusqlite::Connection;
use scraper::{Html, Selector};
use std::collections::VecDeque;
use std::error::Error;
use url::Url;

use crate::save_page;
use crate::block_url::check_page_is_valid;

pub async fn crawl(
    url_queue: &mut VecDeque<String>,
    client: &Client,
    connection: &Connection,
    url: &str
) -> Result<Vec<String>, Box<dyn Error>> {
    info!("Crawling {}", url);
    
    // Check if the URL has already been crawled
    println!("url: {url}");
    if is_url_crawled(connection, url)? || check_page_is_valid(client, connection, url).await? {
        info!("Skipping {}", url);
        return Ok(vec![]);
    }

    // Fetch page
    let result = client.get(url).send().await?;
    let html = result.text().await?;

    // Prepare to parse HTML
    let document = Html::parse_document(&html);
    let selector = Selector::parse("a").unwrap();

    // Select all links
    let urls: Vec<String> = document
        .select(&selector)
        .filter_map(|element| element.value().attr("href"))
        .filter_map(|href| {
            Url::parse(href)
                .ok()
                .or_else(|| Url::parse(&format!("{}/{}", url, href)).ok())
        })
        .map(|url| url.to_string())
        .collect();

    // Enqueue all links
    urls.iter().for_each(|url| url_queue.push_back(url.clone()));

    // Save page
    let filename = save_page(connection, url, &html, &urls)
        .inspect_err(|e| error!("Failed to save page {}", e))
        .unwrap();

    info!("Found {} links: {}", urls.len(), urls.join(", "));
    info!("Saved page to {}", filename);

    Ok(urls)
}

fn is_url_crawled(connection: &Connection, url: &str) -> Result<bool, Box<dyn Error>> {
    // Query database to check if URL exists
    let exists = connection.query_row(
        "SELECT EXISTS(SELECT 1 FROM Page WHERE Url = ?)",
        &[url],
        |row| row.get(0),
    )?;

    Ok(exists)
}