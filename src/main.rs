use serde_json::{self, Map, Value, Number};
use std::{env, str::FromStr};
use serde_derive::{Serialize, Deserialize};


fn decode_bencoded_string(encoded_value: &str) -> (Value, &str) {
    let colon_index = encoded_value.find(':').unwrap();
    let number_string = &encoded_value[..colon_index];
    let number = number_string.parse::<i64>().unwrap();
    let string = &encoded_value[colon_index + 1..colon_index + 1 + number as usize];
    return (Value::String(string.to_string()), &encoded_value[colon_index + 1 + number as usize..]);
}
fn decode_bencoded_integer(encoded_value: &str) -> (Value, &str) {
    let end_index = encoded_value.find('e').unwrap();
    let number_string = &encoded_value[1..end_index];
    let number = Number::from_str(number_string).unwrap();
    return (Value::Number(number), &encoded_value[end_index + 1..]);
}
fn decode_bencoded_list(encoded_value: &str) -> (Value, &str) {
    let mut list:Vec<Value> = Vec::new();
    let mut encoded_value = encoded_value;
    loop {
        match encoded_value.chars().next() { 
            Some(c) if c == 'e' => {encoded_value = &encoded_value[1..]; break},
            Some(_) => (),
            None => break,
        };
        let (value, remaining_string) = decode_bencoded_value(encoded_value);
        list.push(value);
        encoded_value = remaining_string;

    }
    return (Value::Array(list), encoded_value);
}

fn decode_bencoded_dictionary(encoded_value: &str) -> (Value, &str) {
    let mut dictionary = Map::new();
    let mut encoded_value = encoded_value; 
    loop {
        match encoded_value.chars().next() {
            Some(c) if c == 'e' => {encoded_value = &encoded_value[1..]; break},
            Some(_) => (),
            None => break,
        }
        let (key, remaining_string) = decode_bencoded_string(encoded_value);
        let (data, remaining_string) = decode_bencoded_value(remaining_string);
        dictionary.insert(String::from(key.as_str().unwrap()), data);
        encoded_value = remaining_string;
    }
    return (Value::Object(dictionary), encoded_value);
}

#[allow(dead_code)]
fn decode_bencoded_value(encoded_value: &str) -> (Value, &str) {
    return match encoded_value.chars().next().unwrap() {
        c  if c.is_digit(10) => decode_bencoded_string(encoded_value),
        'i' => decode_bencoded_integer(encoded_value),
        'l' => decode_bencoded_list(&encoded_value[1..]),
        'd' => decode_bencoded_dictionary(&encoded_value[1..]),
        _ => panic!("Unhandled encoded value: {}", encoded_value)
    };
}

#[derive(Serialize, Deserialize)]
struct Torrent {
    announce: String,
    info: TorrentInfo,
}

#[derive(Serialize, Deserialize)]
struct TorrentInfo {
    length: u32,
    name: String, 
    
}


// Usage: your_bittorrent.sh decode "<encoded_value>"
fn main() {
    let args: Vec<String> = env::args().collect();
    let command = &args[1];

    match command.as_str() {
        "decode" => {
            // You can use print statements as follows for debugging, they'll be visible when running tests.
            eprintln!("Logs from your program will appear here!");
    
            let encoded_value = &args[2];
            let decoded_value = decode_bencoded_value(encoded_value).0;
            println!("{}", decoded_value.to_string());
        },
        "info" => {
            let file_path = &args[2];
            let contents = match std::fs::read(file_path) {
                Ok(contents) => contents,
                Err(_) => {eprintln!("No such file."); return}
            };
            
            let torrent = serde_bencode::from_bytes::<Torrent>(&contents).unwrap();
            println!("Tracker URL: {}",torrent.announce);
            println!("Length: {}", torrent.info.length);
        },
        _ => println!("unknown command: {}", args[1])
    } 
}
