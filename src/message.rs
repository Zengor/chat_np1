use std::io::Result;

#[derive(Clone, Debug)]
pub enum Message {
    InitUser(String),
    Login(String,String),
    Joined(String),
    Failure(String),
    ListGroups(String),
    ListUsers(String, String),
    ChatMessage(String,String,String),
    PrivateMessage(String,String,String),
    Logout(String,String),
    NewChat(String,String),
    KickUser(String,String,String),
    ConnectionTermination(String),
    TerminateProgram,
}

impl Message {
    pub fn init_user<S: Into<String>>(username: S) -> Message {
        let username = username.into();
        if username != "Server" {
            Message::InitUser(username)
        } else {
            panic!("can't name user that");
        }
    }

    pub fn login<S: Into<String>>(username: S, chat_name: S) -> Message {
        let username = username.into();
        let chat_name = chat_name.into();
        Message::Login(
            username,
            chat_name
        )
    }

    pub fn chat_message<S: Into<String>>(username: S,
                                         chat_name: S,
                                         contents: S) -> Message {
        let username = username.into();
        let chat_name = chat_name.into();
        let contents = contents.into();
        Message::ChatMessage(
            username,
            chat_name,
            contents,
        )
    }

    pub fn private_message<S: Into<String>>(from_user: S,
                                            to_user: S,
                                            contents: S) -> Message {
        let from_user = from_user.into();
        let to_user = to_user.into();
        let contents = contents.into();
        Message::PrivateMessage (
            from_user,
            to_user,
            contents,
        )
    }

    pub fn logout<S: Into<String>>(username: S, chat_name: S) -> Message {
        let username = username.into();
        let chat_name = chat_name.into();
        Message::Logout(username, chat_name)
    }
    
    pub fn new_chat<S: Into<String>>(username: S, chat_name: S) -> Message {
        let username = username.into();
        let chat_name = chat_name.into();
        Message::NewChat(username, chat_name)
    }
    pub fn kick_user<S: Into<String>>(from_user: S,
                                      chat_name: S,
                                      target_user: S) -> Message {
        let from_user = from_user.into();
        let chat_name = chat_name.into();
        let target_user = target_user.into();
        Message::KickUser (
            from_user,
            chat_name,
            target_user,
        )
    }
    pub fn termination<S: Into<String>>(username: S) -> Message {        
        Message::ConnectionTermination(username.into())
    }

    pub fn failure<S: Into<String>>(contents: S) -> Message {
        let contents = contents.into();
        Message::Failure(contents)
    }

    pub fn into_bytes(&self) -> Vec<u8> {
        use message::Message::*;
        let mut buffer: Vec<u8> = Vec::new();        
        buffer.push(match *self {
            InitUser(_) => 0x00,
            Login(_,_) => 0x01,
            Joined(_) => 0x02,
            Failure(_) => 0x03,
            ListGroups(_) => 0x04,
            ListUsers(_,_) => 0x05,
            ChatMessage(_,_,_) => 0x06,
            PrivateMessage(_,_,_) => 0x07,
            Logout(_,_) => 0x08,
            NewChat(_,_) => 0x09,
            KickUser(_,_,_) => 0x0B,
            ConnectionTermination(_) => 0x0C,
            TerminateProgram => 0x0D,
        });
        match *self {
            InitUser(ref s)   | Joined(ref s) | Failure(ref s) |
            ListGroups(ref s) | ConnectionTermination(ref s)
                => buffer.extend_from_slice(s.as_bytes()),
            Login(ref a, ref b)  | ListUsers(ref a, ref b) |
            Logout(ref a, ref b) | NewChat(ref a, ref b)   => {
                let a = a.as_bytes();
                buffer.push(a.len() as u8);
                buffer.extend_from_slice(a);
                let b = b.as_bytes();
                buffer.push(b.len() as u8);
                buffer.extend_from_slice(b);
            },
            ChatMessage(ref a, ref b, ref c) |
            PrivateMessage(ref a, ref b, ref c) |
            KickUser(ref a, ref b, ref c) => {
                let a = a.as_bytes();
                buffer.push(a.len() as u8);
                buffer.extend_from_slice(a);
                let b = b.as_bytes();
                buffer.push(b.len() as u8);
                buffer.extend_from_slice(b);
                let c = c.as_bytes();
                buffer.push(c.len() as u8);
                buffer.extend_from_slice(c);
            }
            TerminateProgram => (),
        }
        buffer.push(b'\n');
        buffer
    }

    pub fn from_bytes(bytes: &[u8]) -> Result<Message> {
        if bytes.len() == 0 {
            return Err(::std::io::Error::from(::std::io::ErrorKind::InvalidData));
        };
        use message::Message::*;
        let(a,b,c) = Message::get_parameters(bytes);
        //println!("{:?} {:?} {:?}", a,b,c,);
        let message_type = bytes[0];
        match message_type {
            0x00 => Ok(InitUser(a.unwrap())),
            0x01 => Ok(Login(a.unwrap(),b.unwrap())),
            0x02 => Ok(Joined(a.unwrap())),
            0x03 => Ok(Failure(a.unwrap())),
            0x04 => Ok(ListGroups(a.unwrap())),
            0x05 => Ok(ListUsers(a.unwrap(),b.unwrap())),
            0x06 => Ok(ChatMessage(a.unwrap(),b.unwrap(),c.unwrap())),
            0x07 => Ok(PrivateMessage(a.unwrap(),b.unwrap(),c.unwrap())),
            0x08 => Ok(Logout(a.unwrap(),b.unwrap())),
            0x09 => Ok(NewChat(a.unwrap(),b.unwrap())),
            0x0B => Ok(KickUser(a.unwrap(),b.unwrap(),c.unwrap())),
            0x0C => Ok(ConnectionTermination(a.unwrap())),
            0x0D => Ok(TerminateProgram),
            _ => Err(::std::io::Error::from(::std::io::ErrorKind::InvalidData)),
        }
    }

    fn get_parameters(bytes: &[u8])
                      -> (Option<String>,Option<String>,Option<String>) {
        let message_type = bytes[0];
        match message_type {
            0x00 | 0x02 | 0x03 | 0x04 | 0x0C => {
                let len = bytes[1..].len();
                (Some(String::from_utf8(bytes[1..len].to_vec()).unwrap()),
                 None,
                 None)
            },
            0x01 | 0x05 | 0x08 | 0x09 => {
                let a_len = bytes[1]  as usize;
                let pos = 2+a_len;
                let a = &bytes[2..pos];
                let b_len = bytes[pos]  as usize;
                let pos = pos + 1;
                let b = &bytes[pos..pos+b_len];
                (Some(String::from_utf8(a.to_vec()).unwrap()),
                 Some(String::from_utf8(b.to_vec()).unwrap()),
                 None)
            },
            0x06 | 0x07 | 0x0B => {
                let a_len = bytes[1] as usize;
                let pos = 2+a_len;
                let a = &bytes[2..pos];
                let b_len = bytes[pos] as usize;
                let pos = pos + 1;
                let b = &bytes[pos..pos+b_len];
                let pos = pos + b_len;
                let c_len = bytes[pos] as usize;
                let pos = pos + 1;
                let c = &bytes[pos..pos+c_len];
                (Some(String::from_utf8(a.to_vec()).unwrap()),
                 Some(String::from_utf8(b.to_vec()).unwrap()),
                 Some(String::from_utf8(c.to_vec()).unwrap()))               
            },
            _ => (None, None, None)
        }
    }
}
