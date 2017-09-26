use std::net::{ToSocketAddrs, TcpListener, TcpStream};
use std::sync::mpsc;
use std::thread;
use std::sync::{Arc, Mutex};
use std::collections::HashMap;
use std::io::{BufReader, BufRead, Write};
use message::Message;

type Sender = mpsc::Sender<Message>;
type Receiver = mpsc::Receiver<Message>;
type Connections = Arc<Mutex<HashMap<String, UserConnection>>>;
type Groups = Arc<Mutex<HashMap<String,Vec<String>>>>;

lazy_static! {
    static ref CONNECTIONS: Connections = Default::default();
    static ref GROUPS: Groups = Default::default();
}

pub fn start_server<A: ToSocketAddrs>(addr: A) {
    let listener = TcpListener::bind(addr)
        .expect("Falha ao criar listener nesse endereço");
    let (comm_snd, comm_rcv) = mpsc::channel();
    GROUPS.lock().unwrap().insert("Chat1".into(), Vec::new());
    let _ = thread::spawn(move || communicate_messages(comm_rcv));
    listen_messages(listener, comm_snd);
}

fn listen_messages(listener: TcpListener,
                   comm_channel: Sender) {
    for stream in listener.incoming() {
        let stream = stream.expect("Failed getting stream in listen_messages");
        let mut reader = BufReader::new(stream.try_clone().unwrap());
        let mut buf = Vec::new();
        loop {
            let mut read_bytes = 0;
            match reader.read_until(b'\n', &mut buf) {
                Ok(b) => read_bytes = b,
                Err(e) => eprintln!("{}", e),
            };
            if read_bytes > 0 {
                match Message::from_bytes(&buf) {
                    Ok(Message::InitUser(username)) => new_client(username,
                                                       stream,
                                                       comm_channel.clone()),
                    _ => eprintln!("Somehow got non-init message from new connection"),
                }
                break;
            }
        }
    }
}

fn new_client(username: String,
              stream: TcpStream,
              comm_channel: Sender) {
    println!("New client");
    let user = UserConnection::new(username.clone(), stream, comm_channel);
    //send response
    let contents = "Following groups available, type /join [GROUP] to join".to_owned();
    
    CONNECTIONS.lock().unwrap()
        .insert(username.clone(), user);
    
    send_chat_message_to_user(username.clone(), contents);
    let contents = groups_list();
    send_chat_message_to_user(username.clone(), contents);
}

///Retorna uma String com todos os grupos disponíveis separados por " | "
#[inline]
fn groups_list() -> String {
    GROUPS.lock().unwrap().keys()
        .fold(String::new(),
              |acc, ref x| format!("{} | {}", x, acc))
}

#[inline]
fn users_in_group(group_name: String) -> String {
    GROUPS.lock().unwrap().get(&group_name).unwrap().iter()
        .fold(String::from("Online: "),
              |acc, ref x| format!("{} {},", acc, x))
}

fn list_users(message: Message) {
    if let Message::ListUsers(username, group_name) = message {
        let online = users_in_group(group_name);
        send_chat_message_to_user(username, online);
    }
}

fn communicate_messages(receiver: Receiver) {
    loop {
        let message = receiver.recv().unwrap();
        println!("{:?}", message);
        use self::Message::*;
        match message {
            ListGroups(u) => send_chat_message_to_user(u, groups_list()),
            m @ ListUsers(_,_) => list_users(m),
            m @ Login(_,_) => login(m),
            m @ ChatMessage(_,_,_) => chat_message(m),
            m @ PrivateMessage(_,_,_) => private_message(m),
            m @ Logout(_,_) => logout(m),
            m @ NewChat(_,_) => create_group(m),
            m @ KickUser(_,_,_) => kick_user(m),
            ConnectionTermination(u) => terminate_connection(u),
            _ => (),
        }
    }    
}


fn login(message: Message) {
    if let Message::Login(username, group_name) = message {
        let mut groups = GROUPS.lock().unwrap();
        let mut group = match groups.get_mut(&group_name) {
            Some(v) => v,
            None => {
                failure_message(username, "No such chat");
                return;
            }
        };
        if !group.contains(&username) {
            CONNECTIONS.lock().expect("login con lock").get_mut(&username).expect("login con get")
                .send_to_user(Message::Joined(group_name.clone()));
            group.push(username);
        } else {
            failure_message(username, "Already in this chat");
        }
    }
}

fn chat_message(message: Message) {
    let repass = message.clone();
    if let Message::ChatMessage(username, chat_name, _) = message {
        let mut groups = GROUPS.lock().unwrap();
        let mut group = match groups.get_mut(&chat_name) {
            Some(v) => v,
            None => {
                failure_message(username, "Must join a chat");
                return;
            }
        };
        let mut connections = CONNECTIONS.lock().unwrap();
        for user in group.iter_mut() {
            if user != &username {
                connections.get_mut(user).unwrap().send_to_user(repass.clone());
            }
        }
    }    
}

