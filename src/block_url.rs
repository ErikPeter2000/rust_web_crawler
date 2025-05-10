use reqwest::Client;
use rusqlite::Connection;
use std::{error::Error, io};
use url::Url;

pub struct BlockedUrls {

}

pub async fn check_page_is_valid(client: &Client, connection: &Connection, url: &str) -> Result<bool, Box<dyn Error>> {
    // TODO: check non HTML pages
    check_robots(client, connection, url).await
    // Ok(true)
}

pub async fn check_robots(client: &Client,  connection: &Connection, url: &str) -> Result<bool, Box<dyn Error>> {
    match parse_robots(client, url).await {
        Ok(robots_parsed) => {
            for line in robots_parsed {
                
                if let (Some(k), Some(v)) = line {
                    match k.trim() {
                        "User-agent" => {
                            // TODO: implement user-agent checking
                        },
                        "Disallow" => {
                            let parsed_url = Url::parse(url)?;
                            let url_subdir = parsed_url.path();
        
                            if url_subdir.trim_start_matches('/') == v.trim_start_matches('/') {
                                return Ok(false);
                            }
                        },
                        _ => {}
                    }
                } else {
                    return Ok(true); // failsafe for broken robots.txt
                }
            }
            Ok(true)
        },
        Err(e) => Err(e)
    }
}

pub async fn check_blocked_list_db(connection: &Connection, url: &str) {
    /*
    let blocked_list = connection.query_row(
        "SELECT Id, Url, Created FROM BlockedUrls WHERE Url = ?", 
        &[url],
        |row| {
            Ok()
        }
    ); */
}

pub async fn add_blocked_to_db(connection: &Connection) {

}

pub async fn get_robots(client: &Client, url: &str) -> Result<String, Box<dyn Error>> {
    let parsed = Url::parse(url)?;
    let mut robots_url = parsed.clone();

    robots_url.set_path("/robots.txt");
    robots_url.set_query(None);
    robots_url.set_fragment(None);

    let result = client.get(robots_url).send().await?;
    if !result.status().is_success() {
        return Err(Box::new(io::Error::new(io::ErrorKind::Other, "No robots.txt" ))); // failsafe for no robots.txt
    }

    Ok(result.text().await?)
}

pub async fn parse_robots(client: &Client, url: &str) -> Result<Vec<(Option<String>, Option<String>)>, Box<dyn Error>> {
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

/* 
pub async fn parse_robots(client: &Client, url: &str) -> Result<bool, Box<dyn Error>> {
    let parsed = Url::parse(url)?;
    let mut robots_url = parsed.clone();

    robots_url.set_path("/robots.txt");
    robots_url.set_query(None);
    robots_url.set_fragment(None);

    let result = client.get(robots_url).send().await?;
    if !result.status().is_success() {
        return Ok(true) // failsafe for no robots.txt
    }

    let text = result.text().await?;
    
    for line in text.lines() {
        let parts: Vec<&str> = line.splitn(2, ":").collect();
        
        let (key, value) = if let [k, v] = &parts[..] {
            (Some(k.trim()), Some(v.trim()))
        } else {
            (None, None)
        };

        if let (Some(k), Some(v)) = (key, value) {
            match k.trim() {
                "User-agent" => {
                    // TODO: implement user-agent checking
                },
                "Disallow" => {
                    let parsed_url = Url::parse(url)?;
                    let url_subdir = parsed_url.path();

                    if url_subdir.trim_start_matches('/') == v.trim_start_matches('/') {
                        return Ok(false);
                    }
                },
                _ => {}
            }
        } else {
            return Ok(true); // failsafe for broken robots.txt
        }
    }
    Ok(true)
}
*/