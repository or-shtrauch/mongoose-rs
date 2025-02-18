use crate::conn::{ConnCallback, MgConn, MgConnType, UserData};
use crate::ev::MgFlag;
use log::debug;
use std::time::{Duration, Instant};

pub struct MgMgr {
    conns: Vec<MgConn>,
}

impl Default for MgMgr {
    fn default() -> Self {
        env_logger::init();
        MgMgr { conns: Vec::new() }
    }
}

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
            user_data,
        );
        debug!(
            "Adding timer: id={}, once={}, interval={:?}",
            conn.id(),
            once,
            interval
        );
        self.conns.push(conn);
    }

    pub fn add_tcp_conn(
        &mut self,
        host: &str,
        port: u16,
        cb: ConnCallback,
        user_data: UserData,
    ) {
        let conn = MgConn::new(
            host,
            port,
            MgConnType::TCP,
            false,
            Instant::now(),
            Duration::from_secs(0),
            cb,
            user_data,
        );
        debug!("Adding TCP Conn to {}:{}", conn.host, conn.port);
        self.conns.push(conn);
    }

    pub fn poll(&mut self, duration: Duration) {
        let mut expired: Vec<usize> = Vec::new();
        loop {
            let now = Instant::now();
            self.handle_conns(now, &mut expired);
            self.remove_expired_tasks(&mut expired);
            expired.clear();
            std::thread::sleep(duration);
        }
    }

    fn handle_conns(&mut self, now: Instant, expired: &mut Vec<usize>) {
        for (index, conn) in self.conns.iter_mut().enumerate() {
            if conn.flag() == MgFlag::MgCloseNow {
                expired.push(index);
                continue;
            }

            match conn.conn_type {
                MgConnType::Timer => {
                    conn.handle_timer_conn(index, now, expired)
                }
                MgConnType::TCP => conn.handle_tcp_conn(index, expired),
            }
        }
    }

    fn remove_expired_tasks(&mut self, expired: &mut Vec<usize>) {
        for expired_conn_index in expired.iter() {
            self.conns.swap_remove(*expired_conn_index);
        }
    }
}
