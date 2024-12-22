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
    text: Option<String>,
}

pub struct SBStory {
    title: String,
    rss_url: String,
    description: String,
    pub_date: String,
    threadmarks: Vec<Threadmark>,
    client: Client,
}

pub trait SBStoryUtils: Sized {
    fn new(url: &str) -> Result<Self, Box<dyn std::error::Error>>;
    fn update_threadmarks(&mut self) -> Result<(), Box<dyn std::error::Error>>;
    fn get_threadmarks(&self) -> Vec<Threadmark>;
    fn load_threadmark_text(&mut self, idx: usize) -> Result<(), Box<dyn std::error::Error>>;
    fn get_title(&self) -> String;
    fn get_description(&self) -> String;
    fn get_pub_date(&self) -> String;
}

pub trait ThreadmarkUtils {
    fn get_title(&self) -> String;
    fn get_url(&self) -> String;
    fn get_pub_date(&self) -> String;
    fn get_text(&self) -> Option<String>;
}

fn get_rss(rss_url: &str, client: &Client) -> Result<Channel, Box<dyn std::error::Error>> {
    let response = client
        .get(rss_url)
        .header("Cache-Control", "no-cache")
        .header("Pragma", "no-cache")
        .header("Priority", "u=0, i")
        .header("Accept", "text/html,application/xhtml+xml,application/xml;q=0.9,image/jxl,image/avif,image/webp,image/apng,*/*;q=0.8,application/signed-exchange;v=b3;q=0.7")
        //.header("if-modified-since", "Thu, 01 Jan 1970 00:00:00 GMT")
        .send()?;

    if response.status() != 200 {
        dbg_println!("URL: {}", rss_url);
        dbg_println!("Status: {}", response.status());
        dbg_println!("Headers: {:?}", response.headers());
        return Err("Failed to get RSS feed".into());
    }

    let channel = Channel::read_from(response.text()?.as_bytes())?;

    if channel.title() == "errors" {
        return Err(format!("Spacebattles returned error: {}", channel.description()).into());
    }

    // wait for 500ms to prevent rate limiting
    std::thread::sleep(std::time::Duration::from_millis(500));
    Ok(channel)
}

fn scrape_sb_post_text(post_url: &str, client: &Client) -> Result<String, Box<dyn std::error::Error>> {
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

    let response = client
        .get(post_url)
        .header("Cache-Control", "no-cache")
        .header("Pragma", "no-cache")
        .header("Accept", "application/rss+xml,application/xml;q=0.9,*/*;q=0.8")
        .header("Referer", "https://forums.spacebattles.com/")
        .header("Cache-Control", "no-cache")
        .header("if-modified-since", "Thu, 01 Jan 1970 00:00:00 GMT")
        .send()?
        .text()?;
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

        let client = Client::builder()
            .user_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome.0.0.0 Safari/537.36")
            .redirect(reqwest::redirect::Policy::limited(10))
            .build()?;

        let channel = get_rss(rss_url.as_str(), &client)?;
        
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
            // let text = scrape_sb_post_text(url)?; defer text scraping until needed

            threadmarks.push(Threadmark {
                title: title.into(),
                url: url.into(),
                pub_date: pub_date.into(),
                text: None,
            });
        }

        Ok(SBStory {
            rss_url,
            title: channel.title().into(),
            description: channel.description().into(),
            pub_date: channel.pub_date().unwrap().into(),
            threadmarks,
            client,
        })
    }
    fn update_threadmarks(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        let channel = get_rss(self.rss_url.as_str(), &self.client)?;
        let mut threadmarks: Vec<Threadmark> = Vec::new();
        for item in channel.items() {
            let title = item.title().unwrap();
            let url = item.link().unwrap();
            let pub_date = item.pub_date().unwrap();
            // let text = scrape_sb_post_text(url)?;
            threadmarks.push(Threadmark {
                title: title.to_string(),
                url: url.to_string(),
                pub_date: pub_date.to_string(),
                text: None
            });
        }
        self.threadmarks = threadmarks;
        Ok(())
    }
    fn load_threadmark_text(&mut self, idx: usize) -> Result<(), Box<dyn std::error::Error>> {
        let post_url = self.threadmarks[idx].url.as_str();
        let post_text = scrape_sb_post_text(post_url, &self.client)?;
        self.threadmarks[idx].text = Some(post_text);
        Ok(())
    }
    fn get_threadmarks(&self) -> Vec<Threadmark> {
        return self.threadmarks.clone();
    }
    fn get_title(&self) -> String {
        return self.title.clone();
    }
    fn get_description(&self) -> String {
        return self.description.clone();
    }
    fn get_pub_date(&self) -> String {
        return self.pub_date.clone();
    }
}

impl ThreadmarkUtils for Threadmark {
    fn get_title(&self) -> String {
        return self.title.clone();
    }
    fn get_url(&self) -> String {
        return self.url.clone();
    }
    fn get_pub_date(&self) -> String {
        return self.pub_date.clone();
    }
    fn get_text(&self) -> Option<String> {
        return self.text.clone();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn get_threadmarks_1() {
        let story = SBStory::new("https://forums.spacebattles.com/threads/omnissiah-vult-a-story-of-ashes-and-empire-wh40k.1053424/page-2#post-99969166").expect("Failed to create SBStory");
        assert_eq!(story.get_threadmarks().len() != 0, true);
    }
    #[test]
    fn get_threadmarks_2() {
        let story = SBStory::new("https://forums.spacebattles.com/threads/a-bad-name-worm-oc-the-gamer.500626/page-412#post-106550411").expect("Failed to create SBStory");
        assert_eq!(story.get_threadmarks().len() != 0, true);
    }
    #[test]
    fn get_threadmarks_3() {
        let story = SBStory::new("https://forums.spacebattles.com/threads/have-you-come-to-meet-your-match-a-young-justice-kryptonian-si.1184788/#post-106216778").expect("Failed to create SBStory");
        assert_eq!(story.get_threadmarks().len() != 0, true);
    }
}
