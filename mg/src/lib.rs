use std::time::{Duration, Instant};
use std::{any::Any, cell::RefCell, fmt, rc::Rc};

type UserData = Rc<RefCell<dyn Any>>;
pub type ConnCallback = fn(&mut MgConn, MgEvent, u32, UserData);

#[derive(Debug)]
pub enum MgEvent {
    EvTimer,
    EvClose,
}

pub enum MgFlags {
    MgCloseNow = 1,
}

pub struct MgConn {
    flags: u32,
    once: bool,
    fire_time: Instant,
    interval: Duration,
    id: u32,
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
        println!("calling close on {}", self.id);
        (self.cb)(self, MgEvent::EvClose, self.id, self.user_data.clone());
    }
}

impl Default for MgMgr {
    fn default() -> Self {
        MgMgr { conns: Vec::new() }
    }
}

impl MgConn {
    pub fn close_now(&mut self) {
        self.flags |= MgFlags::MgCloseNow as u32;
    }
}

fn callback(conn: &mut MgConn, ev: MgEvent) {
    (conn.cb)(conn, ev, conn.id, conn.user_data.clone());
}

impl MgMgr {
    pub fn add_timer(
        &mut self,
        once: bool,
        interval: Duration,
        id: u32,
        cb: ConnCallback,
        user_data: UserData,
    ) {
        self.conns.push(MgConn {
            flags: 0,
            once,
            fire_time: Instant::now() + interval,
            interval,
            id,
            cb,
            user_data,
        });
        println!(
            "Adding timer: id={}, once={}, interval={:?}",
            id, once, interval
        );
    }

    fn handle_conns(&mut self, now: Instant, expired: &mut Vec<usize>) {
        for (index, conn) in self.conns.iter_mut().enumerate() {
            if conn.flags & MgFlags::MgCloseNow as u32 != 0 {
                expired.push(index);
                return;
            }

            if conn.fire_time < now {
                callback(conn, MgEvent::EvTimer);
                conn.fire_time = now + conn.interval;

                if conn.once {
                    expired.push(index);

                    return;
                }
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
