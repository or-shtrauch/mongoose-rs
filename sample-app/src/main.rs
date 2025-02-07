use mg::{MgConn, MgEvent, MgMgr};
use std::{cell::RefCell, rc::Rc, any::Any, time::Duration};

fn on_timer(conn: &mut MgConn, ev: MgEvent, id: u32, user_data: Rc<RefCell<dyn Any>>) {
    if let Ok(mut user_data) = user_data.try_borrow_mut() {
        if let Some(user_data) = user_data.downcast_mut::<u32>() {
            println!("conn: {}, on_timer: {:?}, id: {}, user_data: {}", conn, ev, id, user_data);

            if id == 1 {
                *user_data += 1;
            }

            if id == 2 {
                conn.close_now();
            }
        }
    }
}

fn main() {
    let mut mgr: MgMgr = MgMgr::default();
    let user_data1 = Rc::new(RefCell::new(1u32));
    let user_data2 = Rc::new(RefCell::new(2u32));
    let user_data5 = Rc::new(RefCell::new(5u32));

    mgr.add_timer(false, Duration::from_secs(1), 1, on_timer, user_data1);
    mgr.add_timer(false, Duration::from_secs(2), 2, on_timer, user_data2);
    mgr.add_timer(true, Duration::from_secs(5), 5, on_timer, user_data5);

    mgr.poll(Duration::from_millis(50));
}
