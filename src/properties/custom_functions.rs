use std::collections::HashMap;
use std::path::Path;
use std::fs;
use std::str::FromStr;

use ethnum::{u256, i256, U256};
use serde_json::{Value, Number};
use hex::encode;

use super::error;

use crate::{ChainConfig, set_var, get_var, utils};
use crate::properties::environment::*;
use crate::properties::ast::*;

/// Create a custom struct from a json file
/// The struct is parsed exactly as the json file
pub fn struct_from_json(path_to_json: &str) -> serde_json::Value {
    let content = fs::read_to_string(Path::new(path_to_json)).unwrap();
    let val = serde_json::from_str(&content).unwrap();
    val
}

/// Check each field of the Value struct whether it should be replaced by a property
/// The function returns a HashMap with local variables, which can be used in the pattern
pub fn execute_custom_function(val: &Value) -> Result<HashMap<String, Value>, error::PropertyError> {
    let mut results: HashMap<String, Value> = HashMap::new();
    let properties = val.get("properties").unwrap().as_object().unwrap();
    for (variable_name, value) in properties{
        //println!("{:?} {:?}", key, value.as_str().unwrap());
        let p = value.as_str().unwrap().split('.');
        let func: Vec<&str> = p.collect();
        let chain_name = func[0];
        let function = func[1];
        let field_and_type: Vec<&str> = func[2].split(" ").collect();
        let fieldname = field_and_type[0];        

        // Build Path for Chain Config
        let f = format!("config/{}_config.json", chain_name);
        let p = Path::new(&f);

        let config: ChainConfig = serde_json::from_str(&fs::read_to_string(p).unwrap()).unwrap();

        // Process function
        let f: Vec<&str> = function.split("(").collect();
        let function_name = f[0];
        let parameter_list = &f[1][..f[1].len()-1];

        let function_path = format!("functions/{}/{}.json", chain_name, function_name);
        //println!("{:?}", function_path);
        let mut function_json: Value = serde_json::from_str(std::fs::read_to_string(&function_path).unwrap().as_str()).unwrap();

        let block_function: Value = serde_json::from_str(std::fs::read_to_string(&format!("functions/{}/get_block_number.json", chain_name)).unwrap().as_str()).unwrap();
        let client = reqwest::blocking::Client::new();
        let res = client.post(&config.get_http_url()).json(&block_function).send().unwrap();
        let body = res.text().unwrap();
        let json_body: Value = serde_json::from_str(&body.as_str()).unwrap();
        let block_number = json_body.get("result").unwrap();
        let var_name: String = format!("{}_block_number", chain_name);
        set_var!(var_name, block_number.as_str().unwrap());

        // Get Parameter and change Variable fields
        let parameters_json: Vec<&str> = parameter_list.split(",").collect();
        let mut params: Vec<String> = Vec::new();
        // Remove first and last element of parameters
        for p in parameters_json{
            let mut s = p.clone().trim().replace("'", "");
            // println!("{}", s);
            if s == "current_block" {
                s = block_number.as_str().unwrap().to_string();    
            }else if s.contains("current_block"){
                // Replace the current_block with the block number and make an u64 from it. Then, make it back to string again and build an AST from it
                let exchanged_string = s.replace("current_block", crate::utils::hex_string_to_u64(block_number.as_str().unwrap()).to_string().as_str());
                let (_, root) = build_ast!(exchanged_string);
                let block = crate::utils::u64_to_hex_string(root.evaluate().unwrap().get_value().parse::<u64>().unwrap());
                s = block.to_string();
            }

            if s.starts_with("$"){
                let parts: Vec<&str> = s.split(" ").collect();
                let var_name = parts[0];
                if parts.len() > 1{
                    let var_type = parts[2];
                    let var = get_var!(value var_name).unwrap();
                    match var_type {
                        "hex" => {
                            print!("Found {}", s);
                            let num = U256::from_str_radix(var.as_str(), 10).unwrap();
                            let arr: [u8; 32] = num.to_le_bytes();
                            let hex_string = format!("0x{}", encode(&arr));
                            s = hex_string;
                            println!(" to {}", s);
                        },
                        _ => {
                            print!("Found {}", s);
                            let v = &s[1..];
                            s = get_var!(value v).unwrap();
                            println!(" to {}", s);
                        }
                    }
                }else {
                    print!("Found {}", s);
                    let v = &s[1..];
                    s = get_var!(value v).unwrap();
                    println!(" to {}", s);
                }
                
            }
            // println!("->{}", s);
            params.push(s);
        }

        let mut counter = 0;
        replace_variables(&mut function_json, params, &mut counter);

        // println!("{:?}", function_json);        

        let res = client.post(&config.get_http_url()).json(&function_json).send().unwrap();
        let body = res.text().unwrap();
        // println!("{:?}", body);
        let result:Value = serde_json::from_str(&body.as_str()).unwrap();
        println!("Result: {:?}", result);
        if let Some(object) = result.as_object(){
            let mut found = false;
            for (key, value) in object{
                if key == fieldname {
                    //println!("Found {}", value);
                    // Change the Value Type
                    if field_and_type.len() == 3{
                        let fieldtype = field_and_type[2];
                        match fieldtype {
                            "u256" => {
                                //println!("u256");
                                let mut temp = value.as_str().unwrap().clone();
                                if temp.starts_with("0x"){
                                    let num = utils::hex_string_to_u256(temp).to_string();
                                    let string_num = "u256:".to_owned() + &num;
                                    let v = Value::String(string_num);
                                    //println!("Starts {:?}", v);
                                    results.insert(variable_name.to_string(), v);
                                }else{
                                    let v = serde_json::json!(value.clone().as_u64().unwrap());
                                    //println!("{}", v);
                                    results.insert(variable_name.to_string(), v);
                                }
                            },
                            "i256" => {
                                //println!("i256");
                                let mut temp = value.as_str().unwrap().clone();
                                if temp.starts_with("0x"){
                                    let v = serde_json::json!(utils::hex_string_to_i256(temp));
                                    results.insert(variable_name.to_string(), v);
                                }else{
                                    let v = serde_json::json!(value.clone().as_u64().unwrap());
                                    results.insert(variable_name.to_string(), v);
                                }
                            },
                            "bool" => {
                                //println!("bool");
                                let v = serde_json::json!(value.clone().as_bool().unwrap());
                                results.insert(variable_name.to_string(), v);
                            },
                            "array" => {
                                //println!("array");
                                let v = serde_json::json!(value.clone().as_array().unwrap());
                                results.insert(variable_name.to_string(), v);
                            },
                            "string" => {
                                //println!("string");
                                results.insert(variable_name.to_string(), value.clone());
                            },
                            _ => {
                                println!("Unknown Type");
                                results.insert(variable_name.to_string(), value.clone());
                            }
                        }
                    }else{
                        results.insert(variable_name.to_string(), value.clone());
                    }
                    found = true;
                    break;
                }
            }
            if !found {
                return Err(error::PropertyError::FieldNotFound);
            }
        }
    }
    Ok(results)
}

