use std::collections::{HashMap, HashSet};
use std::f32::consts::E;
use std::path::Path;
use std::{fs, vec};

use serde_json::Value;

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

    // Solve dependencies first
    let mut dependencies: HashMap<String, HashSet<String>> = HashMap::new();
    let possible_dependencies: Vec<&String> = properties.keys().collect();
    for (key, value) in properties {
        let mut deps = HashSet::new();
        // Check if dependant
        for (i, d) in possible_dependencies.iter().enumerate(){
            let re = regex::Regex::new(&format!(r"\${}[^a-zA-Z0-9_]", d)).unwrap();
            if re.is_match(value.as_str().unwrap()){
                deps.insert(d.to_string());
            }
        }
        dependencies.insert(key.clone(), deps);
    }

    // println!("Dependencies: {:?}", dependencies);

    // Sort properties
    let sorted = sort_dependencies(&dependencies).unwrap();

    // println!("Sorted: {:?}", sorted);
    for variable_name in sorted{

    // //for (variable_name, value) in properties{
        let value = properties.get(&variable_name).unwrap();

        if value.as_str().unwrap().starts_with('$'){
            //  Only Parse the variables 
            let v = value.as_str().unwrap();
            // println!("{}: {}", variable_name, v);
            match crate::build_ast_root(v){
                Ok(root) => {
                    match root.evaluate(){
                        Ok(r) => {
                            // println!("Evaluated: {} to {:?}", variable_name, r);
                            set_var!(variable_name, r);
                        },
                        Err(e) => {
                            println!("Failed to evaluate: {} with reason: {:?}", variable_name, e);
                        }
                    }
                },
                Err(e) => {
                    println!("Failed to parse: {}", variable_name);
                    eprintln!("Error: {:?}", e);
                }
            }
            continue;
        }

        if !value.as_str().unwrap().contains('.') && !value.as_str().unwrap().contains('$'){
            set_var!(variable_name, value.clone());
            // println!("{}: {}", variable_name, value.as_str().unwrap());
            continue;
        }

        // println!("{:?} {:?}", variable_name, value.as_str().unwrap());
        let tokens = tokenize(value.as_str().unwrap().to_string());
        //let p = value.as_str().unwrap().split('.');
        let func: Vec<&str> = tokens.iter().map(|s| s.as_str()).collect();
        let chain_name = func[0];
        let function = func[1];
        let field_and_type: Vec<&str> = func[2].split(" ").collect();
        let fieldname = field_and_type[0];
        // println!("Fieldname: {}", fieldname);

        // Build Path for Chain Config
        let f = format!("config/{}_config.json", chain_name);
        let p = Path::new(&f);

        let config: ChainConfig = match fs::read_to_string(p) {
            Ok(s) => {
                serde_json::from_str(&s).unwrap()
            },
            Err(e) => 
            {
                // println!("Searching for file");
                let mut cc: ChainConfig = ChainConfig::default();
                // Find path for chain config
                for file in fs::read_dir("config").unwrap() {
                    let path = file.unwrap().path();
                    cc = serde_json::from_str(&fs::read_to_string(path.as_path()).unwrap()).unwrap();
                    if cc.get_name() == chain_name{
                        // println!("Found: {}", path.display());
                        break;
                    }
                }
                cc
            }
        };

        // let config: ChainConfig = serde_json::from_str(&fs::read_to_string(p).unwrap()).unwrap();

        // Process function
        let temp = tokenize_function(function.to_string());
        let mut f: Vec<&str> = temp.iter().map(|x| x.as_str()).collect();
        let function_name = f[0].clone();
        f.remove(0);
        let mut parameter_list = f.iter().map(|x| x.to_string()).collect::<Vec<String>>();

        let function_path = format!("functions/{}/{}.json", chain_name, function_name);
        //println!("{:?}", function_path);
        let mut function_json: Value = serde_json::from_str(std::fs::read_to_string(&function_path).unwrap().as_str()).unwrap();

        // let block_function: Value = serde_json::from_str(std::fs::read_to_string(&format!("functions/{}/get_block_number.json", chain_name)).unwrap().as_str()).unwrap();
        // let client = reqwest::blocking::Client::new();
        // let res = client.post(&config.get_http_url()).json(&block_function).send().unwrap();
        // let body = res.text().unwrap();
        // let json_body: Value = serde_json::from_str(&body.as_str()).unwrap();
        // let block_number = json_body.get("result").unwrap();
        // // println!("Block Number: {}", block_number.as_str().unwrap());

        // // TODO: Refactor: Move to event detection as the block number is updated on each function call right now.
        // let var_name: String = format!("{}_block_number", chain_name);
        // set_var!(var_name, block_number.as_str().unwrap());

        // Get Parameter and change Variable fields
        let mut params = vec![];
        for p in parameter_list.iter(){
            let mut s = p.clone().trim_start().replace("'", "");

            // Parse Variables
            if s.contains("$"){
                
                // $var | $var - 1 | $var.as(hex)
                if !s.contains(")."){
                    let root = build_ast_root(s.clone().as_str()).unwrap();
                    // println!("Root: {:?}", root);
                    match root.evaluate() {
                        Ok(val) => {
                            s = val.get_value().to_string();
                            // println!("Variable: {}", s);
                        },
                        Err(e) => {
                            println!("Variable error: {}", e);
                        }
                    }
                }else{// ($var - 1 ). as(hex)
                    let parts: Vec<&str> = s.split(").").collect();
                    let stmt1 = &parts[0][1..];
                    // println!("Statement: {}", stmt1);
                    let root = build_ast_root(stmt1).unwrap();
                    match root.evaluate(){
                        Ok(val) => {
                            // Make Conversion
                            let st = val.get_value().to_string() + "." + parts[1];
                            // println!("Statement: {}", st);
                            let stmt2 = build_ast_root(st.as_str()).unwrap();
                            match stmt2.evaluate(){
                                Ok(val) => {
                                    s = val.get_value().to_string();
                                    // println!("Variable: {}", s);
                                },
                                Err(e) => {
                                    println!("Variable error: {}", e);
                                }
                                
                            }
                        },
                        Err(e) => {
                            println!("Variable error: {}", e);
                        }
                    }
                }
            }
            params.push(s.clone());
        }

        let mut counter = 0;
        replace_variables(&mut function_json, params, &mut counter);

        // println!("{:?}", function_json);  
        let client = reqwest::blocking::Client::new();
        let res = client.post(&config.get_http_url()).json(&function_json).send().unwrap();
        let body = res.text().unwrap();
        // println!("{:?}", body);
        let result:Value = serde_json::from_str(&body.as_str()).unwrap();
        // println!("Result: {}", serde_json::to_string_pretty(&result).unwrap());

        let path_to_val: Vec<&str> = fieldname.split(".").collect();
        
        if let Some(v) = find_value_by_path(&result, &path_to_val){
            // println!("{} = {}", fieldname, v);
            if field_and_type.len() == 3{
                let fieldtype = field_and_type[2];
                match fieldtype {
                    "u256" => {
                        //println!("u256");
                        let mut temp = v.as_str().unwrap().clone();
                        if temp.starts_with("0x"){
                            let num = utils::hex_string_to_u256(temp).to_string();
                            let string_num = "u256:".to_owned() + &num;
                            let val = Value::String(string_num);
                            //println!("Starts {:?}", v);
                            set_var!(variable_name, val.clone());
                            results.insert(variable_name.to_string(), val);
                        }else{
                            let val = serde_json::json!(v.clone().as_u64().unwrap());
                            //println!("{}", v);
                            set_var!(variable_name, val.clone());
                            results.insert(variable_name.to_string(), val);
                        }
                    },
                    "i256" => {
                        //println!("i256");
                        let mut temp = v.as_str().unwrap().clone();
                        if temp.starts_with("0x"){
                            let val = serde_json::json!(utils::hex_string_to_i256(temp));
                            set_var!(variable_name, val.clone());
                            results.insert(variable_name.to_string(), val);
                        }else{
                            let val = serde_json::json!(v.clone().as_u64().unwrap());
                            set_var!(variable_name, val.clone());
                            results.insert(variable_name.to_string(), val);
                        }
                    },
                    "bool" => {
                        //println!("bool");
                        let val = serde_json::json!(v.clone().as_bool().unwrap());
                        set_var!(variable_name, val.clone());
                        results.insert(variable_name.to_string(), val);
                    },
                    "array" => {
                        //println!("array");
                        let val = serde_json::json!(v.clone().as_array().unwrap());
                        set_var!(variable_name, val.clone());
                        results.insert(variable_name.to_string(), val);
                    },
                    "string" => {
                        //println!("string");
                        results.insert(variable_name.to_string(), v.clone());
                        set_var!(variable_name, v.clone());
                    },
                    _ => {
                        println!("Unknown Type");
                        results.insert(variable_name.to_string(), v.clone());
                        set_var!(variable_name, v.as_str().unwrap());
                    }
                }
            }else{
                // println!("Key: {}, Value: {}", variable_name, v);
                results.insert(variable_name.to_string(), v.clone());
                set_var!(variable_name, v);
            }
        } else {
            println!("Could not find {} for key {}", result, fieldname);
            return Err(error::PropertyError::FieldNotFound);
        }
    }

    print_variables(&get_variable_map_instance());

    Ok(results)
}