fn private_message(message: Message) {
    let repass = message.clone();
    if let Message::PrivateMessage(from, to, _) = message {
        let mut connections = CONNECTIONS.lock().unwrap();
        match connections.get_mut(&to) {
            Some(u) => u.send_to_user(repass),
            None => failure_message(from, "No such user"),
        }
    }    
}

fn logout(message: Message) {
    if let Message::Logout(username, group_name) = message {
        GROUPS.lock().unwrap().get_mut(&group_name).unwrap()
            .retain(|x| x != &username);
        CONNECTIONS.lock().unwrap().get_mut(&username).unwrap()
            .send_to_user(Message::Logout(username,group_name));
    }
}

fn create_group(message: Message) {
    if let Message::NewChat(username, chat_name) = message {
        let mut groups = GROUPS.lock().unwrap();
        if groups.contains_key(&chat_name) {
            failure_message(username, "Chat already exists".to_owned());
            return;
        } else {
            for group in groups.values_mut() {
                group.retain(|x| x != &username);
            }
            groups.insert(chat_name.clone(),Vec::new());
            groups.get_mut(&chat_name).unwrap().push(username.clone());
            CONNECTIONS.lock().unwrap().get_mut(&username).unwrap()
                .send_to_user(Message::logout("",""));
            CONNECTIONS.lock().unwrap().get_mut(&username).unwrap()
                .send_to_user(Message::Joined(chat_name));
            
            send_chat_message_to_user(username, "Created new group and moved to it!");
        }
    }
}

fn kick_user(message: Message) {
    if let Message::KickUser(from, chat_name, target) = message {
        let mut groups = GROUPS.lock().unwrap();
        let mut group = groups.get_mut(&chat_name).unwrap();
        if group[0] != from {
            failure_message(from, "Not admin in this chat");
            return;
        }
        match CONNECTIONS.lock().unwrap().get_mut(&target) {
            Some(u) => {
                group.retain(|x| x != &target);
                u.send_to_user(Message::logout("",""));
                u.send_to_user(Message::chat_message("Server",
                                                     "(SERVER)",
                                                     "Kicked from chat!"));
            },
            None => failure_message(from, "No such user in this chat")
        }
    }
}

fn terminate_connection(username: String) {
    let connection = CONNECTIONS.lock().expect("terminate connections lock").remove(&username)
        .expect("terminate connections remove");
    drop(connection);
    let mut groups = GROUPS.lock().expect("terminate groups lock");
    for group in groups.values_mut() {
        group.retain(|x| x != &username);
    }
}

fn send_chat_message_to_user<S: Into<String>>(username: String, contents: S) {
    CONNECTIONS.lock().expect("send chat lock").get_mut(&username).expect("send chat get")
                .send_to_user(
                    Message::chat_message("Server".to_owned(),
                                          "(SERVER)".to_owned(),
                                          contents.into()))
}


fn failure_message<S: Into<String>>(username: String, contents: S) {
    let m = Message::failure(contents);
    CONNECTIONS.lock().expect("failure connections lock")
        .get_mut(&username).expect("get mut failure").send_to_user(m);
}

#[derive(Debug)]
struct UserConnection {
    username: String,
    socket: TcpStream,
    thread: thread::JoinHandle<()>,
    thread_comm: mpsc::Sender<()>,
}

impl UserConnection {
    fn new(username: String,
           socket: TcpStream,
           callback_channel: Sender) -> UserConnection {
        let stream_clone = socket.try_clone()
            .expect("Failed cloning tcpstream");
        let (thread, thread_comm) = Self::start_listening(callback_channel,
                                                          stream_clone);
        UserConnection {
            username,
            socket,
            thread,
            thread_comm,
        }
    }

    fn start_listening(sender: Sender, socket: TcpStream)
                       -> (thread::JoinHandle<()>, mpsc::Sender<()>) {
        let sender = sender.clone();
        let (snd, rcv) = mpsc::channel();
        let snd_clone = snd.clone();
        (thread::spawn(move || {            
            let mut buf = Vec::new();
            let mut reader = BufReader::new(socket);
            let mut read_bytes = 0;
            'listen: loop {
                buf.clear();
                match reader.read_until(b'\n', &mut buf) {
                    Ok(b) => read_bytes+=b,
                    Err(_) => {
                        snd.send(()).expect("listen thread comm send");
                    },
                };
                if read_bytes > 0 {
                    let message = Message::from_bytes(&buf)
                        .expect("Failed parsing message");
                    sender.send(message)
                        .expect("Failed sending from UserConnection thread");
                };
                match rcv.try_recv() {
                    Ok(()) => break 'listen,
                    Err(_) => (),
                };
            }
        }), snd_clone)
    }

    fn send_to_user(&mut self, m: Message) {
        self.socket.write(&m.into_bytes()).expect("send write");
        self.socket.flush().expect("send flush");
    }
}

impl Drop for UserConnection {
    fn drop(&mut self) {
        let username = self.username.clone();
        self.send_to_user(Message::termination(username));
        self.thread_comm.send(()).expect("drop thread comm send");
    }
}

// struct Group {
//     name: String,
//     users: Vec<&UserConnection>,
// }

// impl Group {
//     fn new<S: Into<String>>(name: S) -> Group {
//         let name = name.into();
        
//     }
// }
