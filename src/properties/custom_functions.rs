use std::collections::HashMap;
use std::path::Path;
use std::fs;
use std::str::FromStr;

use serde_json::Value;

use super::error;

use crate::{ChainConfig, set_var, get_var};
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
/// Then send the call with correct ID
pub fn execute_custom_function(val: Value) -> Result<HashMap<String, Value>, error::PropertyError> {
    let mut results: HashMap<String, Value> = HashMap::new();
    let properties = val.get("properties").unwrap().as_object().unwrap();
    for (variable_name, value) in properties{
        //println!("{:?} {:?}", key, value.as_str().unwrap());
        let p = value.as_str().unwrap().split('.');
        let func: Vec<&str> = p.collect();
        let chain_name = func[0];
        let function = func[1];
        let fieldname = func[2];

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
            println!("{}", s);
            if s == "current_block" {
                s = block_number.as_str().unwrap().to_string();    
            }else if s.contains("current_block"){
                // Replace the current_block with the block number and make an u64 from it. Then, make it back to string again and build an AST from it
                let exchanged_string = s.replace("current_block", crate::utils::hex_string_to_u64(block_number.as_str().unwrap()).to_string().as_str());
                let (_, root) = build_ast!(exchanged_string);
                let block = crate::utils::u64_to_hex_string(root.evaluate().unwrap().get_value().parse::<u64>().unwrap());
                s = block.to_string();
            }
            println!("->{}", s);
            params.push(s);
        }

        let mut counter = 0;
        replace_variables(&mut function_json, params, &mut counter);

        println!("{:?}", function_json);        

        let res = client.post(&config.get_http_url()).json(&function_json).send().unwrap();
        let body = res.text().unwrap();
        println!("{:?}", body);
        let result:Value = serde_json::from_str(&body.as_str()).unwrap();

        if let Some(object) = result.as_object(){
            let mut found = false;
            for (key, value) in object{
                if key == fieldname {
                    //println!("Found {}", value);
                    results.insert(variable_name.to_string(), value.clone());
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
    let results = execute_custom_function(val);
    println!("{:?}", results.unwrap());
}