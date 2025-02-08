use mg::{MgConn, MgEvent, MgMgr, MgStatus};
use std::{any::Any, cell::RefCell, rc::Rc, time::Duration};

// fn on_timer(conn: &mut MgConn,
//             ev: MgEvent,
//             status: MgStatus,
//             user_data: Rc<RefCell<dyn Any>>) {
//     if let Ok(mut user_data) = user_data.try_borrow_mut() {
//         if let Some(user_data) = user_data.downcast_mut::<u32>() {
//             println!("conn: {}, on_timer: {:?}, status: {:?}, id: {}, user_data: {}",
//                 conn, ev, status, conn.id(), user_data);

//             if conn.id() == 1 {
//                 *user_data += 1;
//             }

//             if conn.id() == 2 {
//                 conn.close_now();
//             }
//         }
//     }
// }

fn on_tcp(conn: &mut MgConn,
        ev: MgEvent,
        status: MgStatus,
        _user_data: Rc<RefCell<dyn Any>>) {
    println!("conn: {}, tcp: {:?}, status: {:?}", conn, ev, status);
    match ev {
        MgEvent::EvSend => println!("Sent: {:?}", status),
        MgEvent::EvClose => println!("tcp close"),
        MgEvent::EvConnect => {
            println!("sending hello");
            conn.send("hello".as_bytes());
        },
        MgEvent::EvRecv => {
            let mut buffer: String = String::new();
            if conn.read(&mut buffer) > 0 {
                println!("read from conn: '{}'", buffer);
                conn.send("bytes".as_bytes());
            }
        },
        _ => {}
    }
}

fn main() {
    let mut mgr: MgMgr = MgMgr::default();
    // let user_data1 = Rc::new(RefCell::new(1u32));
    // let user_data2 = Rc::new(RefCell::new(2u32));
    // let user_data5 = Rc::new(RefCell::new(5u32));

    let user_data_tcp = Rc::new(RefCell::new(false));

    // mgr.add_timer(false, Duration::from_secs(1), on_timer, user_data1);
    // mgr.add_timer(false, Duration::from_secs(2), on_timer, user_data2);
    // mgr.add_timer(true, Duration::from_secs(5), on_timer, user_data5);

    mgr.add_tcp_conn("localhost", 1234, on_tcp, user_data_tcp);

    mgr.poll(Duration::from_millis(50));
}