/// Find the Json Value by path
pub fn find_value_by_path(value: &Value, path: &[&str]) -> Option<Value> {
    if path.is_empty(){
        Some(value.clone())
    }else{
        match value {
            Value::Object(map) => {
                let key = path[0];
                let remaining_path = &path[1..];
                map.get(key).and_then(|v| find_value_by_path(v, remaining_path))
            },
            Value::Array(arr) => {
                if let Ok(index) = path[0].parse::<usize>() {
                    let remaining_path = &path[1..];
                    if let Some(v) = arr.get(index) {
                        find_value_by_path(v, remaining_path)
                    }else {
                        None
                    }
                }else{
                    None
                }
            },
            _ => None
        }
    }
}

#[test]
fn test_find_by_path() {
    let json_str = r#"
        {
            "result": {
                "data": {
                    "value": 42
                }
            },
            "gas": {
                "price": 10
            },
            "some_string": "hello",
            "some_number": 123,
            "some_boolean": true,
            "some_null": null
        }
    "#;

    let json: Value = serde_json::from_str(json_str).expect("Failed to parse JSON");

    let path = vec!["some_string"];
    
    let v = find_value_by_path(&json, &path).unwrap();

    println!("{:?}", v);


}

pub fn sort_dependencies(dependencies: &HashMap<String, HashSet<String>>) -> Result<Vec<String>, error::PropertyError>{
    let mut sorted = Vec::new();
    let mut visited: HashSet<String> = HashSet::new();
    let mut marked: HashSet<String> = HashSet::new();

    for item in dependencies.keys(){
        if !visited.contains(item){
            if !visit(item, dependencies, &mut visited, &mut marked, &mut sorted){
                return Err(error::PropertyError::CyclicDependencies);
            }
        }
    }
    Ok(sorted)
}

