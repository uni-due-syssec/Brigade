pub mod ethereum_socket;
pub mod socket;

pub enum SocketTypes {
    Ethereum(ethereum_socket::EthereumSocketHandler),
    WebSocket(socket::WebSocketClientHandler),
}