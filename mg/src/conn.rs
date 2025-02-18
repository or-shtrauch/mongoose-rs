use crate::ev::{MgEvent, MgFlag, MgStatus};
use log::{debug, error};
use std::{
    any::Any,
    cell::RefCell,
    fmt,
    io::{Read, Write},
    net::TcpStream,
    rc::Rc,
    time::{Duration, Instant},
};
use uuid::Uuid;

pub type UserData = Rc<RefCell<dyn Any>>;
pub type ConnCallback = fn(&mut MgConn, MgEvent, MgStatus, UserData);

pub enum MgConnType {
    TCP,
    Timer,
}

pub struct MgConn {
    pub host: String,
    pub port: u16,
    pub conn_type: MgConnType,
    pub fire_time: Instant,
    pub interval: Duration,
    pub once: bool,
    tcp_stream: Option<TcpStream>,
    flag: MgFlag,
    id: Uuid,
    cb: ConnCallback,
    user_data: UserData,
}

impl fmt::Display for MgConn {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{{ id: {}, interval: {} }}",
            self.id,
            self.interval.as_secs_f64()
        )
    }
}

impl Drop for MgConn {
    fn drop(&mut self) {
        (self.cb)(self, MgEvent::EvClose, MgStatus::Ok, self.user_data.clone());
    }
}

impl MgConn {
    pub fn new(
        host: &str,
        port: u16,
        conn_type: MgConnType,
        once: bool,
        fire_time: Instant,
        interval: Duration,
        cb: ConnCallback,
        user_data: UserData,
    ) -> MgConn {
        MgConn {
            host: host.to_owned(),
            port,
            conn_type,
            tcp_stream: None,
            flag: MgFlag::MgStart,
            once,
            fire_time,
            interval,
            id: Uuid::new_v4(),
            cb,
            user_data,
        }
    }

    pub fn id(&self) -> Uuid {
        self.id
    }

    pub fn flag(&self) -> MgFlag {
        self.flag.clone()
    }

    pub fn send(&mut self, bytes: &[u8]) {
        let mut sent = false;
        if let Some(stream) = self.tcp_stream.as_mut() {
            match stream.write(bytes) {
                Ok(sent_bytes) if sent_bytes == bytes.len() => sent = true,
                _ => {}
            }
        }

        if sent {
            self.flag = MgFlag::MgSent;
        } else {
            self.flag = MgFlag::MgWriteError;
        }
    }

    pub fn read(&mut self, buffer: &mut String) -> usize {
        if let Some(stream) = self.tcp_stream.as_mut() {
            match stream.read_to_string(buffer) {
                Ok(size) => return size,
                Err(_) => return 0,
            }
        }
        0
    }

    fn read_ready(&mut self) -> bool {
        let mut buffer = [0u8; 0];
        if let Some(stream) = self.tcp_stream.as_mut() {
            match stream.peek(&mut buffer) {
                Ok(_) => true,
                _ => {
                    error!("Error reading from stream");
                    false
                }
            }
        } else {
            false
        }
    }

    pub fn close_now(&mut self) {
        self.flag = MgFlag::MgCloseNow;
    }

    // Private methods for handling events
    pub fn callback(&mut self, status: MgStatus, ev: MgEvent) {
        (self.cb)(self, ev, status, self.user_data.clone());
    }

    pub fn handle_tcp_conn(&mut self, index: usize, expired: &mut Vec<usize>) {
        debug!("Handling TCP Conn: {:?} - {:?}", self.id(), self.flag);
        match self.tcp_stream {
            None => self.handle_tcp_conn_connect(index, expired),
            Some(_) => self.handle_tcp_conn_connected(),
        }
    }

    fn handle_tcp_conn_connect(
        &mut self,
        index: usize,
        expired: &mut Vec<usize>,
    ) {
        if let Ok(stream) =
            TcpStream::connect(format!("{}:{}", self.host, self.port))
        {
            if stream.set_nonblocking(true).is_ok() {
                self.tcp_stream = Some(stream);
                self.callback(MgStatus::Ok, MgEvent::EvConnect);
                return;
            }
        }
        self.callback(MgStatus::TCPConnectionError, MgEvent::EvConnect);
        expired.push(index);
    }

    fn handle_tcp_conn_connected(&mut self) {
        match self.flag {
            MgFlag::MgWriteError => {
                self.callback(MgStatus::TCPWriteError, MgEvent::EvSend);
            }
            MgFlag::MgSent => {
                self.callback(MgStatus::Ok, MgEvent::EvSend);
                self.flag = MgFlag::MgWaitForData;
            }
            MgFlag::MgWaitForData => {
                if self.read_ready() {
                    self.callback(MgStatus::Ok, MgEvent::EvRecv);
                }
            }
            _ => {}
        }
    }
}
