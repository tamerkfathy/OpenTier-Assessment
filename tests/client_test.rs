use embedded_recruitment_task::{
    message::{client_message, server_message, AddRequest, EchoMessage},
    server::Server,
};
use std::{
    sync::Arc,
    thread::{self, JoinHandle},
    net::TcpListener,
};

mod client;

fn find_available_port() -> u32 {
    // Try ports in range 8000-8999
    for port in 8000..9000
    {
        if let Ok(bind) = TcpListener::bind(("127.0.0.1", port))
        {
           return port.into();
        }
    }
    panic!("No available ports found in range 8000-8999");
}

fn setup_server_thread(server: Arc<Server>) -> JoinHandle<()> {
    thread::spawn(move || {
        server.run().expect("Server encountered an error");
    })
}

fn create_server() -> (Arc<Server>, u32) {
    let port = find_available_port();
    let server = Arc::new(Server::new(&format!("127.0.0.1:{}", port))
        .expect("Failed to start server"));
    (server, port)
}

#[test]
fn test_client_connection() {
    let (server, port) = create_server();
    let handle = setup_server_thread(server.clone());

    // Allow server time to start
    thread::sleep(std::time::Duration::from_millis(100));

    let mut client = client::Client::new("127.0.0.1", port, 1000);
    assert!(client.connect().is_ok(), "Failed to connect to the server");

    assert!(
        client.disconnect().is_ok(),
        "Failed to disconnect from the server"
    );

    server.stop();
    assert!(
        handle.join().is_ok(),
        "Server thread panicked or failed to join"
    );
}

#[test]
fn test_client_echo_message() {
    let (server, port) = create_server();
    let handle = setup_server_thread(server.clone());

    thread::sleep(std::time::Duration::from_millis(100));

    let mut client = client::Client::new("127.0.0.1", port, 1000);
    assert!(client.connect().is_ok(), "Failed to connect to the server");

    let mut echo_message = EchoMessage::default();
    echo_message.content = "Hello, World!".to_string();
    let message = client_message::Message::EchoMessage(echo_message.clone());

    assert!(client.send(message).is_ok(), "Failed to send message");

    let response = client.receive();
    assert!(
        response.is_ok(),
        "Failed to receive response for EchoMessage"
    );

    match response.unwrap().message {
        Some(server_message::Message::EchoMessage(echo)) => {
            assert_eq!(
                echo.content, echo_message.content,
                "Echoed message content does not match"
            );
        }
        _ => panic!("Expected EchoMessage, but received a different message"),
    }

    assert!(
        client.disconnect().is_ok(),
        "Failed to disconnect from the server"
    );

    server.stop();
    assert!(
        handle.join().is_ok(),
        "Server thread panicked or failed to join"
    );
}

#[test]
fn test_multiple_echo_messages() {
    let (server, port) = create_server();
    let handle = setup_server_thread(server.clone());

    thread::sleep(std::time::Duration::from_millis(100));

    let mut client = client::Client::new("127.0.0.1", port, 1000);
    assert!(client.connect().is_ok(), "Failed to connect to the server");

    let messages = vec![
        "Hello, World!".to_string(),
        "How are you?".to_string(),
        "Goodbye!".to_string(),
    ];

    for message_content in messages {
        let mut echo_message = EchoMessage::default();
        echo_message.content = message_content.clone();
        let message = client_message::Message::EchoMessage(echo_message);

        assert!(client.send(message).is_ok(), "Failed to send message");

        let response = client.receive();
        assert!(
            response.is_ok(),
            "Failed to receive response for EchoMessage"
        );

        match response.unwrap().message {
            Some(server_message::Message::EchoMessage(echo)) => {
                assert_eq!(
                    echo.content, message_content,
                    "Echoed message content does not match"
                );
            }
            _ => panic!("Expected EchoMessage, but received a different message"),
        }
    }

    assert!(
        client.disconnect().is_ok(),
        "Failed to disconnect from the server"
    );

    server.stop();
    assert!(
        handle.join().is_ok(),
        "Server thread panicked or failed to join"
    );
}

#[test]
fn test_multiple_clients() {
    let (server, port) = create_server();
    let handle = setup_server_thread(server.clone());

    thread::sleep(std::time::Duration::from_millis(100));

    let mut clients = vec![
        client::Client::new("127.0.0.1", port, 1000),
        client::Client::new("127.0.0.1", port, 1000),
        client::Client::new("127.0.0.1", port, 1000),
    ];

    for client in clients.iter_mut() {
        assert!(client.connect().is_ok(), "Failed to connect to the server");
    }

    let messages = vec![
        "Hello, World!".to_string(),
        "How are you?".to_string(),
        "Goodbye!".to_string(),
    ];

    for message_content in messages {
        let mut echo_message = EchoMessage::default();
        echo_message.content = message_content.clone();
        let message = client_message::Message::EchoMessage(echo_message.clone());

        for client in clients.iter_mut() {
            assert!(
                client.send(message.clone()).is_ok(),
                "Failed to send message"
            );

            let response = client.receive();
            assert!(
                response.is_ok(),
                "Failed to receive response for EchoMessage"
            );

            match response.unwrap().message {
                Some(server_message::Message::EchoMessage(echo)) => {
                    assert_eq!(
                        echo.content, message_content,
                        "Echoed message content does not match"
                    );
                }
                _ => panic!("Expected EchoMessage, but received a different message"),
            }
        }
    }

    for client in clients.iter_mut() {
        assert!(
            client.disconnect().is_ok(),
            "Failed to disconnect from the server"
        );
    }

    server.stop();
    assert!(
        handle.join().is_ok(),
        "Server thread panicked or failed to join"
    );
}

#[test]
fn test_client_add_request() {
    let (server, port) = create_server();
    let handle = setup_server_thread(server.clone());

    thread::sleep(std::time::Duration::from_millis(100));

    let mut client = client::Client::new("127.0.0.1", port, 1000);
    assert!(client.connect().is_ok(), "Failed to connect to the server");

    let mut add_request = AddRequest::default();
    add_request.a = 10;
    add_request.b = 20;
    let message = client_message::Message::AddRequest(add_request.clone());

    assert!(client.send(message).is_ok(), "Failed to send message");

    let response = client.receive();
    assert!(
        response.is_ok(),
        "Failed to receive response for AddRequest"
    );

    match response.unwrap().message {
        Some(server_message::Message::AddResponse(add_response)) => {
            assert_eq!(
                add_response.result,
                add_request.a + add_request.b,
                "AddResponse result does not match"
            );
        }
        _ => panic!("Expected AddResponse, but received a different message"),
    }

    assert!(
        client.disconnect().is_ok(),
        "Failed to disconnect from the server"
    );

    server.stop();
    assert!(
        handle.join().is_ok(),
        "Server thread panicked or failed to join"
    );
}