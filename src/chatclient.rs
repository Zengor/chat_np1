use std::io::{BufReader, BufRead, Write};
use std::net::{ToSocketAddrs, TcpStream};
use std::sync::mpsc;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread;
use message::Message;

#[derive(Debug)]
pub struct ChatConnection {
    username: String,
    pub chat_name: String,
    socket: TcpStream,
    thread: thread::JoinHandle<()>
}

impl ChatConnection {
    pub fn connect<S: Into<String>,
                   A: ToSocketAddrs>(username: S,
                                     addr: A,
                                     callback_channel: mpsc::Sender<Message>,
                                     terminate: Arc<AtomicBool>)     -> ChatConnection {
        let mut socket = TcpStream::connect(addr).expect("Failed connecting to chat");
        let username = username.into();
        socket.write(&Message::init_user(username.clone()).into_bytes())
            .unwrap();
        let read_socket = socket.try_clone().unwrap();
        let thread = Self::start_listening(callback_channel,read_socket, terminate);
        ChatConnection {
            username,
            chat_name: "".to_owned(),
            socket,
            thread,
        }        
    }

    pub fn start_listening(sender: mpsc::Sender<Message>,
                           socket:TcpStream,
                           terminate: Arc<AtomicBool>) -> thread::JoinHandle<()> {
        let sender = sender.clone();
        thread::spawn(move || {            
            let mut buf = Vec::new();
            let mut reader = BufReader::new(socket);
            let mut read_bytes = 0;
            'listen: loop {
                buf.clear();
                match reader.read_until(b'\n', &mut buf) {
                    Ok(b) => read_bytes+=b,
                    Err(e) => {
                        eprintln!("{}", e);
                        terminate.store(true, Ordering::Relaxed)
                    },
                };
                if read_bytes > 0 {
                    let message = Message::from_bytes(&buf)
                        .expect("Failed parsing message");
                    sender.send(message)
                        .expect("Failed sending message through channel");
                };
                let terminated = terminate.load(Ordering::Relaxed);
                if terminated {               
                    break 'listen;
                }
            }
        })
    }
      
    pub fn send_private_message(&mut self, contents: String) {
        let to = contents.split(' ').nth(0).unwrap()[1..].to_owned();
        let message = Message::private_message(self.username.clone(),
                                               to,
                                               contents);
        self.send_to_server(message);
    }

    pub fn send_public_message(&mut self, contents: String) {
        if self.chat_name == "" {
            println!("Must join chat to send messages");
            return;
        }
        let message = Message::chat_message(self.username.clone(),
                                           self.chat_name.clone(),
                                           contents);
        self.send_to_server(message);
    }

    pub fn request_groups(&mut self) {
        let message = Message::ListGroups(self.username.clone());
        self.send_to_server(message);
    }

    pub fn request_clients(&mut self) {
        if self.chat_name == "" {
            println!("Must join chat to see who's online");
            return;
        }
        let message = Message::ListUsers(self.username.clone(),
                                         self.chat_name.clone());
        self.send_to_server(message);
    }

    pub fn create_chat(&mut self, chat_name: String) {
        let message = Message::NewChat(self.username.clone(),
                                       chat_name);
        self.send_to_server(message);
    }

    pub fn join_chat(&mut self, chat_name: String) {
        if &self.chat_name != "" {
            self.leave_chat();
        }
        let message = Message::Login(self.username.clone(), chat_name);
        self.send_to_server(message);
    }

    pub fn leave_chat(&mut self) {
        if &self.chat_name == "" {
            println!("Must be in chat to leave");
            return
        }
        let message = Message::logout(self.username.clone(), self.chat_name.clone());
        self.send_to_server(message);
    }

    pub fn kick(&mut self, target: String) {
        let message = Message::kick_user(self.username.clone(),
                                         self.chat_name.clone(),
                                         target);
        self.send_to_server(message);
    }

    fn send_to_server(&mut self, m: Message) {
        self.socket.write(&m.into_bytes()).unwrap();
        self.socket.flush().unwrap();
    }
}

impl Drop for ChatConnection {
    fn drop(&mut self) {
        let username = self.username.clone();
        self.send_to_server(Message::ConnectionTermination(username));
    }
}
