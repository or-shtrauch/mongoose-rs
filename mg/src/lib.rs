use uuid::Uuid;
use std::{
    any::Any, cell::RefCell, fmt, io::{Read, Write}, net::TcpStream, rc::Rc, time::{Duration, Instant}
};

type UserData = Rc<RefCell<dyn Any>>;
pub type ConnCallback = fn(&mut MgConn, MgEvent, MgStatus, UserData);

#[derive(Debug)]
pub enum MgEvent {
    EvConnect,
    EvSend,
    EvRecv,
    EvTimer,
    EvClose,
}

#[derive(Debug)]
pub enum MgStatus {
    Ok,
    TCPConnectionError,
    TCPWriteError,
}

#[derive(PartialEq)]
pub enum MgFlag {
    MgStart,
    MgWaitForConnection,
    MgWaitForData,
    MgConnected,
    MgWriteError,
    MgSent,
    MgCloseNow,
}

enum MgConnType {
    TCP,
    Timer
}

pub struct MgConn {
    host: String,
    port: u16,
    conn_type: MgConnType,
    tcp_stream: Option<TcpStream>,
    flag: MgFlag,
    once: bool,
    fire_time: Instant,
    interval: Duration,
    id: Uuid,
    cb: ConnCallback,
    user_data: UserData,
}

pub struct MgMgr {
    conns: Vec<MgConn>,
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
        (self.cb)(
            self,
            MgEvent::EvClose,
            MgStatus::Ok,
            self.user_data.clone()
        );
    }
}

impl Default for MgMgr {
    fn default() -> Self {
        MgMgr { conns: Vec::new() }
    }
}

impl MgConn {
    fn new(host: &str,
            port: u16,
            conn_type: MgConnType,
            once: bool,
            fire_time: Instant,
            interval: Duration,
            cb: ConnCallback,
            user_data: UserData) -> MgConn {
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

    pub fn close_now(&mut self) {
        self.flag = MgFlag::MgCloseNow;
    }

    pub fn id(&self) -> Uuid {
        self.id
    }

    pub fn send(&mut self, bytes: &[u8]) -> () {
        let mut sent: bool = false;

        if let Some(stream) = self.tcp_stream.as_mut() {
            match stream.write(bytes) {
                Ok(sent_bytes) => {
                    if sent_bytes == bytes.len() {
                        sent = true;
                    }
                },
                _ => {},
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
                Err(_) => return 0
            }
        }

        0
    }
}

fn callback(conn: &mut MgConn, status: MgStatus, ev: MgEvent) {
    (conn.cb)(conn, ev, status, conn.user_data.clone());
}

fn handle_timer_conn(conn: &mut MgConn,
                index: usize,
                now: Instant,
                expired: &mut Vec<usize>) {
    if conn.fire_time < now {
        callback(conn, MgStatus::Ok, MgEvent::EvTimer);
        conn.fire_time = now + conn.interval;

        if conn.once {
            expired.push(index);
        }
    }
}

fn tcp_conn_read_ready(conn: &mut MgConn) -> bool {
    let mut buffer: [u8; 1] = [0; 1];

    if let Some(stream) = conn.tcp_stream.as_mut() {
        match stream.peek(&mut buffer) {
            Ok(size) => {
                println!("size is: {}", size);
                if size > 0 {
                    return true
                }
            },
            _ => {},
        }
    }

    false
}

fn handle_tcp_conn_connect(conn: &mut MgConn,
                        index: usize,
                        expired: &mut Vec<usize>) {
    if let Ok(stream) = TcpStream::connect(
                                format!("{}:{}", conn.host, conn.port)) {
        match stream.set_nonblocking(true) {
            Ok(_) => {
                conn.tcp_stream = Some(stream);
                callback(conn,
                    MgStatus::Ok,
                    MgEvent::EvConnect);
                return;
            },
            _ => {},
        }
    }

    callback(conn,
        MgStatus::TCPConnectionError,
        MgEvent::EvConnect);
        expired.push(index);
}

fn handle_tcp_conn_connected(conn: &mut MgConn,
                        _index: usize,
                        _expired: &mut Vec<usize>) {
    match conn.flag {
        MgFlag::MgWriteError => {
            callback(conn,
            MgStatus::TCPWriteError,
            MgEvent::EvSend)
        },
        MgFlag::MgSent => {
            callback(conn,
                MgStatus::Ok,
                MgEvent::EvSend);
            conn.flag = MgFlag::MgWaitForData;
        },
        MgFlag::MgWaitForData => {
            if tcp_conn_read_ready(conn) {
                callback(conn,
                    MgStatus::Ok,
                    MgEvent::EvRecv);
            }
        },
        _ => {}
    }
}

fn handle_tcp_conn(conn: &mut MgConn,
                index: usize,
                expired: &mut Vec<usize>) {
    match conn.tcp_stream {
        None => handle_tcp_conn_connect(conn, index, expired),
        Some(_) => handle_tcp_conn_connected(conn, index, expired),
    }
}

/* private functions */
impl MgMgr {
    fn handle_conns(&mut self, now: Instant, expired: &mut Vec<usize>) {
        for (index, conn) in self.conns.iter_mut().enumerate() {
            if conn.flag == MgFlag::MgCloseNow {
                expired.push(index);
                continue;
            }

            match conn.conn_type {
                MgConnType::Timer => handle_timer_conn(conn, index, now, expired),
                MgConnType::TCP => handle_tcp_conn(conn, index, expired),
            }
        }
    }

    fn remove_expired_tasks(&mut self, expired: &mut Vec<usize>) {
        for expired_conn_index in expired.iter() {
            /*
             * conn will be automatically dropped,
             * at the end of iteration
             */
            let _ = self.conns.swap_remove(*expired_conn_index);
        }
    }

}

/* public functions */
impl MgMgr {
    pub fn add_timer(
        &mut self,
        once: bool,
        interval: Duration,
        cb: ConnCallback,
        user_data: UserData,
    ) {
        let conn = MgConn::new(
            "",
            0,
            MgConnType::Timer,
            once,
            Instant::now() + interval,
            interval,
            cb,
            user_data
        );

        println!(
            "Adding timer: id={}, once={}, interval={:?}",
            conn.id(), once, interval
        );

        self.conns.push(conn);
    }

    pub fn add_tcp_conn(&mut self, host: &str, port: u16,
                        cb: ConnCallback,
                        user_data: UserData) {
        let conn = MgConn::new(
            host,
            port,
            MgConnType::TCP,
            false,
            Instant::now(),
            Duration::from_secs(0),
            cb,
            user_data
        );
        println!("Adding TCP Conn to {}:{}", conn.host, conn.port);
        self.conns.push(conn);
    }

    pub fn poll(&mut self, duration: Duration) {
        let mut expired: Vec<usize> = Vec::new();
        let mut now: Instant;

        loop {
            now = Instant::now();

            self.handle_conns(now, &mut expired);
            self.remove_expired_tasks(&mut expired);
            expired.clear();

            std::thread::sleep(duration);
        }
    }
}
