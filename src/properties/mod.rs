use std::{path::{Path, PathBuf}, str::FromStr, fs};

use serde::{Deserialize, Serialize};
use thiserror::Error;

mod description_parser;
mod ast;
mod environment;
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
    pub(crate) block_number: Option<String>,
    //pub(crate) payer: Option<Account>,
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
            //payer: None,
            payer_address: None,
            value: None,
            payer_balance_before: None,
            payer_balance_after: None,
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