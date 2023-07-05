#[test]
fn build_chain_config(){
    // Build a ChainConfig
    let contract_address = "0x071b8f8f375A1932BAAF356BcA98aAC0128bf5bf".to_string();
    let topics = vec![
        "0x2bce2dbcfd5eae5755dcd9c531a42fbb14a7197117c13854ba063d6fd831437f".to_string(), // SendEthToSol
        "0xed913c13627cc9986ea63489464a9bff9784eabcda6430bce99c500cce13cbe8".to_string(), // InvokeUnlock
        "0xaa568fa165117282ddb19ac1797e528ace29d912dbe3d693cab857e8c02b6396".to_string(),  // ReceiveEthFromSol
    ];

    let topics_json: Vec<Value> = topics.clone()
    .into_iter()
    .map(|topic| json!(topic))
    .collect();

    let eth_config = ChainConfig::new(
        32382,
        "ws://127.0.0.1:8546".to_string(),
        "Ethereum".to_string(),
        contract_address.clone(),
        "eth_subscribe".to_string(),
        
        json!([
            "logs",
            {
                "address": contract_address.clone(),
                "topics": [topics_json]
            }
        ])
        );
    fs::write(
        "config\\ethereum_config.json",
        serde_json::to_string_pretty(&eth_config).unwrap(),
    )
    .unwrap();
}

