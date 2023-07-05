use crate::properties::Properties;

/// Convert hex string to u64 and remove leading 0x
pub fn hex_string_to_u64(hex_string: &str) -> u64 {
    let mut string_hex = hex_string;
    if string_hex.starts_with("0x"){
        string_hex = &string_hex[2..];
    }
    u64::from_str_radix(string_hex, 16).unwrap()
}

/// Convert u64 into hex string 0xXXXXX
pub fn u64_to_hex_string(u64: u64) -> String {
    format!("0x{:X}", u64)
}

/// Return the property and the index which match the hash
pub fn find_property_by_hash(hash: String, properties: &Vec<Properties>) -> Option<(Properties, u64)> {
    let mut id = 0;
    for property in properties.iter(){
        if property.transaction_hash == Some(hash.clone()){
            return Some((property.clone(), id));
        }
        id += 1;
    }
    None
}