"event": "ProofCreated(bytes32,address)",
{
    $keystore.push($event_data.slice(2,66).as(hex))
}