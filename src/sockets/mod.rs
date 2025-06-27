pub mod ethereum_socket;
pub mod event_socket;
pub mod socket;
pub mod solana_socket;
pub mod replay_ethereum_socket;

pub enum SocketTypes {
    Ethereum(ethereum_socket::EthereumSocketHandler),
    Solana(solana_socket::SolanaSocketHandler),
    WebSocket(socket::WebSocketClientHandler),
    // TODO: Add replay handler
    ReplayEthereum(replay_ethereum_socket::ReplayEthereumSocketHandler),
    // The handler should receive a message from the function: trace_replayBlockTransactions and then check for all contracts that are stored in the var and then for their events.
    // The handler should process each block and immediately send the next replayBlockTransactions message.
    //ReplayEthereum(socket::replay::EthereumReplayHandler),
}
