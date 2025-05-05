use reqwest::Client;
use std::error::Error;
use url::Url;

pub async fn check_page(client: &Client, url: &str) -> Result<bool, Box<dyn Error>> {
    // TODO: check non HTML pages
    parse_robots(client, url).await
}

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