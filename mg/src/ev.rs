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

#[derive(PartialEq, Debug, Clone, Copy)]
pub enum MgFlag {
    MgStart,
    MgWaitForConnection,
    MgWaitForData,
    MgConnected,
    MgWriteError,
    MgSent,
    MgCloseNow,
}
