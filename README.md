# Rust Web Crawler

Homework to write a simple web crawler.

This homework was written for educational purposes only.

## Execution

Cargo:
```bash
cargo run -- --url <start_url>

cargo run -- --url <start_url> --depth <depth>
```

Use the `--clean` flag to re-initialize the database and delete the `pages` directory.

### Arguments

| Argument         | Description |
|------------------|-------------|
| `--clean`           | Delete the `pages` directory and `web_crawler.db` database before starting the crawl. |
| `--depth <depth>`   | The maximum depth to crawl. Default is 1.                                  |
| `--url <start_url>` | The URL to start the crawl from.                                           |
| `--ignore-robots`   | Ignore `robots.txt` files when crawling.                                   |
| `--help`            | Display the help message.                                                  |
| `--version`         | Display the version information.                                           |

## Features
 - A SQLite database (`web_crawler.db`) to store pages, links, disallowed URL patterns, and domain. See [`create.sql`](./scripts/create.sql) for the schema.
 - Scraped pages are saved to the `pages` directory. Their filenames are a [Blake3 hash](https://docs.rs/blake3/latest/blake3/) of their contents.
 - Robots.txt rules should be followed.

## Potential Improvements
 - Support multiple threads for faster crawling.
 - Support advanced pattern matching when checking URLs in `robots.txt` files.
 - Filter out non-HTML pages.
 - Optimise the check for visited URLs. The program currently queries the database for each URL.
