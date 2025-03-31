use reqwest;
use std::{collections::HashSet, fs, path::Path};
use tokio;
use futures::{future::BoxFuture, FutureExt};
use html2md::parse_html;
use slugify::slugify;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let domain = "https://www.heygoody.com";
    let robots_url = format!("{}/robots.txt", domain.trim_end_matches('/'));

    //Load robots.txt
    let response = reqwest::get(&robots_url).await?;
    let content = response.text().await?;

    //Extract Sitemap URLs
    let sitemap_urls: Vec<String> = content
        .lines()
        .filter_map(|line| {
            if line.to_lowercase().starts_with("sitemap:") {
                Some(line["Sitemap:".len()..].trim().to_string())
            } else {
                None
            }
        })
        .collect();

    println!("\nSitemap URLs found:");
    for url in &sitemap_urls {
        println!("- {}", url);
    }

    //Crawl all sitemap <loc> links recursively
    let mut urls = HashSet::new();
    for sitemap in &sitemap_urls {
        println!("\nFetching sitemap: {}", sitemap);
        load_sitemap_recursive(sitemap, &mut urls).await?;
    }

    println!("\nTotal discovered URLs: {}", urls.len());

    //Download HTML > Convert to Markdown > Save
    fs::create_dir_all("output")?;
    for url in &urls {
        if let Err(e) = download_and_save_markdown(url).await {
            eprintln!("Error: {} — {}", url, e);
        }
    }

    Ok(())
}

fn load_sitemap_recursive<'a>(
    url: &'a str,
    found_urls: &'a mut HashSet<String>,
) -> BoxFuture<'a, Result<(), Box<dyn std::error::Error>>> {
    async move {
        let xml = reqwest::get(url).await?.text().await?;

        for loc in xml.split("<loc>").skip(1) {
            if let Some(end) = loc.find("</loc>") {
                let link = loc[..end].trim();
                if link.ends_with(".xml") {
                    load_sitemap_recursive(link, found_urls).await?;
                } else {
                    found_urls.insert(link.to_string());
                }
            }
        }

        Ok(())
    }
    .boxed()
}

async fn download_and_save_markdown(url: &str) -> Result<(), Box<dyn std::error::Error>> {
    let res = reqwest::get(url).await?;
    let html = res.text().await?;

    let markdown = parse_html(&html);

    // Generate file name from URL slug
    let slug = slugify(url, "", "-", Some(100));
    let filename = format!("output/{}.md", slug);

    fs::write(&filename, markdown)?;
    println!("Saved: {}", filename);

    Ok(())
}
