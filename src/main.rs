use clap::{Arg, ArgAction, Command};
use env_logger;
use log::{error, info};
use rusqlite::Connection;
use std::error::Error;
use std::fs;
use url::Url;

mod crawler;
mod unique_queue;
use crate::crawler::Crawler;

const SAVE_DIR: &str = "pages";
const DB_NAME: &str = "web_crawler.db";
const CREATE_SCRIPT: &str = "scripts/create.sql";

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    std::env::set_var("RUST_LOG", "info");
    env_logger::init();

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
                .default_value("16"),
        )
        .arg(
            Arg::new("url")
                .short('u')
                .long("url")
                .help("URL to start crawling")
                .required(true),
        )
        .arg(
            Arg::new("ignore-robots")
                .short('i')
                .long("ignore-robots")
                .help("Ignore robots.txt rules when crawling")
                .action(ArgAction::SetTrue),
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
    let connection = Connection::open(DB_NAME).unwrap();
    let iterations = arguments.get_one::<u32>("depth").unwrap();
    let mut crawler = Crawler::new(start_url, "web_crawler_homework", Some(arguments.get_flag("ignore-robots")));

    for _ in 0..*iterations {
        let result = crawler.crawl().await;
        match result {
            Ok(true) => {
                info!("Crawling completed successfully.");
            }
            Ok(false) => {
                info!("No more URLs to crawl.");
                break;
            }
            Err(e) => {
                error!("Error during crawling: {}", e);
            }
        }
    }

    connection.close().unwrap();

    Ok(())
}

fn initialize_data_store() -> Result<(), Box<dyn Error>> {
    info!("Initializing database...");

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
