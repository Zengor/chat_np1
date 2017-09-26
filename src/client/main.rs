extern crate chat_np1;

use std::sync::mpsc;
use std::sync::{Arc};
use std::sync::atomic::{AtomicBool, Ordering};
use std::io::{stdin};
use std::thread;
use chat_np1::chatclient::ChatConnection;
use chat_np1::message::Message;

fn input_loop(sender: mpsc::Sender<String>,
              terminate: Arc<AtomicBool>) {
    let mut buf = String::new();
    'input: loop {
        let terminated = terminate.load(Ordering::Relaxed);
        if terminated { break 'input }

        buf.clear();
        stdin().read_line(&mut buf).unwrap();
        sender.send(buf.trim().to_owned()).unwrap();
    }
}

fn handle_input(input: String,
                connection: &mut ChatConnection,
                terminate: Arc<AtomicBool>) {
    if input.starts_with('/') {
        handle_command(input, connection, terminate);
    } else if input.starts_with('@') {
        connection.send_private_message(input);
    } else {
        connection.send_public_message(input);
    };
}

fn handle_command(input: String,
                  connection: &mut ChatConnection,
                  terminate: Arc<AtomicBool>) {
    let split_str: Vec<&str> = input.split(' ').collect();
    match split_str[0] {
        "/help" => {
            print_help();
        },
        "/list" => {
            connection.request_groups();
        },
        "/join" =>{ 
            if split_str[1..].len() != 1 {
                println!("requires single argument");
                return;
            }
            connection.join_chat(split_str[1].to_owned());
        },
        "/new" => {
            if split_str[1..].len() != 1 {
                println!("requires single argument");
                return;
            }
            connection.create_chat(split_str[1].to_owned())
        },
        "/leave" => {
            connection.leave_chat();
        },
        "/online" => {
            connection.request_clients();
        },
        "/kick" => {
            if split_str[1..].len() != 1 {
                println!("requires single argument");
                return;
            }
            connection.kick(split_str[1].to_owned());
        },
        "/quit" => {
            terminate.store(true, Ordering::Relaxed);
        }
        _ => {
            println!("-->ERROR: no such command");
        },
    }
}

fn handle_server_message(message: Message,
                         connection: &mut ChatConnection,
                         terminate: Arc<AtomicBool>) {
    use chat_np1::message::Message::*;
    match message {
        Failure(m) => println!("SERVER ERROR: {}", &m),
        Joined(c) => connection.chat_name = c,
        ChatMessage(u,_,m) => println!("{}: {}", &u, &m),
        PrivateMessage(f,_,m) => println!("Private message from {}: {}", &f, &m),
        Logout(_,_) => connection.chat_name = "".to_owned(),
        ConnectionTermination(_) => {
            terminate.store(true, Ordering::Relaxed);
            println!("Connection with server terminated!!");
        }
        
        m @ _ => eprintln!("{:?}", m),
    }
}

fn print_help() {
    println!("List of available commands:
/help   -- show this message
/list   -- show available chats
/join   -- join a chat
/new    -- create new chat
/leave  -- leave current chat
/online -- list of users in this chat
/kick   -- kick user from chat (when admin)
/quit   -- quit application");   
}

fn main() {
    let username = match std::env::args().nth(1) {
        Some(u) => u,
        None => {eprintln!("NO USERNAME GIVEN"); panic!("no username")}
    };

    if &username == "Server" {
        eprintln!("Can't be named 'Server'");
        panic!("invalid name");
    }

    
    let (listen_snd, listen_rcv) = mpsc::channel();
    let (input_snd, input_rcv) = mpsc::channel();
    
    let terminate = Arc::new(AtomicBool::new(false));
    let input_terminate = terminate.clone();    
    
    let mut connection = ChatConnection::connect(username,
                                             "127.0.0.1:8080",
                                             listen_snd,
                                             terminate.clone());
    
    let input_thread = thread::spawn(move || {
        input_loop(input_snd, input_terminate);
    });
    print_help();

    'main: loop {
        match input_rcv.try_recv() {
            Ok(input) => {
                handle_input(input, &mut connection, terminate.clone());
                    
            },
            Err(_) => (),
        }
        match listen_rcv.try_recv() {
            Ok(message) => handle_server_message(message,
                                                 &mut connection,
                                                 terminate.clone()),
            Err(_) => (),
        }
        let terminated = terminate.load(Ordering::Relaxed);
        if terminated {
            drop(connection);
            break 'main;
        }
    }
    input_thread.join().unwrap();
}
