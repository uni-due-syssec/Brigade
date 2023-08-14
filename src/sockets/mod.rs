pub mod ethereum_socket;
pub mod socket;
pub mod event_socket;

pub enum SocketTypes {
    Ethereum(ethereum_socket::EthereumSocketHandler),
    WebSocket(socket::WebSocketClientHandler),
}