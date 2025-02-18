use log::debug;
use mg::{MgConn, MgEvent, MgMgr, MgStatus};
use std::{any::Any, cell::RefCell, rc::Rc, time::Duration};

fn on_timer(
    conn: &mut MgConn,
    ev: MgEvent,
    status: MgStatus,
    user_data: Rc<RefCell<dyn Any>>,
) {
    if let Ok(mut user_data) = user_data.try_borrow_mut() {
        if let Some(user_data) = user_data.downcast_mut::<u32>() {
            debug!(
                "conn: {}, on_timer: {:?}, status: {:?}, user_data: {}",
                conn, ev, status, user_data
            );
        }
    }
}

fn on_tcp(
    conn: &mut MgConn,
    ev: MgEvent,
    status: MgStatus,
    _user_data: Rc<RefCell<dyn Any>>,
) {
    debug!("conn: {}, tcp: {:?}, status: {:?}", conn, ev, status);
    match ev {
        MgEvent::EvSend => debug!("Sent: {:?}", status),
        MgEvent::EvClose => debug!("tcp close"),
        MgEvent::EvConnect => {
            debug!("sending hello");
            conn.send("hello".as_bytes());
        }
        MgEvent::EvRecv => {
            let mut buffer: String = String::new();
            conn.read(&mut buffer);
            debug!("read from conn: '{}'", buffer);
            conn.send("bytes".as_bytes());
        }
        _ => {}
    }
}

fn main() {
    let mut mgr: MgMgr = MgMgr::default();
    let user_data1 = Rc::new(RefCell::new(1u32));
    let user_data2 = Rc::new(RefCell::new(2u32));
    let user_data5 = Rc::new(RefCell::new(5u32));

    let user_data_tcp = Rc::new(RefCell::new(false));

    mgr.add_timer(false, Duration::from_secs(1), on_timer, user_data1);
    mgr.add_timer(false, Duration::from_secs(2), on_timer, user_data2);
    mgr.add_timer(true, Duration::from_secs(5), on_timer, user_data5);

    mgr.add_tcp_conn("localhost", 1234, on_tcp, user_data_tcp);

    mgr.poll(Duration::from_millis(50));
}
