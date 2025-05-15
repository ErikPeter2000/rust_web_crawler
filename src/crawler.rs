use blake3::Hasher;
use hex::encode;
use itertools::Itertools;
use log::{error, info};
use regex::Regex;
use rusqlite::Connection;
use scraper::{Html, Selector};
use std::fs;
use url::Url;

use crate::unique_queue::UniqueQueue;

const DB_NAME: &str = "web_crawler.db";
const SAVE_DIR: &str = "pages";
const DISALLOWED_ROBOTS_REGEX: &str = r"(?i)Disallow:\s*(\S+*)";

/// A web crawler that follows links on webpages and stores their contents to SQLite database.
pub struct Crawler {
    pub user_agent: String,
    pub db_connection: Connection,

    url_queue: UniqueQueue<String>,
    hasher: Hasher,
    ignore_robots: bool,
}

impl Crawler {
    /// Creates a new Crawler instance.
    ///
    /// # Arguments
    /// * `start_url` - The URL to start crawling from.
    /// * `user_agent` - The name of the user agent string to.
    /// * `ignore_robots` - Whether to ignore robots.txt rules. Default is false.
    pub fn new(start_url: &str, user_agent: &str, ignore_robots: Option<bool>) -> Self {
        let db_connection = Connection::open(DB_NAME).unwrap();

        let mut url_queue = UniqueQueue::new();
        url_queue.push(start_url.to_string());

        Crawler {
            user_agent: user_agent.to_string(),
            db_connection,
            url_queue,
            hasher: Hasher::new(),
            ignore_robots: ignore_robots.unwrap_or(false),
        }
    }

    /// Fetches the domain id from the database.
    ///
    /// # Arguments
    /// * `url` - The URL of the page.
    ///
    /// # Returns
    /// The id of the domain entity.
    fn get_domain_id(&self, url: &Url) -> Result<i64, Box<dyn std::error::Error>> {
        let domain_name = url.domain().ok_or("Invalid URL")?;
        let id: i64 = self.db_connection.query_row(
            "SELECT Id FROM Domain WHERE Name = ?",
            [domain_name],
            |row| row.get(0),
        )?;
        Ok(id)
    }

    /// Checks if the URL is crawlable based on the robots.txt rules and if it has already been crawled.
    ///
    /// URLs that are already in the database are not crawlable.
    ///
    /// # Arguments
    /// * `url` - The URL to check.
    /// * `domain_id` - The id of the domain entity.
    ///
    /// # Returns
    /// A tuple containing a boolean indicating if the URL is crawlable and an optional reason why it is not.
    fn is_url_crawlable(
        &self,
        url: &Url,
        domain_id: Option<i64>,
    ) -> Result<(bool, Option<&str>), Box<dyn std::error::Error>> {
        let exists = self.db_connection.query_row(
            "SELECT COUNT(*) FROM Page WHERE Url = ?",
            [url.as_str()],
            |row| row.get::<_, i32>(0),
        )? > 0;
        if exists {
            return Ok((false, Some("Already crawled")));
        }

        if self.ignore_robots {
            return Ok((true, None));
        }

        // Check if the URL is crawlable based on robots.txt rules
        let domain_id = match domain_id {
            Some(id) => id,
            None => self.get_domain_id(url)?,
        };
        let mut stmt = self
            .db_connection
            .prepare("SELECT Pattern FROM DisallowedPattern WHERE DomainId = ?")?;
        let disallowed_patterns = stmt
            .query_map([domain_id], |row| row.get::<_, String>(0))?
            .filter_map(Result::ok)
            .collect::<Vec<_>>();

        // Check URL path against disallowed patterns
        let path = url.path();
        for pattern in disallowed_patterns {
            if path.starts_with(&pattern) || pattern == "*" {
                return Ok((false, Some("Disallowed by robots.txt")));
            }
        }
        Ok((true, None))
    }

