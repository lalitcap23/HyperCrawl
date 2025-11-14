use anyhow::Result;
use futures::future::join_all;
use log::{info, warn};
use reqwest::Client;
use scraper::{Html, Selector};
use std::{
    collections::{HashSet, VecDeque},
    sync::Arc,
    time::Duration,
};
use tokio::sync::{Mutex, Semaphore};
use url::Url;

#[tokio::main]
async fn main() -> Result<()> {
    env_logger::init();

    let start = Url::parse("https://example.com")?;
    let client = Client::builder()
        .user_agent("RustCrawler/0.1 (github.com/yourname)")
        .timeout(Duration::from_secs(15))
        .build()?;

    let visited = Arc::new(Mutex::new(HashSet::new()));
    let queue = Arc::new(Mutex::new(VecDeque::from([start.clone()])));
    let max_concurrency = 10;
    let sem = Arc::new(Semaphore::new(max_concurrency));

    let mut workers = Vec::new();
    for _ in 0..max_concurrency {
        let client = client.clone();
        let visited = Arc::clone(&visited);
        let queue = Arc::clone(&queue);
        let sem = Arc::clone(&sem);
        let start_domain = start.domain().map(|s| s.to_string());

        let worker = tokio::spawn(async move {
            loop {
                let next_url_opt = {
                    let mut q = queue.lock().await;
                    q.pop_front()
                };

                let url = match next_url_opt {
                    Some(u) => u,
                    None => break,
                };

                {
                    let mut v = visited.lock().await;
                    if v.contains(&url) {
                        continue;
                    }
                    v.insert(url.clone());
                }

                if let Some(domain) = &start_domain {
                    if url.domain().map(|d| d != domain).unwrap_or(true) {
                        info!("Skipping out-of-domain: {}", url);
                        continue;
                    }
                }

                // acquire permit (rate / concurrency control)
                let permit = sem.acquire().await.unwrap();
                info!("Fetching: {}", url);

                match fetch_and_extract(&client, &url).await {
                    Ok(links) => {
                        // push discovered links
                        let mut q = queue.lock().await;
                        for link in links {
                            // normalize & push if not visited
                            if let Ok(abs) = url.join(&link) {
                                let abs = abs.into();
                                // push new url
                                if !visited.lock().await.contains(&abs) {
                                    q.push_back(abs);
                                }
                            }
                        }
                    }
                    Err(e) => warn!("Error fetching {}: {:?}", url, e),
                }

                // drop permit to free slot
                drop(permit);

                // small polite sleep (optional)
                tokio::time::sleep(Duration::from_millis(100)).await;
            }
        });

        workers.push(worker);
    }

    // wait for all workers to finish
    join_all(workers).await;

    info!("Crawling finished.");
    Ok(())
}

async fn fetch_and_extract(client: &Client, url: &Url) -> Result<Vec<String>> {
    let res = client.get(url.clone()).send().await?;
    let body = res.text().await?;

    // parse html
    let document = Html::parse_document(&body);

    // example: extract page title
    let title_sel = Selector::parse("title").unwrap();
    if let Some(t) = document.select(&title_sel).next() {
        let title = t.text().collect::<String>().trim().to_string();
        info!("Title: {} -> {}", url, title);
    }

    // extract hrefs
    let a_sel = Selector::parse("a").unwrap();
    let mut links = Vec::new();
    for el in document.select(&a_sel) {
        if let Some(href) = el.value().attr("href") {
            links.push(href.to_string());
        }
    }

    Ok(links)
}
