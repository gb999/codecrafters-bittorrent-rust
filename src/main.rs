use serde_bytes::ByteBuf;
use serde_json::{self, Map, Value, Number};
use std::{env, str::FromStr, io::{Write, Read, self, Error}, net::TcpStream, fs};
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

impl Torrent {
    fn from_file(file_path: &String) -> Result<Self, io::Error> {
        let contents = match std::fs::read(file_path) {
            Ok(contents) => contents,
            Err(error) => return Err(Error::new(io::ErrorKind::NotFound, error))
        };
        return Ok(serde_bencode::from_bytes::<Torrent>(&contents).unwrap());
    }
    fn print_info(&self) {
        println!("Tracker URL: {}", self.announce);
        println!("Length: {}", self.info.length);
        println!("Info Hash: {}", hex::encode(self.info.get_sha1_hash()));
        println!("Piece Length: {}", self.info.piece_length);
        println!("Piece Hashes:");
        for chunk in self.info.pieces.chunks(20) {
            let hash = hex::encode(chunk);
            println!("{hash}");
        }
    }
}

#[derive(Serialize, Deserialize)]
struct TorrentInfo {
    length: u32,
    name: String, 
    #[serde(rename = "piece length")]
    piece_length: u32,
    pieces: ByteBuf
}

impl TorrentInfo {
    fn get_sha1_hash(&self) -> Vec<u8> {
        let bytes = serde_bencode::to_bytes(&self).unwrap();
        Sha1::digest(bytes).to_vec()
    }
}

// Usage: your_bittorrent.sh decode "<encoded_value>"
fn decode(encoded_value: &String) {
    eprintln!("Logs from your program will appear here!");
    let decoded_value = decode_bencoded_value(encoded_value).0;
    println!("{}", decoded_value.to_string());
}

fn info(file_path: &String) {
    Torrent::from_file(file_path)
        .unwrap()
        .print_info();
}

fn get_peer_list(torrent: &Torrent) -> TrackerResponse {
    let res = reqwest::blocking::get(
        format!("{}?info_hash={}&peer_id={}&port={}&uploaded={}&downloaded={}&left={}&compact={}", 
        torrent.announce, 
        urlencoding::encode_binary(&torrent.info.get_sha1_hash()), 
        18693067284950604732 as i128, // peer_id
        6881, // port
        0, // uploaded
        0, // downloaded
        torrent.info.length,
        1 // compact
        )
    ).unwrap();
    serde_bencode::from_bytes::<TrackerResponse>(&res.bytes().unwrap()).unwrap()
}
 
fn peers(file_path: &String) {
    let torrent = Torrent::from_file(file_path).unwrap();
    let res = get_peer_list(&torrent);
    res.print_peers();
}

fn perform_handshake(torrent: &Torrent, peer_addr: &String) -> TcpStream {
    let handshake = HandShake {
        length: [19],
        protocol: *b"BitTorrent protocol",
        reserved: [0; 8],
        info_hash: torrent.info.get_sha1_hash().try_into().unwrap(),
        peer_id: [0,0,1,1,2,2,3,3,4,4,5,5,6,6,7,7,8,8,9,9]
    };
    let mut stream = std::net::TcpStream::connect(peer_addr).unwrap();
    stream.write(&handshake.as_bytes()).unwrap();
    return stream;
}
fn read_peer_id(stream: &mut TcpStream) -> String {
    let mut buf = [0u8; 68];
    stream.read(&mut buf).unwrap();
    hex::encode(&buf[48..])
}
fn handshake(file_path: &String, peer_addr: &String) {
    let torrent = Torrent::from_file(file_path).unwrap();
    let mut stream = perform_handshake(&torrent, peer_addr);
    let peer_id = read_peer_id(&mut stream);
    println!("Peer ID: {}", peer_id);
}

