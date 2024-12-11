se std::collections::HashSet;
use std::io::Write;
use reqwest::blocking::Client;
use scraper::{Html, Selector};
use rss::Channel;
use regex::Regex;

macro_rules! dbg_println {
    ($($arg:tt)*) => {
        #[cfg(debug_assertions)]
        {
            println!($($arg)*);
        }
    };
}

#[derive(Clone)]
pub struct Threadmark {
    title: String,
    url: String,
    pub_date: String,
    text: String, // TODO: will probably need to implement something to get both text and images from threadmarks
}

pub struct SBStory {
    title: String,
    rss_url: String,
    description: String,
    pub_date: String,
    threadmarks: Vec<Threadmark>,
}

pub trait SBStoryUtils: Sized {
    fn new(url: &str) -> Result<Self, Box<dyn std::error::Error>>;
    fn update_threadmarks(&mut self) -> Result<(), Box<dyn std::error::Error>>;
    fn get_threadmarks(&self) -> Vec<Threadmark>;
}

fn get_rss(rss_url: &str) -> Result<Channel, Box<dyn std::error::Error>> {
    let client = Client::builder().build()?;
    let response = client.get(rss_url).send()?.text()?;
    if response.is_empty() {
        return Err(format!("Spacebattles returned empty response to URL: {}", rss_url).into());
    }

    let channel = Channel::read_from(response.as_bytes())?;

    if channel.title() == "errors" {
        return Err(format!("Spacebattles returned error: {}", channel.description()).into());
    }

    Ok(channel)
}

fn scrape_sb_post_text(post_url: &str) -> Result<String, Box<dyn std::error::Error>> {
    let post_regex = Regex::new(r"post-[0-9]+$").unwrap();
    let post_id = format!(
        "#js-{} \
        > div \
        > div.message-cell.message-cell--main \
        > div \
        > div \
        > div \
        > article \
        > div:nth-child(1) \
        > div", post_regex.find(post_url).unwrap().as_str());

    let client = Client::builder().build()?;
    let response = client.get(post_url).send()?.text()?;
    let document = Html::parse_document(&response);

    let bbwrapper = document.select(&Selector::parse(&post_id).unwrap()).next().unwrap();

    dbg_println!("Post Text: {}", bbwrapper.inner_html());
    Ok(bbwrapper.inner_html())
} 

