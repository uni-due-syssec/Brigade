use std::{path::PathBuf, str::FromStr, fs};

use ethnum::{u256, uint};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use thiserror::Error;


mod description_parser;
#[macro_use]
pub mod ast;

pub mod custom_functions;

pub(crate) mod environment;
pub mod definition;
mod error;

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
    pub(crate) block_number: Option<u256>,
    //pub(crate) payer: Option<Account>,
    pub(crate) payer_address: Option<String>,
    pub(crate) payer_balance_before: Option<u256>,
    pub(crate) payer_balance_after: Option<u256>,
    pub(crate) value: Option<u256>,
    pub(crate) src_chain: Option<String>,
}

impl Properties{
    pub fn new() -> Self {
        Self{
            occured_event: None,
            transaction_hash: None,
            block_number: None,
            //payer: None,
            payer_address: None,
            value: None,
            payer_balance_before: None,
            payer_balance_after: None,
            src_chain: None
        }
    }

    pub fn check(&mut self) -> Result<&'static str, PropertyError>{
        let mut property_description = PathBuf::from_str("properties").unwrap();
        property_description = property_description.canonicalize().unwrap();

        if !property_description.exists(){
            return Err(PropertyError::PropertyFolderNotFound);
        }

        // Get All Files in the property_description directory
        let files = fs::read_dir(property_description).unwrap();
        let file_paths = files.map(|f| f.unwrap().path());

        

        Ok("Transaction can be processed!")
    }

    pub fn serialize(&self) -> Value {
        serde_json::json!({
            "payer_address": self.payer_address,
            "payer_balance_before": format!("u256:{}", self.payer_balance_before.unwrap_or(uint!("0"))),
            "payer_balance_after": format!("u256:{}", self.payer_balance_after.unwrap_or(uint!("0"))),
            "block_number": format!("u256:{}",self.block_number.unwrap_or(uint!("0"))),
            "occured_event": self.occured_event,
            "src_chain": self.src_chain,
            "transaction_hash": self.transaction_hash,
            "value": format!("u256:{}",self.value.unwrap_or(uint!("0")))
        })
    }
}

#[derive(Error, Debug)]
pub enum PropertyError{
    #[error("the property description folder does not exist")]
    PropertyFolderNotFound,
    #[error("the property is invalid")]
    InvalidProperty,
}
/// Struct to manage Accounts 
#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct Account{
    pub address: Option<String>,
    pub balance: Option<String>,
    pub previous_balance: Option<String>
}

#[test]
fn test_prp_as_vars() {

    use crate::{set_var, get_var, properties::environment::GetVar};
    use crate::VarValues;
    use crate::get_variable_map_instance;
    use ethnum::AsU256;
    use ethnum::i256;
    let mut prp = Properties::new();

    prp.payer_address = Some("0x0".to_string());
    prp.payer_balance_before = Some(15000.as_u256());
    prp.payer_balance_after = Some(666666.as_u256());
    prp.block_number = Some(10.as_u256());
    prp.occured_event = Some("Flugzeug".to_string());
    prp.src_chain = Some("ethereum".to_string());
    prp.transaction_hash = Some("0x1".to_string());
    prp.value = Some(1000.as_u256());

    let prp_val = prp.serialize();

    for (key, value) in prp_val.as_object().unwrap() {
        if value.is_string() && value.as_str().unwrap().starts_with("u256:"){
            let s = &value.as_str().unwrap()[5..];
            set_var!(key, u256::from_str(s).unwrap());
        }
        else if value.is_string() && value.as_str().unwrap().starts_with("i256:"){
            let s = &value.as_str().unwrap()[5..];
            set_var!(key, i256::from_str(s).unwrap());
        }else{
            set_var!(key, value.clone() );
        }
    }

    let addr = String::get_value(get_var!("payer_address").unwrap());

    assert_eq!(addr, Some("0x0".to_string()));

    let v = u256::get_value(get_var!("payer_balance_before").expect("Value not found")).unwrap();
    assert_eq!(v, 15000.as_u256());
}