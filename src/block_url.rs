use log::info;
use reqwest::Client;
use rusqlite::Connection;
use std::{error::Error, io};
use url::Url;

pub struct BlockedList {
    root_id: i64,
    sub_url: Vec<String>,
    modified: String
}

pub async fn check_page_is_valid(client: &Client, connection: &Connection, url: &str) -> Result<bool, Box<dyn Error>> {
    // TODO: check non HTML pages
    if let Err(e) = check_blocked_list(connection, url).await { return Err(e) }
    check_robots(client, url).await
    // Ok(true)
}

async fn check_robots(client: &Client, url: &str) -> Result<bool, Box<dyn Error>> {
    match parse_robots(client, url).await {
        Ok(robots_parsed) => {
            for line in robots_parsed {
                let mut success = true;
                if let (Some(k), Some(v)) = line {
                    match k.trim() {
                        "User-agent" => {
                            // TODO: implement user-agent checking
                        },
                        "Disallow" => {
                            let parsed_url = Url::parse(url)?;
                            let url_subdir = parsed_url.path();
        
                            if url_subdir.trim_start_matches('/') == v.trim_start_matches('/') {
                                success = false;
                            }
                            // TODO: check if url is within db, and if not, add it to database
                        },
                        _ => {}
                    }
                } else {
                    info!("Robots.txt couldn't be parsed");
                    success = false; // failsafe for broken robots.txt
                }
                if !success { return Ok(false); }
            }
            Ok(true)
        },
        Err(e) => Err(e)
    }
}

async fn check_blocked_list(connection: &Connection, url: &str) -> Result<bool, Box<dyn Error>> {
    // TODO: check timestamp and compare
    let blocked_urls = get_blocked_list(connection, url).await?; 
    let sub_path = get_subpath(url)?;

    for url in blocked_urls.sub_url {
        if url == sub_path { return Ok(false) }
    }
    Ok(true)
}

async fn get_blocked_list(connection: &Connection, url: &str) -> Result<BlockedList, Box<dyn Error>> {
    let root_url = get_url_path(url, None)?;
    let root: (i64, String) = connection.query_row(
        "SELECT Id, Modified FROM RootUrl WHERE Url = ?1",
        [&root_url],
        |row| {
            let id = row.get(0)?;
            let modified = row.get(1)?;
            Ok((id, modified))
        },
    )?;
    
    let blocked_list: Vec<String> = connection
        .prepare("SELECT SubUrl FROM BlockedUrl WHERE RootId = ?")?
        .query_map([root.0], |row| row.get::<_, String>(0))?
        .collect::<Result<Vec<_>, _>>()?;
    
    Ok(BlockedList {
        root_id: root.0,
        sub_url: blocked_list,
        modified: root.1,
    })
}

fn get_subpath(url: &str) -> Result<String, Box<dyn std::error::Error>> {
    let parsed_url = Url::parse(url)?;
    let subpath = parsed_url.path().trim_start_matches('/');
    Ok(subpath.to_string())
}

fn get_url_path(url: &str, path: Option<&str>) -> Result<String, Box<dyn Error>>  {
    let local_path = match path {
        Some(path) => path, 
        None => "/"
    };
    
    let parsed = Url::parse(url)?;
    let mut robots_url = parsed.clone();
    
    robots_url.set_path(local_path);
    robots_url.set_query(None);
    robots_url.set_fragment(None);
    Ok(robots_url.as_str().to_string())
}

async fn add_blocked_to_db(connection: &Connection) {
    // TODO: add new blocked urls to db
}

async fn get_robots(client: &Client, url: &str) -> Result<String, Box<dyn Error>> {
    let robots_url = get_url_path(url, Some("/robots.txt"))?;
    let result = client.get(robots_url).send().await?;
    if !result.status().is_success() {
        return Err(Box::new(io::Error::new(io::ErrorKind::Other, "No robots.txt" ))); // failsafe for no robots.txt
    }

    Ok(result.text().await?)
}

async fn parse_robots(client: &Client, url: &str) -> Result<Vec<(Option<String>, Option<String>)>, Box<dyn Error>> {
    match get_robots(client, url).await {
        Ok(text) => {
            let mut robots_parsed: Vec<(Option<String>, Option<String>)> = Vec::new();
            for line in text.lines() {
                let parts: Vec<&str> = line.splitn(2, ":").collect();

                if let [k, v] = &parts[..] {
                    robots_parsed.push((Some(k.trim().to_string()), Some(v.trim().to_string())));
                } else {
                    robots_parsed.push((None, None));
                };
            }
            Ok(robots_parsed)
        },
        Err(e) => Err(e)
    }
}