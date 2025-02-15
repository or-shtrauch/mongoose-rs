use uuid::Uuid;
use log::{debug, error};
use std::{
    any::Any,
    cell::RefCell,
    fmt, io::{Read, Write},
    net::TcpStream,
    rc::Rc,
    time::{Duration, Instant}
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

#[derive(PartialEq, Debug)]
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
        env_logger::init();
        MgMgr {
            conns: Vec::new()
        }
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

    fn callback(&mut self, status: MgStatus, ev: MgEvent) {
        (self.cb)(self, ev, status, self.user_data.clone());
    }

    fn handle_timer_conn(&mut self,
                       index: usize,
                       now: Instant,
                       expired: &mut Vec<usize>) {
        if self.fire_time < now {
            self.callback(MgStatus::Ok, MgEvent::EvTimer);
            self.fire_time = now + self.interval;

            if self.once {
                expired.push(index);
            }
        }
    }

    fn handle_tcp_conn_connect(&mut self,
                            index: usize,
                            expired: &mut Vec<usize>) {
        if let Ok(stream) = TcpStream::connect(
                        format!("{}:{}", self.host, self.port)) {
            match stream.set_nonblocking(true) {
                Ok(_) => {
                    self.tcp_stream = Some(stream);
                    self.callback(MgStatus::Ok, MgEvent::EvConnect);
                    return;
                },
                _ => {},
            }
        }

        self.callback(MgStatus::TCPConnectionError,
        MgEvent::EvConnect);
        expired.push(index);
    }

    fn handle_tcp_conn_connected(&mut self,
                            _index: usize,
                            _expired: &mut Vec<usize>) {
        match self.flag {
            MgFlag::MgWriteError => {
                self.callback(MgStatus::TCPWriteError, MgEvent::EvSend);
            },
            MgFlag::MgSent => {
                self.callback(MgStatus::Ok, MgEvent::EvSend);
                self.flag = MgFlag::MgWaitForData;
            },
            MgFlag::MgWaitForData => {
                if self.read_ready() {
                    self.callback(MgStatus::Ok, MgEvent::EvRecv);
                }
            },
            _ => {}
        }
    }

    fn handle_tcp_conn(&mut self,
                    index: usize,
                    expired: &mut Vec<usize>) {
        debug!("Handling TCP Conn: {:?} - {:?}", self.id(), self.flag);
        match self.tcp_stream {
            None => self.handle_tcp_conn_connect(index, expired),
            Some(_) => self.handle_tcp_conn_connected(index, expired),
        }
    }

    fn read_ready(&mut self) -> bool {
        let mut buffer = [0u8; 0];

        if let Some(stream) = self.tcp_stream.as_mut() {
            match stream.peek(&mut buffer) {
                Ok(_) => {
                    return true;
                },
                _ => {
                    error!("Error reading from stream");
                },
            }
        }

        return false
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

/* private functions */
impl MgMgr {
    fn handle_conns(&mut self, now: Instant, expired: &mut Vec<usize>) {
        for (index, conn) in self.conns.iter_mut().enumerate() {
            if conn.flag == MgFlag::MgCloseNow {
                expired.push(index);
                continue;
            }

            match conn.conn_type {
                MgConnType::Timer => conn.handle_timer_conn(index, now, expired),
                MgConnType::TCP => conn.handle_tcp_conn(index, expired),
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

        debug!(
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
        debug!("Adding TCP Conn to {}:{}", conn.host, conn.port);
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
