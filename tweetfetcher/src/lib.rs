use kinode_process_lib::{call_init, println, Address, Request};

// use chrono::{DateTime, TimeZone, Utc};
use std::time::{SystemTime, UNIX_EPOCH};

use storage_interface::Request as StorageRequest;
use storage_interface::Response as StorageResponse;

wit_bindgen::generate!({
    path: "target/wit",
    world: "tweetfetcher-template-dot-os-v0",
    generate_unused_types: true,
    additional_derives: [serde::Deserialize, serde::Serialize],
});

call_init!(init);
fn init(_our: Address) {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs() as i64;
    let yesterday = now - 86400 as i64; // Subtract 86400 seconds (1 day) from the current time

    let request = serde_json::to_vec(&StorageRequest::GetTweets {
        start_time: yesterday,
        end_time: now,
    })
    .unwrap();

    // Send the request to storage
    let storage_address: (&str, &str, &str, &str) =
        ("our", "storage", "command_center", "appattacc.os");

    let Ok(Ok(response)) = Request::to(storage_address)
        .body(request)
        .send_and_await_response(30)
    else {
        panic!("What the fuck happened");
    };
    let body = response.body();
    let response: StorageResponse = serde_json::from_slice(&body).unwrap();
    println!("Response is {:?}", response);
}
