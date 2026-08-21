use heic::{DecoderConfig, PixelLayout};
use image::{DynamicImage, ImageReader, RgbImage};
use rpi_led_panel::{HardwareMapping, RGBMatrix, RGBMatrixConfig};
use std::error::Error;
use std::net::TcpStream;
use std::sync::{Arc, Mutex};
use std::thread;
use std::thread::sleep;
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
    // register_all_decoding_hooks();
    dotenvy::from_filename("/etc/coverboy.conf").unwrap_or_else(|e| {
        panic!("Failed to load .env file: {e}");
    });
    let config = RGBMatrixConfig {
        hardware_mapping: HardwareMapping::regular(),
        cols: 64,
        rows: 64,
        refresh_rate: 0,
        led_brightness: 75,
        ..Default::default()
    };

    let (mut matrix, mut canvas) = RGBMatrix::new(config, 0).expect("Couldn't create matrix.");

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

    let matrix_image: Arc<Mutex<Option<RgbImage>>> = Arc::new(Mutex::new(None));

    let thread_image = matrix_image.clone();

    thread::spawn(move || {
        loop {
            let matrix_image = thread_image.lock().unwrap().clone();
            if let Some(image) = matrix_image {
                for (x, y, pixel) in image.enumerate_pixels() {
                    let x = usize::try_from(x).unwrap();
                    let y = usize::try_from(y).unwrap();
                    canvas.set_pixel(x, y, pixel[0], pixel[1], pixel[2]);
                }
            }
            canvas = matrix.update_on_vsync(canvas);
        }
    });

    let mut connection_attempts = 0;

    loop {
        println!("Connection attempt {connection_attempts}:");
        let Ok(stream) = TcpStream::connect(&app.config.url) else {
            println!("Could not create TCP connection.");
            break;
        };
        let (mut socket, response) =
            match client(format!("ws://{}/api/websocket", app.config.url), &stream) {
                Ok(ws) => ws,
                Err(e) => {
                    println!("Could not connect to WebSocket due to {e}");
                    break;
                }
            };
        println!("Connected to the server");
        println!("Response HTTP code: {}", response.status());
        println!("Response contains the following headers:");
        for (header, _value) in response.headers() {
            println!("* {header}");
        }
        connection_attempts = 0;
        loop {
            let now = std::time::Instant::now();
            if (now - app.state.last_update) > Duration::from_mins(10) {
                let mut mut_image = matrix_image.lock().unwrap();
                *mut_image = None;
            }
            match socket.read() {
                Ok(Message::Text(text)) => {
                    let result = handle_message(&mut app, &serde_json::from_str(&text).unwrap());
                    match result {
                        MessageResult::Image(image) => {
                            println!("Loading new image.");
                            let mut mut_image = matrix_image.lock().unwrap();
                            *mut_image = Some(image);
                        }
                        MessageResult::Message(resp) => {
                            println!("Sending response: {resp}");
                            socket
                                .send(Message::Text(serde_json::to_string(&resp).unwrap().into()))
                                .unwrap();
                        }
                        MessageResult::None => {}
                    }
                }
                Err(e) => {
                    println!("Error: {e}");
                    break;
                }
                _ => {}
            }
        }
        socket.close(None).unwrap();
        println!("Disconnected, retrying to connect in 30 seconds.");
        connection_attempts += 1;
        sleep(Duration::from_secs(30));
    }
}

enum MessageResult {
    Image(RgbImage),
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
            println!("Failed to get local image");
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

        println!("Resized image");
        return MessageResult::Image(image);
    }
    MessageResult::None
}

fn get_image(app: &App, mut url: String) -> Result<RgbImage, Box<dyn Error>> {
    if url.starts_with('/') {
        url = "http://".to_string() + &app.config.url.clone() + &url;
    }
    println!("Downloading image from {url}");
    let image_bytes = reqwest::blocking::get(url)?.bytes()?;
    println!("Downloaded image");
    println!("Loading image");

    let mut image = try_image_decode(&image_bytes);
    if image.is_err() {
        println!("Failed to decode image, trying HEIC...");
        image = try_heic_decode(&image_bytes);
    }
    println!("Loaded image");
    image
}

fn try_image_decode(image_bytes: &[u8]) -> Result<RgbImage, Box<dyn Error>> {
    let image = ImageReader::new(std::io::Cursor::new(image_bytes))
        .with_guessed_format()?
        .decode()?;
    Ok(image
        .resize(64, 64, image::imageops::FilterType::Lanczos3)
        .into_rgb8())
}

fn try_heic_decode(data: &[u8]) -> Result<RgbImage, Box<dyn Error>> {
    let output = DecoderConfig::new().decode(data, PixelLayout::Rgb8)?;
    let rgb_image = RgbImage::from_raw(output.width, output.height, output.data)
        .ok_or("Failed to create RgbImage from HEIC data")?;
    Ok(DynamicImage::ImageRgb8(rgb_image)
        .resize_exact(64, 64, image::imageops::FilterType::Lanczos3)
        .into_rgb8())
}
