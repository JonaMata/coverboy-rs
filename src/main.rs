use tungstenite::{connect, Message};
use std::sync::{Arc, Mutex};
use image::{DynamicImage, GenericImageView, Pixel};
use rpi_led_matrix::{LedMatrix, LedColor};

struct App {
    config: Config,
    state: Mutex<State>,
}

struct State {
    last_update: std::time::Instant,
    current_song: Option<String>,
}
struct Config {
    access_token: String,
}

fn main() {

    let matrix = LedMatrix::new(None, None).unwrap();

    let mut canvas = matrix.offscreen_canvas();

    let app = Arc::new(App {
        config: Config {
            access_token: std::env::var("HA_TOKEN").unwrap()
        },
        state: Mutex::new(State {
            last_update: std::time::Instant::now(),
            current_song: None,
        })
    });

    let (mut socket, response) = match connect(format!("ws://{}/api/websocket", std::env::var("HA_URL").unwrap())) {
        Ok(ws) => ws,
        Err(e) => panic!("Could not connect to WebSocket due to {}", e)
    };

    println!("Test");
    println!("Connected to the server");
    println!("Response HTTP code: {}", response.status());
    println!("Response contains the following headers:");
    for (header, _value) in response.headers() {
        println!("* {header}");
    }

    loop {
        let msg = socket.read().expect("Error reading message");
        match msg {
            Message::Text(text) => {
                let result = handle_message(app.clone(), serde_json::from_str(&text).unwrap());
                match result {
                    MessageResult::Image(image) => {
                        println!("Creating frame");
                        for (x, y, pixel) in image.pixels() {
                            let pixel = pixel.clone();
                            let red = pixel.to_rgb().channels()[0];
                            let green = pixel.to_rgb().channels()[1];
                            let blue = pixel.to_rgb().channels()[2];
                            canvas.set(x as i32, y as i32, &LedColor{red, green, blue});
                        }
                        println!("Loading frame");
                        canvas = matrix.swap(canvas);
                        println!("Loaded frame");
                    }
                    MessageResult::Message(resp) => {
                        println!("Sending response: {}", resp);
                        socket.send(Message::Text(serde_json::to_string(&resp).unwrap().into())).unwrap();
                    },
                    MessageResult::None => {}
                }
            }
            _ => {}
        }
    }
    // socket.close(None);
}

enum MessageResult {
    Image(DynamicImage),
    Message(serde_json::Value),
    None
}

fn handle_message(app: Arc<App>, msg: serde_json::Value) -> MessageResult {
    if msg["type"] == "auth_required" {
        return MessageResult::Message(serde_json::json!({
            "type": "auth",
            "access_token": app.config.access_token
        }))
    }
    if msg["type"] == "auth_ok" {
        return MessageResult::Message(serde_json::json!({
            "id": 1,
            "type": "subscribe_events",
            "event_type": "state_changed"
        }))
    }
    if msg["type"] == "event" &&
        msg["event"]["data"]["entity_id"].as_str().unwrap().starts_with("media_player.") &&
        msg["event"]["data"]["new_state"]["state"] == "playing" &&
        msg["event"]["data"]["new_state"]["attributes"]["media_content_type"] == "music" {
        let mut state = app.state.lock().unwrap();
        state.last_update = std::time::Instant::now();
        let attrs = msg["event"]["data"]["new_state"]["attributes"].clone();
        if state.current_song == Some(attrs["media_title"].as_str().unwrap().to_string()) {
            return MessageResult::None
        }
        state.current_song = Some(attrs["media_title"].as_str().unwrap().to_string());
        let mut cover_url: String = {
        if attrs["entity_picture"].is_string() {
            attrs["entity_picture"].as_str().unwrap().to_string()
        } else if attrs["entity_picture_local"].is_string() {
                attrs["entity_picture_local"].as_str().unwrap().to_string()
            } else {
                "".to_string()
            }
        };
        println!("New song: {} - {} (cover: {})", attrs["media_artist"].as_str().unwrap(), attrs["media_title"].as_str().unwrap(), cover_url);

        if cover_url.is_empty() {
            return MessageResult::None
        }

        if cover_url.starts_with("/") {
            cover_url = "http://192.168.1.102:8123".to_string() + &cover_url;
        }
        println!("Downloading image");
        let image_bytes = match reqwest::blocking::get(&cover_url) {
            Ok(resp) => match resp.bytes() {
                Ok(bytes) => bytes,
                Err(e) => {
                    println!("Error getting bytes from {}: {}", cover_url, e);
                    return MessageResult::None
                },
            },
            Err(e) => {
                println!("Error fetching cover image: {}", e);
                return MessageResult::None
            }
        };
        println!("Downloaded image");
        println!("Loading image");
        let image = match image::load_from_memory(&image_bytes) {
            Ok(image) => image,
            Err(e) => {
                println!("Error loading image from memory: {}", e);
                return MessageResult::None
            }
        };
        println!("Loaded image");
        println!("Resizing image");
        let image = image.resize(64, 64, image::imageops::FilterType::Lanczos3);
        println!("Resized image");
        return MessageResult::Image(image);
    }
    MessageResult::None
}