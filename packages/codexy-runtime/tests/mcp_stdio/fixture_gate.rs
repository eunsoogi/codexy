use super::*;
use std::net::{TcpListener, TcpStream};
use std::sync::mpsc::{self, Sender};
use std::thread::{self, JoinHandle};

const STDERR_PENDING: &[u8] = b"stderr-buffer-pending";
const SHUTDOWN_OBSERVED: &[u8] = b"shutdown-observed";

pub(super) struct StderrPublicationGate {
    reader_address: String,
    shutdown_address: String,
    release: Sender<()>,
    reader: Option<JoinHandle<Result<(), String>>>,
    shutdown: Option<JoinHandle<Result<(), String>>>,
}

impl StderrPublicationGate {
    pub(super) fn new() -> Result<Self, Box<dyn std::error::Error>> {
        let reader_listener = TcpListener::bind("127.0.0.1:0")?;
        let shutdown_listener = TcpListener::bind("127.0.0.1:0")?;
        let reader_address = reader_listener.local_addr()?.to_string();
        let shutdown_address = shutdown_listener.local_addr()?.to_string();
        let (release, released) = mpsc::channel();
        let reader = thread::spawn(move || {
            let (mut stream, _) = reader_listener.accept().map_err(|error| error.to_string())?;
            read_marker(&mut stream, STDERR_PENDING)?;
            let _ = released.recv();
            stream.write_all(&[1]).map_err(|error| error.to_string())?;
            stream.flush().map_err(|error| error.to_string())
        });
        let release_on_shutdown = release.clone();
        let shutdown = thread::spawn(move || {
            let (mut stream, _) = shutdown_listener.accept().map_err(|error| error.to_string())?;
            read_marker(&mut stream, SHUTDOWN_OBSERVED)?;
            release_on_shutdown.send(()).map_err(|error| error.to_string())
        });
        Ok(Self {
            reader_address,
            shutdown_address,
            release,
            reader: Some(reader),
            shutdown: Some(shutdown),
        })
    }

    pub(super) fn reader_address(&self) -> Result<String, Box<dyn std::error::Error>> {
        Ok(self.reader_address.clone())
    }

    pub(super) fn shutdown_address(&self) -> Result<String, Box<dyn std::error::Error>> {
        Ok(self.shutdown_address.clone())
    }

    pub(super) fn complete(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        join(self.shutdown.take())?;
        join(self.reader.take())
    }
}

impl Drop for StderrPublicationGate {
    fn drop(&mut self) {
        let _ = self.release.send(());
    }
}

fn read_marker(stream: &mut TcpStream, expected: &[u8]) -> Result<(), String> {
    let mut marker = vec![0_u8; expected.len()];
    stream.read_exact(&mut marker).map_err(|error| error.to_string())?;
    (marker == expected)
        .then_some(())
        .ok_or_else(|| "fixture gate marker mismatch".to_owned())
}

fn join(handle: Option<JoinHandle<Result<(), String>>>) -> Result<(), Box<dyn std::error::Error>> {
    let handle = handle.ok_or("fixture gate worker missing")?;
    handle
        .join()
        .map_err(|_| "fixture gate worker panicked")?
        .map_err(Into::into)
}
