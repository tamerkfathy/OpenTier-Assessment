use crate::message::{ClientMessage, ServerMessage, server_message};
use log::{error, info, warn};
use prost::Message;
use std::{
    io::{self, ErrorKind, Read, Write},
    net::{TcpListener, TcpStream},
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    thread::{self, JoinHandle},
    time::Duration,
};

struct Client {
    stream: TcpStream,
}

impl Client {
    pub fn new(stream: TcpStream) -> Self {
        Client { stream }
    }

    pub fn handle(&mut self) -> io::Result<bool> {
        let mut buffer = [0; 1024];
        match self.stream.read(&mut buffer) {
            Ok(0) => {
                info!("Client disconnected.");
                return Ok(false);
            }
            Ok(bytes_read) => {
                if let Ok(message) = ClientMessage::decode(&buffer[..bytes_read]) {
                    match message.message {
                        Some(crate::message::client_message::Message::EchoMessage(echo)) => {
                            info!("Received echo message: {}", echo.content);
                            let response = ServerMessage {
                                message: Some(server_message::Message::EchoMessage(echo)),
                            };
                            let payload = response.encode_to_vec();
                            self.stream.write_all(&payload)?;
                        }
                        Some(crate::message::client_message::Message::AddRequest(add)) => {
                            info!("Received add request: {} + {}", add.a, add.b);
                            let result = add.a + add.b;
                            let response = ServerMessage {
                                message: Some(server_message::Message::AddResponse(
                                    crate::message::AddResponse { result },
                                )),
                            };
                            let payload = response.encode_to_vec();
                            self.stream.write_all(&payload)?;
                        }
                        None => {
                            error!("Received empty message");
                        }
                    }
                    self.stream.flush()?;
                    Ok(true)
                } else {
                    error!("Failed to decode message");
                    Ok(true)
                }
            }
            Err(e) if e.kind() == ErrorKind::WouldBlock => Ok(true),
            Err(e) => Err(e),
        }
    }
}

pub struct Server {
    listener: Option<TcpListener>,
    is_running: Arc<AtomicBool>,
    client_threads: Arc<parking_lot::Mutex<Vec<JoinHandle<()>>>>,
}

impl Server {
    pub fn new(addr: &str) -> io::Result<Self> {
        match TcpListener::bind(addr) {
            Ok(listener) => {
                Ok(Server {
                    listener: Some(listener),
                    is_running: Arc::new(AtomicBool::new(false)),
                    client_threads: Arc::new(parking_lot::Mutex::new(Vec::new())),
                })
            }
            Err(e) => {
                error!("Failed to bind to address {}: {}", addr, e);
                Err(e)
            }
        }
    }

    pub fn run(&self) -> io::Result<()> {
        let listener = self.listener.as_ref().ok_or_else(|| {
            io::Error::new(io::ErrorKind::Other, "Server not properly initialized")
        })?;

        self.is_running.store(true, Ordering::SeqCst);
        info!("Server is running on {}", listener.local_addr()?);

        listener.set_nonblocking(true)?;

        while self.is_running.load(Ordering::SeqCst) {
            match listener.accept() {
                Ok((stream, addr)) => {
                    info!("New client connected: {}", addr);
                    
                    let is_running = Arc::clone(&self.is_running);
                    let client_threads = Arc::clone(&self.client_threads);
                    
                    let handle = thread::spawn(move || {
                        let mut client = Client::new(stream);
                        while is_running.load(Ordering::SeqCst) {
                            match client.handle() {
                                Ok(true) => continue,
                                Ok(false) | Err(_) => break,
                            }
                        }
                        info!("Client handler thread terminated");
                    });

                    client_threads.lock().push(handle);
                }
                Err(ref e) if e.kind() == ErrorKind::WouldBlock => {
                    thread::sleep(Duration::from_millis(100));
                }
                Err(e) => {
                    error!("Error accepting connection: {}", e);
                }
            }
        }

        // Wait for all client threads to finish
        let mut threads = self.client_threads.lock();
        while let Some(handle) = threads.pop() {
            if let Err(e) = handle.join() {
                error!("Error joining client thread: {:?}", e);
            }
        }

        info!("Server stopped.");
        Ok(())
    }

    pub fn stop(&self) {
        if self.is_running.load(Ordering::SeqCst) {
            self.is_running.store(false, Ordering::SeqCst);
            info!("Shutdown signal sent.");
        } else {
            warn!("Server was already stopped or not running.");
        }
    }
}