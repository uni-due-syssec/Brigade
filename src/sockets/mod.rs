pub mod ethereum_socket;
pub mod solana_socket;
pub mod socket;
pub mod event_socket;

pub enum SocketTypes {
    Ethereum(ethereum_socket::EthereumSocketHandler),
    Solana(solana_socket::SolanaSocketHandler),
    WebSocket(socket::WebSocketClientHandler),
}