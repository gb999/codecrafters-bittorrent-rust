use serde_bytes::ByteBuf;
use serde_json::{self, Map, Value, Number};
use std::{env, str::FromStr};
use serde_derive::{Serialize, Deserialize};
use sha1::{Sha1, Digest};


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
    #[serde(rename = "piece length")]
    piece_length: u32,
    pieces: ByteBuf
}

fn torrent_from_file(file_path: &str) -> Option<Torrent> {
    let contents = match std::fs::read(file_path) {
        Ok(contents) => contents,
        Err(_) => {eprintln!("No such file."); return None}
    };

    return Some(serde_bencode::from_bytes::<Torrent>(&contents).unwrap());
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
            let torrent = torrent_from_file(file_path).unwrap();

            let bytes = serde_bencode::to_bytes(&torrent.info).unwrap();
            let hash = hex::encode(Sha1::digest(bytes));
            
            println!("Tracker URL: {}", torrent.announce);
            println!("Length: {}", torrent.info.length);
            println!("Info Hash: {}", hash);
            println!("Piece Length: {}", torrent.info.piece_length);
            println!("Piece Hashes:");
            for chunk in torrent.info.pieces.chunks(20) {
                let hash = hex::encode(chunk);
                println!("{hash}");
            }
            
        },
        "peers" => {
            let file_path = &args[2];
            let torrent = torrent_from_file(file_path).unwrap();

            let bytes = serde_bencode::to_bytes(&torrent.info).unwrap();
            let hash = Sha1::digest(bytes);

            let res = reqwest::blocking::get(
                format!("{}?info_hash={}&peer_id={}&port={}&uploaded={}&downloaded={}&left={}&compact={}", 
                torrent.announce, 
                urlencoding::encode_binary(&hash), 
                18693067284950604732, // peer_id
                6881, // port
                0, // uploaded
                0, // downloaded
                torrent.info.length,
                1 // compact
                )
            ).unwrap();
            let res = serde_bencode::from_bytes::<TrackerResponse>(&res.bytes().unwrap()).unwrap();

            for peer in res.peers.chunks(6) {
                let ip_addr = format!("{}.{}.{}.{}:{}",
                        peer[0],
                        peer[1],
                        peer[2],
                        peer[3],
                        (peer[4] as u16) << 8 | peer[5] as u16
                );
                println!("{ip_addr}");
            }
        }
        _ => println!("unknown command: {}", args[1])
    } 
}


#[derive(Serialize, Deserialize, Debug)]
struct TrackerResponse {
    // complete: u32,
    // incomplete: u32,
    interval: u32,
    peers: ByteBuf
}