fn visit(item: &String, dependencies: &HashMap<String, HashSet<String>>, visited: &mut HashSet<String>, marked: &mut HashSet<String>, sorted: &mut Vec<String>) -> bool{
    if marked.contains(item){
        return false;
    }
    if !visited.contains(item){
        marked.insert(item.clone());
        for dep in dependencies.get(item).unwrap_or(&HashSet::new()){
            if !visit(dep, dependencies, visited, marked, sorted){
                return false;
            }
        }
        visited.insert(item.clone());
        marked.remove(item);
        sorted.push(item.clone());
    }

    true
}

/// Tokenize Function header
pub fn tokenize_function(s: String) -> Vec<String>{
    let mut tokens: Vec<String> = Vec::new();

    let mut current_token = String::new();
    let mut is_name = true;
    let mut in_parantheses = 0;
    for token in s.chars(){
        if token == '('{
            in_parantheses += 1;
        }
        if token == ')'{
            in_parantheses -= 1;
        }

        if token == '(' && is_name{
            is_name = false;
            tokens.push(current_token);
            current_token = String::new();
            continue;
        }

        // Check for params
        if token == ',' && in_parantheses == 1{
            tokens.push(current_token);
            current_token = String::new();
            continue;
        }
        current_token.push(token);
    }
    tokens.push(current_token[..current_token.len()-1].to_string());
    // println!("{:?}", tokens);
    tokens

}

#[test]
fn test_func_tokenizer(){
    let property = "get_balance($payer_address, $ethereum_block_number.as(hex))";
    let tokens = tokenize_function(property.to_string());
    println!("{:?}", tokens);
}

/// Tokenize the properties
pub fn tokenize(s: String) -> Vec<String>{
    let mut tokens: Vec<String> = Vec::new();

    // Iterate through the string and split on . if not inside parentheses
    let mut current_token = String::new();
    let mut in_parantheses = 0;
    let mut was_function = 0; // Check whether we passed the function
    let mut was_above = false;
    let mut back_to_zero = false;
    for token in s.chars(){
        if token == '.'{
            // Check if inside parentheses 
            if in_parantheses == 0 && was_function == 0{
                if back_to_zero{
                    was_function += 1;
                }
                tokens.push(current_token);
                current_token = String::new();
                continue;
            }  
        }
        if token == '('{
            in_parantheses += 1;
            was_above = true
        }

        if token == ')'{
            in_parantheses -= 1;
        }

        if in_parantheses == 0 && was_above{
            back_to_zero = true;
        }

        current_token.push(token);
    }
    tokens.push(current_token);
    tokens
}

#[test]
fn test_tokenizer(){
    let property = "ethereum.get_balance($payer_address, $ethereum_block_number.as(hex)).result";
    let tokens = tokenize(property.to_string());
    println!("{:?}", tokens);
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
    use ethnum::{u256, i256};
    use std::str::FromStr;


    let val = struct_from_json("D:/Masterarbeit/brigade/properties/test_definition2.json");
    println!("{:?}", val);
    let results = execute_custom_function(&val).unwrap();
    println!("Result {:?}", results);

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

    let var: u256 = get_var!(u256 "balance_after").unwrap();

    println!("{:?}", var);

}