    /// Resolves the href attribute of an anchor tag and returns a Url object.
    ///
    /// # Arguments
    /// * `href` - The href attribute value.
    /// * `base_url` - The base URL to resolve against.
    ///
    /// # Returns
    /// An Option containing the resolved URL if successful, None otherwise.
    fn parse_href(&self, href: &str, base_url: &Url) -> Option<Url> {
        let mut new_url: Url;
        if let Ok(parsed_url) = Url::parse(href) {
            new_url = parsed_url;
        } else if href.starts_with("//") {
            let scheme = base_url.scheme();
            new_url = Url::parse(&format!("{}:{}", scheme, href)).ok()?;
        } else if href.starts_with('/') {
            new_url = base_url.clone();
            new_url.set_path(href);
        } else {
            new_url = base_url.clone();
        }
        new_url.set_query(None);
        new_url.set_fragment(None);
        Some(new_url)
    }

    /// Records the url domain in the database, and returns the domain id.
    ///
    /// # Arguments
    /// * `url` - The URL of the page.
    ///
    /// # Returns
    /// The id of the created domain entity.
    fn record_domain(&self, url: &Url) -> Result<i64, Box<dyn std::error::Error>> {
        let domain_name = url.domain().ok_or("Invalid URL")?;
        self.db_connection.execute(
            "INSERT OR IGNORE INTO Domain (Name) VALUES (?)",
            [domain_name],
        )?;
        let id: i64 = self.db_connection.query_row(
            "SELECT Id FROM Domain WHERE Name = ?",
            [domain_name],
            |row| row.get(0),
        )?;
        Ok(id)
    }

    /// Parses a html page and records the links found in the database.
    ///
    /// # Arguments
    /// * `url` - The URL of the page.
    /// * `body` - The contents of the page.
    /// * `page_id` - The id of the page entity.
    /// * `domain_id` - The id of the domain entity.
    ///
    /// # Returns
    /// A Result indicating success or failure.
    fn record_page_links(
        &mut self,
        url: &Url,
        body: &str,
        page_id: i64,
        domain_id: Option<i64>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        // Fetch the id here, before iteration
        let domain_id = match domain_id {
            Some(id) => id,
            None => self.get_domain_id(url)?,
        };

        let document = Html::parse_document(body);
        let selector = Selector::parse("a")?;
        let urls: Vec<String> = document
            .select(&selector)
            .filter_map(|element| element.value().attr("href"))
            .filter_map(|href| self.parse_href(href, url))
            .filter(|url| {
                self.is_url_crawlable(url, Some(domain_id))
                    .unwrap_or((false, None))
                    .0
            })
            .map(|url| url.to_string())
            .collect();

        info!("Found {} links on page {}", urls.len(), url);

        for url in urls {
            self.url_queue.push(url.clone());
            self.db_connection.execute(
                "INSERT OR IGNORE INTO PageLink (PageId, Url) VALUES (?, ?)",
                [page_id.to_string(), url.clone()],
            )?;
        }
        Ok(())
    }

    /// Records the page contents in the database and saves it to a file.
    ///
    /// # Arguments
    /// * `url` - The URL of the page.
    /// * `body` - The contents of the page.
    /// # Returns
    /// The id of the created page entity.
    fn record_page_contents(
        &mut self,
        url: &Url,
        body: &str,
    ) -> Result<i64, Box<dyn std::error::Error>> {
        self.hasher.reset();
        self.hasher.update(body.as_bytes());
        let hash = encode(self.hasher.finalize().as_bytes());
        let filename = format!("{}.html", hash);
        let filepath = format!("{}/{}", SAVE_DIR, filename);
        fs::write(filepath, body)?;
        self.db_connection.execute(
            "INSERT INTO Page (Url, Hash) VALUES (?, ?)",
            &[url.as_str(), &hash],
        )?;
        let page_id = self.db_connection.last_insert_rowid();
        Ok(page_id)
    }

