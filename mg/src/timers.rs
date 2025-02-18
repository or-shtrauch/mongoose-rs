use crate::conn::MgConn;
use crate::ev::{MgEvent, MgStatus};
use std::time::Instant;

impl MgConn {
    pub fn handle_timer_conn(
        &mut self,
        index: usize,
        now: Instant,
        expired: &mut Vec<usize>,
    ) {
        if self.fire_time < now {
            self.callback(MgStatus::Ok, MgEvent::EvTimer);
            self.fire_time = now + self.interval;

            if self.once {
                expired.push(index);
            }
        }
    }
}
