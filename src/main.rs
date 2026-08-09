use image::DynamicImage;
use rpi_led_matrix::{LedColor, LedMatrix, LedMatrixOptions, LedRuntimeOptions};
use std::error::Error;
use std::net::TcpStream;
use std::time::Duration;
use tungstenite::{Message, client};

struct App {
    config: Config,
    state: State,
}

struct State {
    last_update: std::time::Instant,
    current_song: Option<String>,
}
struct Config {
    url: String,
    access_token: String,
}

fn main() {
    dotenvy::from_filename("/etc/coverboy.conf").unwrap_or_else(|e| {
        panic!("Failed to load .env file: {e}");
    });
    let mut options = LedMatrixOptions::new();
    options.set_cols(64);
    options.set_rows(64);
    options.set_chain_length(1);
    options.set_parallel(1);
    options.set_hardware_mapping("regular");
    options.set_pwm_lsb_nanoseconds(150);
    options.set_limit_refresh(0);
    options.set_brightness(75).unwrap();

    let mut rt_options = LedRuntimeOptions::new();
    rt_options.set_gpio_slowdown(0);
    let matrix = LedMatrix::new(Some(options), Some(rt_options)).unwrap();

    let mut canvas = matrix.offscreen_canvas();

    let mut app = App {
        config: Config {
            url: std::env::var("HA_URL").unwrap(),
            access_token: std::env::var("HA_TOKEN").unwrap(),
        },
        state: State {
            last_update: std::time::Instant::now(),
            current_song: None,
        },
    };
    println!("{:?}", app.config.url);
    let stream = TcpStream::connect(&app.config.url).expect("Could not connect to WebSocket");
    let (mut socket, response) =
        match client(format!("ws://{}/api/websocket", app.config.url), &stream) {
            Ok(ws) => ws,
            Err(e) => panic!("Could not connect to WebSocket due to {e}"),
        };

    stream.set_nonblocking(true).unwrap();

    println!("Connected to the server");
    println!("Response HTTP code: {}", response.status());
    println!("Response contains the following headers:");
    for (header, _value) in response.headers() {
        println!("* {header}");
    }

    loop {
        match socket.read() {
            Ok(Message::Text(text)) => {
                let result = handle_message(&mut app, &serde_json::from_str(&text).unwrap());
                match result {
                    MessageResult::Image(image) => {
                        println!("Creating frame");
                        let image = image.into_rgb8();
                        for (x, y, pixel) in image.enumerate_pixels() {
                            let x = i32::try_from(x).unwrap();
                            let y = i32::try_from(y).unwrap();
                            let red = pixel.0[0];
                            let green = pixel.0[1];
                            let blue = pixel.0[2];
                            canvas.set(x, y, &LedColor { red, green, blue });
                        }
                        println!("Loading frame");
                        canvas = matrix.swap(canvas);
                        println!("Loaded frame");
                    }
                    MessageResult::Message(resp) => {
                        println!("Sending response: {resp}");
                        socket
                            .send(Message::Text(serde_json::to_string(&resp).unwrap().into()))
                            .unwrap();
                    }
                    MessageResult::None => {}
                }
            },
            Err(tungstenite::Error::Io(e))
                if e.kind() != std::io::ErrorKind::WouldBlock => {
                    println!("Error reading from socket: {e}");
                    break;
            }
            _ => {}
        }
        let now = std::time::Instant::now();
        if (now - app.state.last_update) > Duration::from_mins(10) {
            canvas.clear();
            canvas = matrix.swap(canvas);
        }
    }
    socket.close(None).unwrap();
}

enum MessageResult {
    Image(DynamicImage),
    Message(serde_json::Value),
    None,
}

fn handle_message(app: &mut App, msg: &serde_json::Value) -> MessageResult {
    if msg["type"] == "auth_required" {
        return MessageResult::Message(serde_json::json!({
            "type": "auth",
            "access_token": app.config.access_token
        }));
    }
    if msg["type"] == "auth_ok" {
        return MessageResult::Message(serde_json::json!({
            "id": 1,
            "type": "subscribe_events",
            "event_type": "state_changed"
        }));
    }
    if msg["type"] == "event"
        && msg["event"]["data"]["entity_id"]
            .as_str()
            .unwrap()
            .starts_with("media_player.")
        && msg["event"]["data"]["new_state"]["state"] == "playing"
        && msg["event"]["data"]["new_state"]["attributes"]["media_content_type"] == "music"
        && msg["event"]["data"]["new_state"]["attributes"]["media_title"].is_string()
    {
        app.state.last_update = std::time::Instant::now();
        let attrs = msg["event"]["data"]["new_state"]["attributes"].clone();
        if app.state.current_song == Some(attrs["media_title"].as_str().unwrap().to_string()) {
            return MessageResult::None;
        }
        println!(
            "New song: {} - {}",
            attrs["media_artist"].as_str().unwrap(),
            attrs["media_title"].as_str().unwrap()
        );

        let mut image = Err("No cover local found".into());
        if attrs["entity_picture_local"].is_string() {
            image = get_image(
                app,
                attrs["entity_picture_local"].as_str().unwrap().to_string(),
            );
        }
        if image.is_err() {
            println!("Failed to get local image: {}", image.as_ref().unwrap_err());
            if attrs["entity_picture"].is_string() {
                println!("Retrying with global image.");
                match get_image(app, attrs["entity_picture"].as_str().unwrap().to_string()) {
                    Ok(img) => image = Ok(img),
                    Err(e) => {
                        println!("Failed to get global image: {e}");
                        return MessageResult::None;
                    }
                }
            } else {
                println!("No global image found.");
                return MessageResult::None;
            }
        }

        let image = image.unwrap();
        app.state.current_song = Some(attrs["media_title"].as_str().unwrap().to_string());

        println!("Resizing image");
        let image = image.resize(64, 64, image::imageops::FilterType::Lanczos3);
        println!("Resized image");
        return MessageResult::Image(image);
    }
    MessageResult::None
}

fn get_image(app: &App, mut url: String) -> Result<DynamicImage, Box<dyn Error>> {
    if url.starts_with('/') {
        url = app.config.url.clone() + &url;
    }
    println!("Downloading image");
    let image_bytes = reqwest::blocking::get(url)?.bytes()?;
    println!("Downloaded image");
    println!("Loading image");
    let image = image::load_from_memory(&image_bytes)?;
    println!("Loaded image");
    Ok(image)
}
