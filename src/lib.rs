use reqwest::blocking::Client;
use scraper::{Html, Selector};
use regex::Regex;

macro_rules! dbg_println {
    ($($arg:tt)*) => {
        #[cfg(debug_assertions)]
        {
            println!($($arg)*);
        }
    };
}

pub struct SBStory {
    url: String,
    threadmarks: Option<Vec<String>>,
    text: Option<Vec<String>>,
}

pub trait SBStoryUtils {
    fn new(url: &str) -> Result<Self, Box<dyn std::error::Error>>;
    fn update_threadmarks(&mut self) -> Result<(), Box<dyn std::error::Error>>;
    fn get_threadmarks(&self) -> Option<&Vec<String>>;
    fn update_text(&mut self) -> Result<(), Box<dyn std::error::Error>>;
    fn get_text(&self) -> Option<&Vec<String>>;
}

impl SBStoryUtils for SBStory {
    fn new(url: &str) -> Result<Self, Box<dyn std::error::Error>> {
        let story = Regex::new(r"https:\/\/forums.spacebattles.com\/threads\/.*\/").unwrap();
        let clean_url = story.find(url).unwrap().as_str();
        if clean_url.is_empty() {
            return Err("Invalid spacebattles URL provided".into());
        }
        let mut s = SBStory {
            url: clean_url.to_string(),
            threadmarks: None,
            text: None,
        };
        s.update_threadmarks();
        Ok(s)
    }
    fn update_threadmarks(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        self.threadmarks = Some(self.scrape_sb_threadmarks()?);
        Ok(())
    }
    fn get_threadmarks(&self) -> Option<&Vec<String>> {
        self.threadmarks.as_ref()
    }
    fn update_text(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        todo!()
    }
    fn get_text(&self) -> Option<&Vec<String>> {
        self.text.as_ref()
    }
}

impl SBStory { 
    fn scrape_sb_threadmarks(&mut self) -> Result<Vec<String>, Box<dyn std::error::Error>> {
        let client = Client::builder().build()?;

        let response = client.get(&self.url).send()?.text()?;
        let document = Html::parse_document(&response);

        let selector = Selector::parse(".structItemContainer .structItem.structItem--threadmark.js-inlineModContainer")?;

        let mut count = 0;
        let mut threadmarks: Vec<String> = Vec::new();
        for (index, element) in document.select(&selector).enumerate() {
            count += 1;
            // collect all text contained within elements within div
            dbg_println!("Threadmark {}:\n{}", index + 1, element.text().map(|s| s.trim()).collect::<Vec<_>>().join(" ").trim());
            let threadmark_url = element.select(&Selector::parse(".structItem-cell.structItem-cell--main .structItem-title.threadmark_depth0 .listInline.listInline--bullet li a").unwrap()).next().unwrap().value().attr("href").unwrap();
            threadmarks.push(threadmark_url.to_string());
            dbg_println!("Threadmark URL: {}", threadmark_url);
            dbg_println!("--------------------");
        }

        if count == 0 {
            return Err("No match found".into());
        }

        Ok(threadmarks)
    } 
    fn scrape_sb_text(&mut self) -> Result<Vec<String>, Box<dyn std::error::Error>> {
        todo!() 
    } 
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_update_and_get_threadmarks_1() {
        let mut story = SBStory::new("https://forums.spacebattles.com/threads/omnissiah-vult-a-story-of-ashes-and-empire-wh40k.1053424/page-2#post-99969166");
        assert_eq!(story.update_threadmarks());
        assert_eq!(story.get_threadmarks() != None, true);
    }
    #[test]
    fn test_update_and_get_threadmarks_1_precleaned() {
        let mut story = SBStory::new("https://forums.spacebattles.com/threads/omnissiah-vult-a-story-of-ashes-and-empire-wh40k.1053424/");
        assert_eq!(story.update_threadmarks());
        assert_eq!(story.get_threadmarks() != None, true);
    }
    #[test]
    fn test_update_and_get_threadmarks_2() {
        let mut story = SBStory::new("https://forums.spacebattles.com/threads/a-bad-name-worm-oc-the-gamer.500626/page-412#post-106550411");
        assert_eq!(story.update_threadmarks());
        assert_eq!(story.get_threadmarks() != None, true);
    }
    #[test]
    fn test_update_and_get_threadmarks_2_precleaned() {
        let mut story = SBStory::new("https://forums.spacebattles.com/threads/a-bad-name-worm-oc-the-gamer.500626/");
        assert_eq!(story.update_threadmarks());
        assert_eq!(story.get_threadmarks() != None, true);
    }
    #[test]
    fn test_update_and_get_threadmarks_3() {
        let mut story = SBStory::new("https://forums.spacebattles.com/threads/have-you-come-to-meet-your-match-a-young-justice-kryptonian-si.1184788/#post-106216778");
        assert_eq!(story.update_threadmarks());
        assert_eq!(story.get_threadmarks() != None, true);
    }
    #[test]
    fn test_update_and_get_threadmarks_3_precleaned() {
        let mut story = SBStory::new("https://forums.spacebattles.com/threads/have-you-come-to-meet-your-match-a-young-justice-kryptonian-si.1184788/");
        assert_eq!(story.update_threadmarks());
        assert_eq!(story.get_threadmarks() != None, true);
    }
}