fn main() {
    let args: Vec<String> = env::args().collect();
    let command = &args[1];
    match command.as_str() {
        "decode" => decode(&args[2]),
        "info" => info(&args[2]),
        "peers" => peers(&args[2]),
        "handshake" => handshake(&args[2], &args[3]),
        "download_piece" => {
            let download_location = &args[3];
            let file_path = &args[4];
            let piece_index: u32 = args[5].parse().unwrap();
            let torrent = Torrent::from_file(file_path).unwrap();
            let peer_addresses = get_peer_list(&torrent).get_peer_addresses();
            let peer_address = peer_addresses.first().unwrap();
            let mut stream = perform_handshake(&torrent, peer_address);
            read_peer_id(&mut stream);
            PeerMessage::read_message(&mut stream); // Read bitfield message
            send_interested(&mut stream);
            PeerMessage::read_message(&mut stream); // Read unchoke message

            let mut piece_data: Vec<u8> = Vec::new(); 
            let mut remaining_bytes = torrent.info.piece_length;
            let mut block_index = 0;
            while remaining_bytes != 0 {
                let mut block_length = 16 * 1024;
                remaining_bytes -= block_length;
                if remaining_bytes < 16 * 1024 {
                    block_length = remaining_bytes;
                }
                send_request_piece(&mut stream, piece_index, block_index * (16 * 1024), block_length);
                
                if let PeerMessage::Piece {index:_, begin:_, block} = PeerMessage::read_message(&mut stream) {
                    piece_data.extend(block);
                    // Check integrity
                } else {panic!("Invalid response.") ; }

                block_index += 1;
            }
            fs::write(download_location, piece_data).unwrap();
           
        },
        _ => eprintln!("Unknown command: {}", args[1])
    } 
}
enum PeerMessage {
    BitField,
    Interested,
    Unchoke,
    Request,
    Piece {
        index: u32,
        begin: u32,
        block: Vec<u8>
    }
}

impl PeerMessage {
    fn read_message(stream: &mut TcpStream) -> Self {
        let mut message_size: [u8; 4] = [0u8; 4];
        stream.read_exact(&mut message_size).unwrap();
        let message_size = u32::from_be_bytes(message_size);
        let mut buf = vec![0; message_size as usize];
        stream.read_exact(&mut buf).unwrap();
        let message_id = buf[0];
        match message_id {
            5 => {
                // Read and ignore payload
                PeerMessage::BitField
            },
            1 => {
                PeerMessage::Unchoke
            },
            7 => {
                let mut index = [0u8; 4];
                index.copy_from_slice(&buf[0..4]);
                let mut begin = [0u8; 4];
                begin.copy_from_slice(&buf[4..8]);
                PeerMessage::Piece {
                    index: u32::from_be_bytes(index),
                    begin: u32::from_be_bytes(begin),
                    block: (&buf[8..]).to_vec()
                }
            },
            _=> panic!("Unexpected message")
        }
    }

}

fn send_interested(stream: &mut TcpStream) {
    let mut buf = [0; 5]; 
    buf[0..4].copy_from_slice(&1u32.to_be_bytes()); // 1 is message length
    buf[4] = 2; // message id 2 = interested   
    stream.write_all(&buf).unwrap();
}
fn send_request_piece(stream: &mut TcpStream, index:u32, begin:u32, length:u32) {
    let mut buf =  [0u8; 17];
    buf[0..4].copy_from_slice(&(1 + 3 * 4u32).to_be_bytes()); // message length 
    buf[4] = 6; // message id 6 = request
    buf[5..9].copy_from_slice(&index.to_be_bytes()); // index 
    buf[9..13].copy_from_slice(&begin.to_be_bytes()); // begin
    buf[13..17].copy_from_slice(&length.to_be_bytes()); // length 
    stream.write_all(&buf).unwrap();
}

#[derive(Serialize, Deserialize, Debug)]
struct TrackerResponse {
    // complete: u32,
    // incomplete: u32,
    interval: u32,
    peers: ByteBuf
}

impl TrackerResponse {
    fn get_peer_addresses(&self) -> Vec<String> {
        let mut res = Vec::new();
        for peer in self.peers.chunks(6) {
            let ip_addr = format!("{}.{}.{}.{}:{}",
                    peer[0],
                    peer[1],
                    peer[2],
                    peer[3],
                    (peer[4] as u16) << 8 | peer[5] as u16
            );
            res.push(ip_addr);
        }
        return res;
    }
    fn print_peers(&self) {
        for peer in self.get_peer_addresses() {
            println!("{peer}");
        }
    }
}

struct HandShake {
    length: [u8; 1],
    protocol: [u8; 19],
    reserved: [u8; 8],
    info_hash: [u8; 20],
    peer_id: [u8; 20]
}

impl HandShake {
    fn as_bytes(&self) -> Vec<u8> {
        let mut res: Vec<u8> = Vec::with_capacity(68);
        res.extend_from_slice(&self.length); 
        res.extend_from_slice(&self.protocol); 
        res.extend_from_slice(&self.reserved); 
        res.extend_from_slice(&self.info_hash); 
        res.extend_from_slice(&self.peer_id);
        res
    }
}