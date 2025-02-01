use clap::{Arg, ArgAction, Command};
use env_logger;
use hex::encode;
use log::{error, info};
use reqwest::Client;
use rusqlite::Connection;
use scraper::{Html, Selector};
use sha2::{Digest, Sha256};
use std::collections::VecDeque;
use std::error::Error;
use std::fs;
use url::Url;

const DB_NAME: &str = "web_crawler.db";
const CREATE_SCRIPT: &str = "scripts/create.sql";
const SAVE_DIR: &str = "pages";

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::init(); // Initialize logger

    // Specify command line arguments
    let arguments = Command::new("web_crawler_homework")
        .version("0.1.0")
        .author("Erik")
        .about("Web crawler homework")
        .arg(
            Arg::new("clean")
                .short('c')
                .long("clean")
                .help("Cleans the database")
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new("depth")
                .short('d')
                .long("depth")
                .help("Number of iterations to crawl")
                .value_parser(clap::value_parser!(u32))
                .default_value("16")
        )
        .arg(
            Arg::new("url")
                .short('u')
                .long("url")
                .help("URL to start crawling")
                .required(true)
        )
        .get_matches();
    
    // Initialize database if necessary
    if arguments.get_flag("clean") || !fs::metadata(DB_NAME).is_ok() {
        initialize_data_store()
            .inspect_err(|e| error!("Failed to create database {}", e))
            .unwrap();
    }

    // Parse start URL
    let start_url = arguments.get_one::<String>("url").unwrap();
    if !Url::parse(&start_url).is_ok() {
        error!("\"{}\" is not a valid URL", start_url);
        return Ok(());
    }

    // Start crawling
    let client = Client::new();
    let connection = Connection::open(DB_NAME).unwrap();
    let iterations = arguments.get_one::<u32>("depth").unwrap();
    let mut url_queue: VecDeque<_> = VecDeque::new();

    // Breadth-first search
    url_queue.push_back(start_url.to_string());
    for _ in 0..*iterations {
        if url_queue.is_empty() {
            break;
        }

        // Dequeue and crawl next URL
        let url = url_queue.pop_front().unwrap();
        crawl(&mut url_queue, &client, &connection, &url)
            .await
            .inspect_err(|e| error!("Failed to crawl {}", e))
            .unwrap();
    }

    connection.close().unwrap();

    Ok(())
}

fn initialize_data_store() -> Result<(), Box<dyn Error>> {
    // Remove existing pages
    if fs::metadata(SAVE_DIR).is_ok() {
        fs::remove_dir_all(SAVE_DIR)?;
    }
    fs::create_dir(SAVE_DIR)?;

    // Remove existing database
    if fs::metadata(DB_NAME).is_ok() {
        fs::remove_file(DB_NAME)?;
    }

    // Create database
    let create_script = fs::read_to_string(CREATE_SCRIPT)?;
    let connection = Connection::open(DB_NAME)?;
    connection.execute_batch(&create_script)?;
    connection.close().unwrap();

    Ok(())
}

fn save_page(
    connection: &Connection,
    url: &str,
    html: &str,
    links: &Vec<String>,
) -> Result<String, Box<dyn Error>> {
    // Hash HTML content
    let mut hasher = Sha256::new();
    hasher.update(html);
    let hash = encode(hasher.finalize());

    // Save HTML content to a file    
    let filename = format!("{}.html", hash);
    let filepath = format!("{}/{}", SAVE_DIR, filename);
    fs::write(filepath, html)?;

    // Save page to the database
    connection.execute("INSERT INTO Page (Url, Hash) VALUES (?, ?)", &[url, &hash])?;
    let page_id = connection.last_insert_rowid();

    // Save links to the database
    for link in links {
        connection.execute(
            "INSERT OR IGNORE INTO PageLink (PageId, Url) VALUES (?, ?)",
            &[&page_id.to_string(), &link.to_string()],
        )?;
    }

    Ok(filename)
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

async fn crawl(
    url_queue: &mut VecDeque<String>,
    client: &Client,
    connection: &Connection,
    url: &str
) -> Result<Vec<String>, Box<dyn Error>> {
    info!("Crawling {}", url);
    
    // Check if the URL has already been crawled
    if is_url_crawled(connection, url)? {
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
