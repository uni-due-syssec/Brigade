use ws::Handler;
use serde_json::Value;

/// The Endpoint Client for the Blockchain Smart Contracts
/// Here a Handler will fetch and process the events and take care of the websocket connection
pub struct WebSocketClientHandler{
    // State of the Client
}

impl Handler for WebSocketClientHandler {
    fn on_message(&mut self, msg: ws::Message) -> ws::Result<()> {

        if let Ok(json_value) = serde_json::from_str::<Value>(&msg.to_string()) {
            // Convert the JSON value to a pretty-printed string
            if let Ok(pretty_json) = serde_json::to_string_pretty(&json_value) {
                // Print the pretty-printed JSON string
                println!("{}", pretty_json);
            }
        }else {
            println!("Invalid JSON");
        }
        Ok(())
    }

    fn on_open(&mut self, _shake: ws::Handshake) -> ws::Result<()> {
        println!("Open");
        Ok(())
    }
}