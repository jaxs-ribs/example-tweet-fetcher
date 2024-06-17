use kinode_process_lib::{call_init, println, Address, Request};

// use chrono::{DateTime, TimeZone, Utc};
use std::time::{SystemTime, UNIX_EPOCH};

use storage_interface::Request as StorageRequest;
use storage_interface::Response as StorageResponse;
use storage_interface::TweetData;

use llm_interface::openai::LLMRequest;
use llm_interface::openai::LLMResponse;
use llm_interface::openai::MessageBuilder;
use llm_interface::openai::ChatRequestBuilder;

pub const LLM_ADDRESS: (&str, &str, &str, &str) =
    ("our", "openai", "command_center", "appattacc.os");

wit_bindgen::generate!({
    path: "target/wit",
    world: "tweetfetcher-template-dot-os-v0",
    generate_unused_types: true,
    additional_derives: [serde::Deserialize, serde::Serialize],
});

fn get_groq_answer(text: &str) -> anyhow::Result<String> {
    let request = ChatRequestBuilder::default()
        .model("llama3-70b-8192".to_string())
        .messages(vec![MessageBuilder::default()
            .role("user".to_string())
            .content(text.to_string())
            .build()?])
        .build()?;
    let request = serde_json::to_vec(&LLMRequest::GroqChat(request))?;
    let response = Request::to(LLM_ADDRESS)
        .body(request)
        .send_and_await_response(30)??;
    let LLMResponse::Chat(chat) = serde_json::from_slice(response.body())? else {
        println!("chatbot: failed to parse LLM response");
        return Err(anyhow::anyhow!("Failed to parse LLM response"));
    };
    Ok(chat.choices[0].message.content.clone())
}

call_init!(init);
fn init(_our: Address) {
    // Get the unix time between now and 24 hours ago. 
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs() as i64;
    let yesterday = now - 86400 as i64; // Subtract 86400 seconds (1 day) from the current time

    // Send the request to storage to get the tweets from the last 24 hours.
    let request = serde_json::to_vec(&StorageRequest::GetTweets {
        start_time: yesterday,
        end_time: now,
    })
    .unwrap();
    let storage_address: (&str, &str, &str, &str) =
        ("our", "storage", "command_center", "appattacc.os");
    let Ok(Ok(response)) = Request::to(storage_address)
        .body(request)
        .send_and_await_response(30)
    else {
        panic!("What the fuck happened");
    };

    // Parse the response from storage.
    let body = response.body();
    let response: StorageResponse = serde_json::from_slice(&body).unwrap();
    let StorageResponse::GetTweets { tweets } = response;

    // Extract the top 25 tweets by views
    let mut tweets_vec: Vec<(&String, &TweetData)> = tweets.iter().collect();
    tweets_vec.sort_by(|a, b| b.1.views.unwrap_or(0).cmp(&a.1.views.unwrap_or(0)));
    let top_25_tweets = tweets_vec.into_iter().take(25).collect::<Vec<_>>();
    // Print the top 25 tweets with views and id
    // for (tweet_id, tweet_data) in top_25_tweets {
    //     println!("Tweet ID: {}, Content: {}, Views: {}", tweet_id, tweet_data.content, tweet_data.views.unwrap_or(0));
    // }

    // Delineate the tweets with <start_tweet> and </end_tweet>
    let mut delineated_tweet_contents = String::new();
    for (_, tweet_data) in top_25_tweets {
        delineated_tweet_contents.push_str("\n<start_tweet>");
        delineated_tweet_contents.push_str(&tweet_data.content);
        delineated_tweet_contents.push_str("</end_tweet>");
    }
    // println!("{}", delineated_tweet_contents);

    // Make the prompt for the llm
    let prompt = format!("Given this list of the top 25 most popular tweets in my feed today, write me the 3-6 most common topics. The goal here is to find the current thing people are talking about. For each current thing, write it in no more than 1-3 sentences. Only answer with these topics, don't give a prelude, just the list of topics in the form of a json list, and nothing else!
    Don't touch on topics that aren't about something that is very current. It has to be about something very specific. One example is some recent world event like the G20, or some topic of debate. A good example is the man vs bear debate that happened at one point. Try to infer and convey as best as possible what these topics are. 
    The goal is to use your answer as prompts for an image generator, in order to generate good meme templates regarding that topic. 
    Don't be vague about the topics, talk about the exact specific topic. It's better to be more specific than vague. 
    \n\n {}", delineated_tweet_contents);

    // Send the prompt to llama3-70b
    match get_groq_answer(&prompt) {
        Ok(response) => println!("{}", response),
        Err(e) => println!("Failed to get response from llm: {}", e),
    }
}
