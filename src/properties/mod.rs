use serde::{Deserialize, Serialize};


/// List of properties
/// 
/// LockLogic:
/// Occured Event -> Act as trigger for the property
/// Transaction Hash -> Identify the Transaction
/// Block Number -> Identify the Block
/// Payer Address -> Account Address that paid for the Lock
/// Value -> Amount of Currency Locked
/// 
/// UnlockLogic:
/// Occured Event -> Act as trigger for the property
/// Transaction Hash -> Identify the Transaction
/// Block Number -> Identify the Block
/// Payer Address -> Account Address that paid for the Lock
/// Value -> Amount of Currency Locked
/// 
/// The Properties struct should contain all properties needed by the logic parser
#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct Properties{
    pub(crate) occured_event: Option<String>,
    pub(crate) transaction_hash: Option<String>,
    pub(crate) block_number: Option<String>,
    pub(crate) payer_address: Option<String>,
    pub(crate) payer_balance_before: Option<String>,
    pub(crate) payer_balance_after: Option<String>,
    pub(crate) value: Option<String>
}

impl Properties{
    pub fn new() -> Self {
        Self{
            occured_event: None,
            transaction_hash: None,
            block_number: None,
            payer_address: None,
            value: None,
            payer_balance_before: None,
            payer_balance_after: None,
        }
    }
}