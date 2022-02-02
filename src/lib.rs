use std::{error::Error, fmt::Display};

use gloo::utils::document;
use gloo_events::EventListener;
use js_sys::Math::random;
use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::spawn_local;
use web_sys::console::log_1;

struct Meme<'a> {
    url: Url,
    subreddit: String,
    title: String,
    css: &'a str,
}

enum Url {
    Video(String),
    Image(String),
}

enum Type<'a> {
    JsArray(&'a serde_json::Value),
    Vector(&'a Vec<String>),
}

#[derive(Debug)]
struct ArrayNotFound {}

impl Error for ArrayNotFound {}

impl Display for ArrayNotFound {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::result::Result<(), std::fmt::Error> {
        write!(f, "Unable to serialize the array")
    }
}

// add link to meme
// random int don't work
#[wasm_bindgen(start)]
pub fn main_js() -> Result<(), JsValue> {
    let button_change_theme = document().get_element_by_id("switch").unwrap();
    let subreddits = vec!["memes", "shitposts", "dankmemes", "whenthe"];
    let links = get_subreddits_links(subreddits);

    match get_random(&Type::Vector(&links)) {
        Ok(i) => {
            let link = links[i as usize].clone();

            spawn_local(async move {
                let button = document().get_element_by_id("update").unwrap();
                operation(link.clone()).await;

                EventListener::new(&button, "click", move |_| {
                    let link = link.clone();
                    spawn_local(async move {
                        operation(link).await;
                    });
                })
                .forget();

                EventListener::new(&button_change_theme, "click", move |_| {
                    change_theme();
                })
                .forget();
            });
        }
        Err(err) => log_1(
            &format!("There was an Error: \nDetails {:#?}", err)
                .as_str()
                .into(),
        ),
    };

    Ok(())
}

fn get_subreddits_links(subreddits: Vec<&str>) -> Vec<String> {
    subreddits
        .into_iter()
        .map(|x| format!("https://www.reddit.com/r/{}.json", x))
        .collect()
}

async fn operation(link: String) {
    match get_json(link.as_str()).await {
        Ok(json) => match get_meme(json, "rounded-xl object-contain h-96 w-96") {
            Ok(meme) => {
                match display(meme) {
                    Ok(_) => log_1(&"Got the meme".into()),
                    Err(err) => log_1(
                        &format!("There was an Error: \nDetails {:#?}", err)
                            .as_str()
                            .into(),
                    ),
                };
            }
            Err(err) => {
                log_1(
                    &format!("There was an Error: \nDetails {:#?}", err)
                        .as_str()
                        .into(),
                );
            }
        },
        Err(err) => {
            log_1(
                &format!("There was an Error: \nDetails {:#?}", err)
                    .as_str()
                    .into(),
            );
        }
    };
}

fn display(meme: Meme) -> Result<(), JsValue> {
    let document = document();

    match meme.url {
        Url::Video(link) => {
            let element = document
                .get_element_by_id("video")
                .ok_or_else(|| JsValue::from_str("<video> tag with the id \"video\" not found."))?;
            element.set_inner_html(format!("<source src=\"{}\" type=\"video/mp4\"", link).as_str());
            element.set_class_name(meme.css);

            // Cleaning the <img> tag
            let element = document
                .get_element_by_id("image")
                .ok_or_else(|| JsValue::from_str("<img> tag with the id \"image\" not found."))?;
            element.set_attribute("src", "")?;
            element.set_class_name("hidden");
        }
        Url::Image(link) => {
            let element = document
                .get_element_by_id("image")
                .ok_or_else(|| JsValue::from_str("<img> tag with the id \"image\" not found."))?;
            element.set_attribute("src", link.as_str())?;
            element.set_attribute("alt", "Image containing the meme")?;
            element.set_class_name(meme.css);

            //Cleaning the <video> tag
            let element = document
                .get_element_by_id("video")
                .ok_or_else(|| JsValue::from_str("<video> tag with the id \"video\" not found."))?;
            element.set_inner_html("");
            element.set_class_name("hidden")
        }
    };

    let title_element = document
        .get_element_by_id("title")
        .ok_or_else(|| JsValue::from_str("<p> tag with the id \"title\" not found."))?;

    title_element.set_text_content(Some(meme.title.as_str()));

    let subreddit_element = document
        .get_element_by_id("subreddit")
        .ok_or_else(|| JsValue::from_str("<p> tag with the id \"subreddit\" not found."))?;

    subreddit_element.set_text_content(Some(meme.subreddit.as_str()));

    Ok(())
}

fn get_meme(json: serde_json::Value, css: &str) -> Result<Meme, Box<dyn Error>> {
    let arr = &json["data"]["children"];
    let random_int = get_random(&Type::JsArray(arr))? as usize;
    let head = &json["data"]["children"][random_int]["data"];

    // let is_nsfw = head["over_18"].to_string().parse::<bool>()?;

    let is_video = head["is_video"].to_string().parse::<bool>()?;

    let url = match is_video {
        true => Url::Video(
            head["media"]["reddit_video"]["scrubber_media_url"]
                .to_string()
                .replace("\"", ""),
        ),
        false => Url::Image(head["url"].to_string().replace("\"", "")),
    };

    let subreddit = head["subreddit"].to_string().replace("\"", "");

    let title = head["title"].to_string().replace("\"", "");

    Ok(Meme {
        url,
        subreddit,
        title,
        css,
    })
}

fn get_random(value: &Type) -> Result<u64, ArrayNotFound> {
    let length = match value {
        Type::Vector(val) => Some(val.len()),
        Type::JsArray(val) => match val {
            serde_json::Value::Array(arr) => Some(arr.len()),
            _ => None,
        },
    };

    match length {
        Some(len) => {
            let length = len as f64;
            let random_int = (random() * length).floor() as u64;
            Ok(random_int)
        }
        None => Err(ArrayNotFound {}),
    }
}

async fn get_json<'a>(url: &'a str) -> Result<serde_json::Value, Box<dyn Error + 'a>> {
    let response = reqwest_wasm::get(url).await?;
    let json = response.json::<serde_json::Value>().await?;
    Ok(json)
}

fn change_theme() {
    let element = document().get_element_by_id("mode").unwrap();
    let mode = element.class_name();
    let mut mode = mode.as_str();
    let button_text;

    match mode {
        "dark" => {
            button_text = "Light 🌞".to_string();
            mode = ""
        }
        _ => {
            button_text = "Dark 🌝".to_string();
            mode = "dark"
        }
    }

    element.set_class_name(mode);
    document()
        .get_element_by_id("switch")
        .unwrap()
        .set_inner_html(button_text.as_str());
}
