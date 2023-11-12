use inputbot::KeybdKey;
use std::collections::HashSet;
use std::fs::File;
use std::io::BufReader;
use std::sync::mpsc;
use std::sync::mpsc::{Receiver, Sender};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Instant;
struct TimedKeyInput {
    timestamp: Instant,
    key: char,
}
use std::io::Read;

fn main() {
    // Bind the number 1 key your keyboard to a function that types
    // "Hello, world!" when pressed.

    let (tx, rx): (Sender<TimedKeyInput>, Receiver<TimedKeyInput>) = mpsc::channel();

    let _processor = thread::spawn(|| message_processor(rx));

    KeybdKey::bind_all(move |event| {
        match inputbot::from_keybd_key(event) {
            Some(c) => {
                let instant = Instant::now();
                let message = TimedKeyInput {
                    timestamp: instant,
                    key: c,
                };
                tx.send(message).unwrap();
            }
            None => println!("Unregistered Key"),
        };
    });

    // Call this to start listening for bound inputs.
    inputbot::handle_input_events();
}

fn message_processor(rx: Receiver<TimedKeyInput>) {
    let (tx_audio, rx_audio): (Sender<bool>, Receiver<bool>) = mpsc::channel();

    let last_time = Arc::new(Mutex::new(Instant::now()));
    let last_is_left: Arc<Mutex<Option<bool>>> = Arc::new(Mutex::new(None));

    let _audio_handler = thread::spawn(|| audio_player(rx_audio));

    let (left_keys, right_keys) = get_config().unwrap();
    loop {
        let event = rx.recv().unwrap();
        let key = event.key;
        let is_left = if left_keys.contains(&key) {
            Some(true)
        } else if right_keys.contains(&key) {
            Some(false)
        } else {
            None
        };
        let last_instant = last_time.lock().unwrap();
        let time_delta = event.timestamp.duration_since(last_instant.clone());
        drop(last_instant);
        println!("{key}: {:?}", time_delta);
        let last_is_left_val = *last_is_left.lock().unwrap();

        if let Some(last_is_left_val) = last_is_left_val {
            if let Some(is_left) = is_left {
                if is_left == last_is_left_val {
                    let time_copy = last_time.clone();
                    let last_is_left_copy = Arc::clone(&last_is_left);
                    let tx_audio_copy = tx_audio.clone();
                    thread::spawn(move || {
                        thread::sleep(std::time::Duration::from_millis(10));
                        let cur_is_left_option = last_is_left_copy.lock().unwrap();
                        let cur_time = time_copy.lock().unwrap();
                        if let Some(cur_is_left) = *cur_is_left_option {
                            if cur_is_left == last_is_left_val
                                && *cur_time == event.timestamp.clone()
                            {
                                println!("Two inputs from same hand detected!");
                                tx_audio_copy.send(true).unwrap();
                            }
                        }
                    });
                }
            }
        }

        // TODO: clean this up
        {
            *last_is_left.lock().unwrap() = {
                if time_delta.as_millis() < 10 {
                    None
                } else {
                    is_left
                }
            };
            *last_time.lock().unwrap() = event.timestamp;
        }
    }
}

// TODO: change this to something lower weight
fn audio_player(rx: Receiver<bool>) {
    let (_stream, stream_handle) = rodio::OutputStream::try_default().unwrap();
    loop {
        rx.recv().unwrap();
        let file = File::open("assets\\warn.mp3").unwrap();
        // let file_copy = file.try_clone().unwrap();
        let buf = BufReader::new(file);
        let _beep1 = stream_handle.play_once(buf).unwrap();
        std::thread::sleep(std::time::Duration::from_millis(400));
    }
}

fn get_config() -> Result<(HashSet<char>, HashSet<char>), &'static str> {
    // Config reading

    let mut file = File::open("config.json").unwrap();
    let mut data = String::new();
    file.read_to_string(&mut data).unwrap();

    let parsed_settings = jzon::parse(&data).unwrap();

    let left_keys = match parsed_settings["left_keys"].as_array() {
        Some(x) => x,
        None => {
            return Err("Error with field left_keys");
        }
    };
    let mut left_keys_char: HashSet<char> = HashSet::new();
    for item in left_keys {
        let item_string = match item.as_str() {
            Some(x) => x,
            None => {
                return Err("Error with field left_keys");
            }
        };
        if item_string.chars().count() != 1 {
            return Err("Error with field left_keys");
        }
        left_keys_char.insert(item_string.chars().next().unwrap());
    }

    let right_keys = match parsed_settings["right_keys"].as_array() {
        Some(x) => x,
        _ => {
            return Err("Error with field right_keys");
        }
    };
    let mut right_keys_char: HashSet<char> = HashSet::new();
    for item in right_keys {
        let item_string = match item.as_str() {
            Some(x) => x,
            None => {
                return Err("Error with field right_keys");
            }
        };
        if item_string.chars().count() != 1 {
            return Err("Error with field right_keys");
        }
        right_keys_char.insert(item_string.chars().next().unwrap());
    }
    return Ok((left_keys_char, right_keys_char));
}