impl SBStoryUtils for SBStory {
    fn new(url: &str) -> Result<Self, Box<dyn std::error::Error>> {
        let story = Regex::new(r"https:\/\/forums.spacebattles.com\/threads\/.*\/").unwrap();
        let clean_url = story.find(url).unwrap().as_str();
        if clean_url.is_empty() {
            return Err("Invalid Spacebattles URL".into());
        }
        let rss_url = format!("{}threadmarks.rss", clean_url);
        let channel = get_rss(rss_url.as_str())?;
        
        dbg_println!("RSS URL: {}", rss_url);
        dbg_println!("Title: {}", channel.title());
        dbg_println!("Description: {}", channel.description());
        dbg_println!("Pub Date: {}", channel.pub_date().unwrap());
        dbg_println!("Threadmark Count: {}", channel.items().len());

        let mut threadmarks: Vec<Threadmark> = Vec::new();
        for item in channel.items() {
            let title = item.title().unwrap();
            let url = item.link().unwrap();
            let pub_date = item.pub_date().unwrap();
            let text = scrape_sb_post_text(url)?;

            threadmarks.push(Threadmark {
                title: title.into(),
                url: url.into(),
                pub_date: pub_date.into(),
                text
            });
        }

        Ok(SBStory {
            rss_url,
            title: channel.title().into(),
            description: channel.description().into(),
            pub_date: channel.pub_date().unwrap().into(),
            threadmarks,
        })
    }
    fn update_threadmarks(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        let channel = get_rss(self.rss_url.as_str())?;
        let mut threadmarks: Vec<Threadmark> = Vec::new();
        for item in channel.items() {
            let title = item.title().unwrap();
            let url = item.link().unwrap();
            let pub_date = item.pub_date().unwrap();
            let text = scrape_sb_post_text(url)?;
            threadmarks.push(Threadmark {
                title: title.to_string(),
                url: url.to_string(),
                pub_date: pub_date.to_string(),
                text: text.to_string(),
            });
        }
        self.threadmarks = threadmarks;
        Ok(())
    }
    fn get_threadmarks(&self) -> Vec<Threadmark> {
        return self.threadmarks.clone();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sb_post_scraping(){
        let post_url = "https://forums.spacebattles.com/threads/gloryhound-worm-jujutsu-kaisen-si-fanfic.1162563/#post-101319000";
        let post_text = scrape_sb_post_text(post_url).unwrap();
        assert_eq!(post_text.is_empty(), false);
    }
    #[test]
    fn test_sb_post_scraping_2(){
        let post_url = "https://forums.spacebattles.com/threads/omnissiah-vult-a-story-of-ashes-and-empire-wh40k.1053424/page-2#post-99969166";
        let post_text = scrape_sb_post_text(post_url).unwrap();
        assert_eq!(post_text.is_empty(), false);
    }
    #[test]
    fn test_sb_post_scraping_3(){
        let post_url = "https://forums.spacebattles.com/threads/a-bad-name-worm-oc-the-gamer.500626/page-412#post-106550411";
        let post_text = scrape_sb_post_text(post_url).unwrap();
        assert_eq!(post_text.is_empty(), false);
    }
    #[test]
    fn test_update_and_get_threadmarks_1() {
        let mut story = SBStory::new("https://forums.spacebattles.com/threads/omnissiah-vult-a-story-of-ashes-and-empire-wh40k.1053424/page-2#post-99969166").expect("Failed to create SBStory");
        story.update_threadmarks().expect("Failed to update threadmarks");
        assert_eq!(story.get_threadmarks().len() != 0, true);
    }
    #[test]
    fn test_update_and_get_threadmarks_1_precleaned() {
        let mut story = SBStory::new("https://forums.spacebattles.com/threads/omnissiah-vult-a-story-of-ashes-and-empire-wh40k.1053424/").expect("Failed to create SBStory");
        story.update_threadmarks().expect("Failed to update threadmarks");
        assert_eq!(story.get_threadmarks().len() != 0, true);
    }
    #[test]
    fn test_update_and_get_threadmarks_2() {
        let mut story = SBStory::new("https://forums.spacebattles.com/threads/a-bad-name-worm-oc-the-gamer.500626/page-412#post-106550411").expect("Failed to create SBStory");
        story.update_threadmarks().expect("Failed to update threadmarks");
        assert_eq!(story.get_threadmarks().len() != 0, true);
    }
    #[test]
    fn test_update_and_get_threadmarks_2_precleaned() {
        let mut story = SBStory::new("https://forums.spacebattles.com/threads/a-bad-name-worm-oc-the-gamer.500626/").expect("Failed to create SBStory");
        story.update_threadmarks().expect("Failed to update threadmarks");
        assert_eq!(story.get_threadmarks().len() != 0, true);
    }
    #[test]
    fn test_update_and_get_threadmarks_3() {
        let mut story = SBStory::new("https://forums.spacebattles.com/threads/have-you-come-to-meet-your-match-a-young-justice-kryptonian-si.1184788/#post-106216778").expect("Failed to create SBStory");
        story.update_threadmarks().expect("Failed to update threadmarks");
        assert_eq!(story.get_threadmarks().len() != 0, true);
    }
    #[test]
    fn test_update_and_get_threadmarks_3_precleaned() {
        let mut story = SBStory::new("https://forums.spacebattles.com/threads/have-you-come-to-meet-your-match-a-young-justice-kryptonian-si.1184788/").expect("Failed to create SBStory");
        story.update_threadmarks().expect("Failed to update threadmarks");
        assert_eq!(story.get_threadmarks().len() != 0, true);
    }
}