/// Replace the variables in the json file with the specified parameters of the property
pub fn replace_variables(json: &mut Value, params: Vec<String>, counter: &mut usize) {
    match json {
        Value::Object(map) => {
            for (key, value) in map.iter_mut() {
                //println!("{}", counter);
                replace_variables(value, params.clone(), counter);
            }
        },
        Value::Array(arr) => {
            for v in arr.iter_mut() {
                //println!("{}", counter);
                replace_variables(v, params.clone(), counter);
            }
        },
        Value::String(s) if s.starts_with("$") => {
            if s == "$id" {
                *s = "0".to_string();
            }else{
                //println!("Changed variable: {} to {}", s, params[*counter]);
                *s = params[*counter].to_string();
                *counter+=1;
            }
        },
        _ => {},
    }
}

#[test]
fn test_execute_custom_function() {
    let val = struct_from_json("D:/Masterarbeit/brigade/properties/test_definition.json");
    let results = execute_custom_function(&val).unwrap();
    println!("{:?}", results);

    for (key, value) in results {
        if value.is_string() {
            if value.as_str().unwrap().starts_with("u256:"){
                let s = &value.as_str().unwrap()[5..];
                set_var!(key, u256::from_str(s).unwrap());
            }
            if value.as_str().unwrap().starts_with("i256:"){
                let s = &value.as_str().unwrap()[5..];
                set_var!(key, i256::from_str(s).unwrap());
            }
        }else{
            set_var!(key, value);
        }
    }

    let var: u256 = get_var!(u256 "balance_before").unwrap();

    println!("{:?}", var);

}