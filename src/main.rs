use serde_json;
use std::{env, str::FromStr};

// Available if you need it!
// use serde_bencode

fn decode_bencoded_string(encoded_value: &str) -> (serde_json::Value, &str) {
    let colon_index = encoded_value.find(':').unwrap();
    let number_string = &encoded_value[..colon_index];
    let number = number_string.parse::<i64>().unwrap();
    let string = &encoded_value[colon_index + 1..colon_index + 1 + number as usize];
    return (serde_json::Value::String(string.to_string()), &encoded_value[colon_index + 1 + number as usize..]);
}
fn decode_bencoded_integer(encoded_value: &str) -> (serde_json::Value, &str) {
    let end_index = encoded_value.find('e').unwrap();
    let number_string = &encoded_value[1..end_index];
    let number = serde_json::Number::from_str(number_string).unwrap();
    return (serde_json::Value::Number(number), &encoded_value[end_index+1..]);
}
fn decode_bencoded_list(encoded_value: &str) -> (serde_json::Value, &str) {
    let mut list:Vec<serde_json::Value> = Vec::new();
    let mut encoded_value = encoded_value;
    loop {
        let first_char = match encoded_value.chars().next() {
            Some(c) => c,
            None => break,
        };
        let (value, remaining_string) = match first_char {
            c  if c.is_digit(10) => decode_bencoded_string(encoded_value),
            'i' => decode_bencoded_integer(encoded_value),
            'l' => decode_bencoded_list(&encoded_value[1..]),
            'e' => {encoded_value = &encoded_value[1..]; break},
            _ => panic!("Unhandled encoded value: {}", encoded_value)
        };
        list.push(value);
        encoded_value = remaining_string;

    }
    return (serde_json::Value::Array(list), encoded_value);
}

#[allow(dead_code)]
fn decode_bencoded_value(encoded_value: &str) -> serde_json::Value {
    return match encoded_value.chars().next().unwrap() {
        c  if c.is_digit(10) => decode_bencoded_string(encoded_value).0,
        'i' => decode_bencoded_integer(encoded_value).0,
        'l' => decode_bencoded_list(&encoded_value[1..]).0,
        _ => panic!("Unhandled encoded value: {}", encoded_value)
    };
}

// Usage: your_bittorrent.sh decode "<encoded_value>"
fn main() {
    let args: Vec<String> = env::args().collect();
    let command = &args[1];

    if command == "decode" {
        // You can use print statements as follows for debugging, they'll be visible when running tests.
        eprintln!("Logs from your program will appear here!");

        // Uncomment this block to pass the first stage
        let encoded_value = &args[2];
        let decoded_value = decode_bencoded_value(encoded_value);
        println!("{}", decoded_value.to_string());
    } else {
        println!("unknown command: {}", args[1])
    }
}
