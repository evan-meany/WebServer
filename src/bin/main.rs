use std::fs;
use std::net::TcpListener;
use std::net::TcpStream;
use std::io::prelude::*;
use std::thread;
use std::time::Duration;
use web_server::ThreadPool;

extern crate rand;
use rand::seq::SliceRandom;

use serde::{Serialize, Deserialize};
use serde_json::json;

use lazy_static::lazy_static;
use std::sync::Mutex;

lazy_static! {
    static ref GLOBAL_COUNTER: Mutex<usize> = Mutex::new(0);
}

#[derive(Copy, Clone, Serialize)]
enum CardSuit {
    Clubs, Diamonds, Hearts, Spades 
}
#[derive(Copy, Clone, Serialize)]
enum CardValue {
    Ace = 1, Two, Three, Four, Five, Six, Seven, Eight, Nine, Ten, Jack, Queen, King = 13
}

#[derive(Serialize)]
struct Card {
    suit: CardSuit,
    value: CardValue
}

#[derive(Serialize)]
struct Player {
    id: usize,
    money: f64,
    cards: Vec<Card>,
}

impl Player {
    fn new() -> Self {
        let mut counter = GLOBAL_COUNTER.lock().unwrap();
        *counter += 1;
        Player{id: *counter, money: 100.0, cards: Vec::new()}
    }
}

fn shuffle_deck(deck: &mut Vec<Card>) {
    // Shuffle the deck using the Fisher-Yates algorithm
    let mut rng = rand::thread_rng();
    deck.shuffle(&mut rng);
}

fn new_deck() -> Vec<Card> {
    let mut deck: Vec<Card> = Vec::new();
    let suits: Vec<CardSuit> = vec![CardSuit::Clubs, CardSuit::Diamonds, CardSuit::Hearts, CardSuit::Spades];
    let values: Vec<CardValue> = vec![CardValue::Ace, CardValue::Two, CardValue::Three, CardValue::Four, CardValue::Five,
                                      CardValue::Six, CardValue::Seven, CardValue::Eight, CardValue::Nine, CardValue::Ten,
                                      CardValue::Jack, CardValue::Queen, CardValue::King];
    for suit in &suits {
        for value in &values {
            let card = Card{suit: *suit, value: *value};
            deck.push(card);
        }
    }

    shuffle_deck(&mut deck);

    return deck;
}

struct Blackjack {
    deck: Vec<Card>,
    house: Vec<Card>,
    players: Vec<Player>
}

impl Blackjack {
    fn new(num_players: usize) -> Self {
        let mut players: Vec<Player> = Vec::new();

        for _ in 0..num_players {
            players.push(Player::new());
        }

        Blackjack { deck: Vec::new(), house: Vec::new(), players: players}
    }

    fn deal(&mut self) {
        self.deck = new_deck();

        for player in &mut self.players {
            player.cards.push(self.deck.pop().unwrap());
        }

        self.house.push(self.deck.pop().unwrap());

        for player in &mut self.players {
            player.cards.push(self.deck.pop().unwrap());
        }

        self.house.push(self.deck.pop().unwrap());
    }

    fn game_state_as_json(&self) -> String {
        // Serialize players and house to JSON
        let players_json = serde_json::to_value(&self.players).unwrap();
        let house_json = serde_json::to_value(&self.house).unwrap();
    
        // Create a JSON object with "Players" and "House" keys
        let mut game_state_json = json!({});
        game_state_json["players"] = players_json; // Assign the JSON array directly
        game_state_json["house"] = house_json;     // Assign the JSON array directly
    
        // Convert the JSON object to a JSON string
        let result_json_string = serde_json::to_string(&game_state_json).unwrap();
    
        result_json_string
    }
    
}

fn handle_connection(mut stream: TcpStream, blackjack: &mut Blackjack) {
    // This buffer is 1024 bytes long
    let mut buffer = [0; 1024];
    stream.read(&mut buffer).unwrap();
    // println!("Buffer: {}", String::from_utf8_lossy(&buffer[..]));
    
    let home = b"GET / HTTP/1.1\r\n";
    let sleep = b"GET /sleep HTTP/1.1\r\n";
    let post_request = b"POST /hit-stay HTTP/1.1\r\n";
    let fetch_request = b"GET /server-endpoint HTTP/1.1\r\n";
    let start_request = b"GET /start-game HTTP/1.1\r\n";

    let (status_line, filename) = 
        if buffer.starts_with(home) {
            ("HTTP/1.1 200 OK", "index.html")
        } else if buffer.starts_with(sleep) {
            thread::sleep(Duration::from_secs(5));
            ("HTTP/1.1 200 OK", "index.html")
        } else if buffer.starts_with(post_request) {
            let js_function_call = "<script>addPlayer(1);</script>";
            let response_body = fs::read_to_string("index.html").unwrap_or_else(|_| String::from("Error: Could not read index.html"));
        
            // Insert the JavaScript function call within the <body> section of the HTML
            let modified_response_body = response_body.replace("</body>", &format!("{}\n</body>", js_function_call));
        
            let response = format!(
                "HTTP/1.1 200 OK\r\nContent-Type: text/html\r\n\r\n{}",
                modified_response_body
            );
        
            stream.write(response.as_bytes()).unwrap();
            stream.flush().unwrap();
            return;
        } else if buffer.starts_with(fetch_request) {
            let response = format!(
                "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\n\r\n{{\"message\": \"Hello, JSON!\"}}"
            );
        
            stream.write(response.as_bytes()).unwrap();
            stream.flush().unwrap();
            return;
        } else if buffer.starts_with(start_request) {
            blackjack.deal();
            let game_json = blackjack.game_state_as_json();

            let response = format!(
                "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\n\r\n{}",
                game_json
            );
        
            println!("RESPONSE:: {game_json}");

            stream.write(response.as_bytes()).unwrap();
            stream.flush().unwrap();
            return;
        } else {
            ("HTTP/1.1 404 NOT FOUND", "404.html")
        };

    let contents = fs::read_to_string(filename).unwrap();

    // response format:
    // HTTP-Version Status-Code Reason-Phrase CRLF
    // headers CRLF (Character Return Line Feed sequence - \r\n)
    // message-body
    //
    // ex: HTTP/1.1 200 Ok\r\n\r\n [contains no headers/message-body]
    let response = format!(
        "{}\r\nContent-Length: {}\r\n\r\n{}",
        status_line,
        contents.len(),
        contents
    );

    stream.write(response.as_bytes()).unwrap();
    stream.flush().unwrap();
}

fn main() {
    // 127.0.0.1 is local host ip address and 7878 is the port number 
    let listener = TcpListener::bind("127.0.0.1:7878").unwrap();
    let pool = ThreadPool::new(4);
    let mut blackjack = Blackjack::new(1);
    
    for stream in listener.incoming() {
        let stream = stream.unwrap();
        // pool.execute(|| { handle_connection(stream, &mut blackjack); });
        handle_connection(stream, &mut blackjack);
    }
}
