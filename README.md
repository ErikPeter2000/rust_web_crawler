# Rust Web Crawler

Homework to write a simple web crawler.

This homework was written for educational purposes only.

## Execution

Using Cargo:
```bash
cargo run -- --url <start_url>

cargo run -- --url <start_url> --depth <depth>
```

### Arguments

| Argument         | Description |
|------------------| ------------|
| `--url <start_url>` | The URL to start the crawl from.                                           |
| `--depth <depth>`   | The maximum depth to crawl. Default is 1.                                  |
| `--clean`           | Delete the `pages` directory and `web_crawler.db` database before starting the crawl. |
| `--help`            | Display the help message.                                                  |
| `--version`         | Display the version information.                                           |

## Features
 - `web_crawler.db` SQLite database stores the URLs and any immediate links for that page. See the [`create.sql`](./scripts/create.sql) file for the schema.
 - Scraped pages are saved to the `pages` directory. Their filenames are a hash of their content.

## Improvements
 - Add a `robots.txt` parser to respect the rules.
 - Support multiple threads for faster crawling.
 - Filter out non-HTML pages.
 - Optimise the check for visited URLs. The program currently queries the database for each URL.