    /// Fetches the robots.txt file for an existing domain in the database and records the disallowed patterns.
    ///
    /// Will return if the robots.txt file is not found.
    ///
    /// # Arguments
    /// * `url` - The URL of the page.
    /// * `domain_id` - The id of the domain entity.
    ///
    /// # Returns
    /// A Result indicating success or failure.
    async fn record_robots_txt(
        &self,
        url: &Url,
        domain_id: Option<i64>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let domain_id = match domain_id {
            Some(id) => id,
            None => self.get_domain_id(url)?,
        };

        // Fetch the robots.txt file
        let domain_name = url.domain().ok_or("Invalid URL")?;
        let robots_url = format!("{}://{}/robots.txt", url.scheme(), domain_name);
        let response = reqwest::get(&robots_url).await?;

        // Return if the robots.txt file is not found
        let status = response.status();
        if !status.is_success() {
            info!("No robots.txt found for {}", domain_name);
            return Ok(());
        }

        // Parse the robots.txt file
        let robots_txt = response.text().await?;

        // Split the file into "user-agent" sections
        let user_agent_regex = Regex::new(r"(?i)User-agent:\s*(\S+*)")?;
        let disallowed_regex = Regex::new(DISALLOWED_ROBOTS_REGEX)?;
        let mut user_agent_matches = user_agent_regex
            .find_iter(&robots_txt)
            .map(|m| m.start())
            .collect::<Vec<_>>();
        user_agent_matches.push(robots_txt.len());

        // Iterate over the user-agent sections and record disallowed patterns if the user-agent matches
        for (first_match, last_match) in user_agent_matches.iter().tuple_windows() {
            let section = &robots_txt[*first_match..*last_match];
            let user_agent = user_agent_regex
                .captures(section)
                .and_then(|cap| cap.get(1))
                .map(|m| m.as_str())
                .unwrap_or("");

            if user_agent != "*" && user_agent != self.user_agent {
                continue;
            }

            // Record disallowed patterns
            for disallowed in disallowed_regex.captures_iter(section) {
                if let Some(disallowed_pattern) = disallowed.get(1) {
                    self.db_connection.execute(
                        "INSERT OR IGNORE INTO DisallowedPattern (DomainId, Pattern) VALUES (?, ?)",
                        &[
                            &domain_id.to_string().as_str(),
                            &disallowed_pattern.as_str(),
                        ],
                    )?;
                }
            }
        }
        Ok(())
    }

    /// Fetches the page contents and records them in the database.
    ///
    /// Records any links found on the page.
    ///
    /// # Arguments
    /// * `url` - The URL of the page.
    /// * `domain_id` - The id of the domain entity.
    ///
    /// # Returns
    /// A Result indicating success or failure.
    async fn process_page(
        &mut self,
        url: &Url,
        domain_id: Option<i64>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let response = reqwest::get(url.as_str()).await?;
        let status = response.status();
        if !status.is_success() {
            error!("Failed to fetch page ({}): {}", status.as_str(), url);
            return Ok(());
        }
        let body = response.text().await?;

        let page_id = self.record_page_contents(url, &body)?;
        self.record_page_links(url, &body, page_id, domain_id)?;

        Ok(())
    }

    /// Perform a single crawl iteration.
    ///
    /// An iteration consists of processing the next URL in a queue.
    ///
    /// # Returns
    /// `true` if there are more URLs to crawl, `false` otherwise.
    pub async fn crawl(&mut self) -> Result<bool, Box<dyn std::error::Error>> {
        let next_url = self.url_queue.pop();
        match next_url {
            Some(url) => {
                info!("Crawling URL: {}", url);
                let url = Url::parse(&url)?;
                let domain_id = self.record_domain(&url)?;
                self.record_robots_txt(&url, Some(domain_id)).await?;

                if let (false, reason) = self.is_url_crawlable(&url, Some(domain_id))? {
                    info!("URL {} is not crawlable: {}", url, reason.unwrap_or(""));
                } else {
                    self.process_page(&url, Some(domain_id)).await?;
                }
                if self.url_queue.is_empty() {
                    return Ok(false);
                }
            }
            None => {
                return Ok(false);
            }
        }
        Ok(true)
    }
}
