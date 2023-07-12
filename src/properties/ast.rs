use std::collections::VecDeque;

use super::error::{self, ASTError};

use super::environment::{get_variable, VariableMap, get_variable_map_instance};

/// This file describes an Abstract Syntax Tree which should contain as leaves constants and the branches refer to logical or arithmetic operators.


/// A Logical Operator which refers to Logical Statements returning either true or false
#[derive(Debug, Clone, PartialEq)]
pub enum LogicOperator {
    Greater,
    Less,
    GreaterOrEqual,
    LessOrEqual,
    Equal,
    NotEqual,
    And,
    Or,
    Not,
}

impl LogicOperator {
    pub fn to_string(&self) -> &str {
        match self {
            LogicOperator::Greater => ">",
            LogicOperator::Less => "<",
            LogicOperator::GreaterOrEqual => ">=",
            LogicOperator::LessOrEqual => "<=",
            LogicOperator::Equal => "==",
            LogicOperator::NotEqual => "!=",
            LogicOperator::And => "&&",
            LogicOperator::Or => "||",
            LogicOperator::Not => "!",
        }
    }

    pub fn from_str(string: &str) -> Result<LogicOperator, ASTError> {
        match string {
            ">" => Ok(LogicOperator::Greater),
            "<" => Ok(LogicOperator::Less),
            ">=" => Ok(LogicOperator::GreaterOrEqual),
            "<=" => Ok(LogicOperator::LessOrEqual),
            "==" => Ok(LogicOperator::Equal),
            "!=" => Ok(LogicOperator::NotEqual),
            "&&" => Ok(LogicOperator::And),
            "||" => Ok(LogicOperator::Or),
            "!" => Ok(LogicOperator::Not),
            _ => Err(ASTError::InvalidLogicOperator(string.to_owned())),
        }
    }
}

/// An Arithmetic Operator which refers to Statements returning a constant value
#[derive(Debug, Clone, PartialEq)]
pub enum ArithmeticOperator {
    Add,
    Subtract,
    Multiply,
    Divide,
    Modulo,
    Negate,
}

impl ArithmeticOperator{
    pub fn to_string(&self) -> &str {
        match self {
            ArithmeticOperator::Add => "+",
            ArithmeticOperator::Subtract => "-",
            ArithmeticOperator::Multiply => "*",
            ArithmeticOperator::Divide => "/",
            ArithmeticOperator::Modulo => "%",
            ArithmeticOperator::Negate => "neg",
        }
    }

    pub fn from_str(string: &str) -> Result<ArithmeticOperator, ASTError> {
        match string {
            "+" => Ok(ArithmeticOperator::Add),
            "-" => Ok(ArithmeticOperator::Subtract),
            "*" => Ok(ArithmeticOperator::Multiply),
            "/" => Ok(ArithmeticOperator::Divide),
            "%" => Ok(ArithmeticOperator::Modulo),
            "neg" => Ok(ArithmeticOperator::Negate),
            _ => Err(ASTError::InvalidArithmeticOperator(string.to_owned())),
        }
    }
}

/// AST Node which contains leaves and branches for an Abstract Syntax Tree
#[derive(Debug, Clone)]
pub enum ASTNode {
    ConstantBool(bool),
    ConstantNumber(f64),
    ConstantString(String),
    Variable(String, &'static VariableMap), // String points to a variable on the VariableMap
    UnaryArithmetic(ArithmeticOperator, Box<ASTNode>),
    BinaryArithmetic(ArithmeticOperator, Box<ASTNode>, Box<ASTNode>),
    UnaryLogic(LogicOperator, Box<ASTNode>),
    BinaryLogic(LogicOperator, Box<ASTNode>, Box<ASTNode>),
}

/// AST Value which contains a constant value
#[derive(Debug, Clone, PartialEq)]
pub enum ASTConstant {
    Bool(bool),
    Number(f64),
    String(String),
}

impl ASTConstant{
    pub fn get_constant_info(&self) -> (&str, String) {
        match self {
            ASTConstant::Bool(value) => ("Bool", value.to_string()),
            ASTConstant::Number(value) => ("Number", value.to_string()),
            ASTConstant::String(value) => ("String", value.clone()),
        }
    }

    pub fn parse(value: String) -> Self {
        match value.parse::<f64>() {
            Ok(value) => ASTConstant::Number(value),
            Err(_) => {
                match value.parse::<bool>() {
                    Ok(value) => ASTConstant::Bool(value),
                    Err(_) => ASTConstant::String(value),
                }
            },
        }
    }
}

impl ASTNode {
    pub fn evaluate(&self) -> Result<ASTConstant, ASTError> {
        match self {
            ASTNode::ConstantBool(value) => Ok(ASTConstant::Bool(*value)),
            ASTNode::ConstantNumber(value) => Ok(ASTConstant::Number(*value)),
            ASTNode::ConstantString(value) => Ok(ASTConstant::String(value.clone())),
            ASTNode::Variable(name, map_ref) => {

                match get_variable(map_ref, name) {
                    Some(value) => {
                        println!("{:?}", value);
                        Ok(ASTConstant::parse(value))
                    },
                    None => Err(ASTError::VariableNotFound { var: name.clone() }),
                }
            },
            ASTNode::UnaryArithmetic(operator, value) => { // Implementation of Unary Arithmetic Operations
                let val = value.evaluate()?;
                match operator { 
                    ArithmeticOperator::Negate => {
                        match val {
                            ASTConstant::Number(value) => Ok(ASTConstant::Number(-value)),
                            _ => Err(ASTError::InvalidOperation(ArithmeticOperator::Negate.to_string().to_owned(), "bool".to_owned(), "string".to_owned())),
                        }
                    },
                    _ => Err(ASTError::InvalidUnaryOperator),
                }
            }
            ASTNode::BinaryArithmetic(operator, left, right) => { // Implementation of Binary Arithmetic Operations
                let left = left.evaluate()?;
                let right = right.evaluate()?;
                match left {
                    ASTConstant::Number(left) => {
                        match right {
                            ASTConstant::Number(right) => {
                                match operator {
                                    ArithmeticOperator::Add => Ok(ASTConstant::Number(left + right)),
                                    ArithmeticOperator::Subtract => Ok(ASTConstant::Number(left - right)),
                                    ArithmeticOperator::Multiply => Ok(ASTConstant::Number(left * right)),
                                    ArithmeticOperator::Divide => Ok(ASTConstant::Number(left / right)),
                                    ArithmeticOperator::Modulo => Ok(ASTConstant::Number(left % right)),
                                    _ => Err(ASTError::InvalidArithmeticOperator(operator.to_string().to_owned())),
                                }
                            },
                            _ => Err(ASTError::InvalidConstant(operator.to_string().to_owned())),
                        }
                    }
                    _ => Err(ASTError::InvalidConstant(operator.to_string().to_owned())),
                }
            },
            ASTNode::UnaryLogic(operator, value) => {
                let val = value.evaluate()?;
                match val {
                    ASTConstant::Bool(value) => {
                        match operator {
                        LogicOperator::Not => Ok(ASTConstant::Bool(!value)),
                        _ => Err(ASTError::InvalidUnaryOperator),
                        }
                    },
                    _ => Err(ASTError::InvalidConstant(operator.to_string().to_owned())),
                }
            },
            ASTNode::BinaryLogic(operator, left, right) => {
                let left = left.evaluate()?;
                let right = right.evaluate()?;
                match left {
                    ASTConstant::Bool(left) => {
                        match right {
                            ASTConstant::Bool(right) => {
                                match operator {
                                    LogicOperator::And => Ok(ASTConstant::Bool(left && right)),
                                    LogicOperator::Or => Ok(ASTConstant::Bool(left || right)),
                                    LogicOperator::Equal => Ok(ASTConstant::Bool(left == right)),
                                    LogicOperator::NotEqual => Ok(ASTConstant::Bool(left != right)),
                                    _ => Err(ASTError::InvalidBinaryOperator),
                                }
                            },
                            _ => Err(ASTError::InvalidConstant(operator.to_string().to_owned())),
                        }
                    },
                    ASTConstant::Number(left) => {
                        match right {
                            ASTConstant::Number(right) => {
                                match operator {
                                    LogicOperator::Equal => Ok(ASTConstant::Bool(left == right)),
                                    LogicOperator::NotEqual => Ok(ASTConstant::Bool(left != right)),
                                    LogicOperator::Greater => Ok(ASTConstant::Bool(left > right)),
                                    LogicOperator::Less => Ok(ASTConstant::Bool(left < right)),
                                    LogicOperator::GreaterOrEqual => Ok(ASTConstant::Bool(left >= right)),
                                    LogicOperator::LessOrEqual => Ok(ASTConstant::Bool(left <= right)),
                                    _ => Err(ASTError::InvalidBinaryOperator),
                                }
                            },
                            _ => Err(ASTError::InvalidConstant(operator.to_string().to_owned())),
                        }
                    },
                    ASTConstant::String(left) => {
                        match right {
                            ASTConstant::String(right) => {
                                match operator {
                                    LogicOperator::Equal => Ok(ASTConstant::Bool(left == right)),
                                    LogicOperator::NotEqual => Ok(ASTConstant::Bool(left != right)),
                                    _ => Err(ASTError::InvalidBinaryOperator),
                                }
                            },
                            _ => Err(ASTError::InvalidConstant(operator.to_string().to_owned())),
                        }
                    }
                }
            }
        }
    }

    fn format(&self) -> String {
        match self {
            ASTNode::ConstantBool(value) => value.to_string(),
            ASTNode::ConstantNumber(value) => value.to_string(),
            ASTNode::ConstantString(value) => value.clone(),
            ASTNode::Variable(name, map_ref) => get_variable(map_ref, name).unwrap().to_string(),
            ASTNode::UnaryArithmetic(operator, value) => {
                format!("\t{}\t\n{}", operator.to_string(), value.format())
            },
            ASTNode::BinaryArithmetic(operator, left, right) => {
                format!("\t{}\n{}\t\t{}", operator.to_string(), left.format(), right.format())
            },
            ASTNode::UnaryLogic(operator, value) => {
                format!("\t{}\t\n{}", operator.to_string(), value.format())
            },
            ASTNode::BinaryLogic(operator, left, right) => {
                format!("\t{}\n{}\t\t{}", operator.to_string(), left.format(), right.format())
            }
        }
    }

}

/// Build the tree from a Vec of tokens and return the AST and the root node
fn parse_postfix(tokens: VecDeque<String>) -> Result<(Vec<ASTNode>, ASTNode), &'static str>{
    let mut ast_vec: Vec<ASTNode> = vec![];
    let mut stack: Vec<ASTNode> = vec![];

    for token in tokens {
        if is_operator(token.as_str()) {
            match ArithmeticOperator::from_str(token.as_str()) {
                Ok(value) => {
                    match value {
                        ArithmeticOperator::Negate => {
                            let node = ASTNode::UnaryArithmetic(ArithmeticOperator::Negate, Box::new(stack.pop().unwrap()));
                            ast_vec.push(node.clone());
                            stack.push(node);
                        },
                        ArithmeticOperator::Add => {
                            let right = stack.pop().unwrap();
                            let left = stack.pop().unwrap();
                            let node = ASTNode::BinaryArithmetic(ArithmeticOperator::Add, Box::new(left), Box::new(right));
                            ast_vec.push(node.clone());
                            stack.push(node);
                        },
                        ArithmeticOperator::Subtract => {
                            let right = stack.pop().unwrap();
                            let left = stack.pop().unwrap();
                            let node = ASTNode::BinaryArithmetic(ArithmeticOperator::Subtract, Box::new(left), Box::new(right));
                            ast_vec.push(node.clone());
                            stack.push(node);
                        },
                        ArithmeticOperator::Multiply => {
                            let right = stack.pop().unwrap();
                            let left = stack.pop().unwrap();
                            let node = ASTNode::BinaryArithmetic(ArithmeticOperator::Multiply, Box::new(left), Box::new(right));
                            ast_vec.push(node.clone());
                            stack.push(node);
                        },
                        ArithmeticOperator::Divide => {
                            let right = stack.pop().unwrap();
                            let left = stack.pop().unwrap();
                            let node = ASTNode::BinaryArithmetic(ArithmeticOperator::Divide, Box::new(left), Box::new(right));
                            ast_vec.push(node.clone());
                            stack.push(node);
                        },
                        ArithmeticOperator::Modulo => {
                            let right = stack.pop().unwrap();
                            let left = stack.pop().unwrap();
                            let node = ASTNode::BinaryArithmetic(ArithmeticOperator::Modulo, Box::new(left), Box::new(right));
                            ast_vec.push(node.clone());
                            stack.push(node);
                        }
                    }
                },
                Err(_) => {
                    //println!("{} is not an Arithmetic Operator", token);
                    match LogicOperator::from_str(token.as_str()) {
                        Ok(value) => {
                            match value {
                                LogicOperator::Not => {
                                    let node = ASTNode::UnaryLogic(LogicOperator::Not, Box::new(stack.pop().unwrap()));
                                    ast_vec.push(node.clone());
                                    stack.push(node);
                                },
                                LogicOperator::And => {
                                    let right = stack.pop().unwrap();
                                    let left = stack.pop().unwrap();
                                    let node = ASTNode::BinaryLogic(LogicOperator::And, Box::new(left), Box::new(right));
                                    ast_vec.push(node.clone());
                                    stack.push(node);
                                },
                                LogicOperator::Or => {
                                    let right = stack.pop().unwrap();
                                    let left = stack.pop().unwrap();
                                    let node = ASTNode::BinaryLogic(LogicOperator::Or, Box::new(left), Box::new(right));
                                    ast_vec.push(node.clone());
                                    stack.push(node);
                                },
                                LogicOperator::Equal => {
                                    let right = stack.pop().unwrap();
                                    let left = stack.pop().unwrap();
                                    let node = ASTNode::BinaryLogic(LogicOperator::Equal, Box::new(left), Box::new(right));
                                    ast_vec.push(node.clone());
                                    stack.push(node);
                                },
                                LogicOperator::NotEqual => {
                                    let right = stack.pop().unwrap();
                                    let left = stack.pop().unwrap();
                                    let node = ASTNode::BinaryLogic(LogicOperator::NotEqual, Box::new(left), Box::new(right));
                                    ast_vec.push(node.clone());
                                    stack.push(node);
                                },
                                LogicOperator::Greater => {
                                    let right = stack.pop().unwrap();
                                    let left = stack.pop().unwrap();
                                    let node = ASTNode::BinaryLogic(LogicOperator::Greater, Box::new(left), Box::new(right));
                                    ast_vec.push(node.clone());
                                    stack.push(node);
                                },
                                LogicOperator::Less => {
                                    let right = stack.pop().unwrap();
                                    let left = stack.pop().unwrap();
                                    let node = ASTNode::BinaryLogic(LogicOperator::Less, Box::new(left), Box::new(right));
                                    ast_vec.push(node.clone());
                                    stack.push(node);
                                },
                                LogicOperator::GreaterOrEqual => {
                                    let right = stack.pop().unwrap();
                                    let left = stack.pop().unwrap();
                                    let node = ASTNode::BinaryLogic(LogicOperator::GreaterOrEqual, Box::new(left), Box::new(right));
                                    ast_vec.push(node.clone());
                                    stack.push(node);
                                },
                                LogicOperator::LessOrEqual => {
                                    let right = stack.pop().unwrap();
                                    let left = stack.pop().unwrap();
                                    let node = ASTNode::BinaryLogic(LogicOperator::LessOrEqual, Box::new(left), Box::new(right));
                                    ast_vec.push(node.clone());
                                    stack.push(node);
                                }
                            }
                        },
                        Err(_) => {
                            println!("{} is not an Logic Operator", token);
                        }
                    }
                },
            }
        }else{ // Parse Operand in respective type
            if token.starts_with("$"){ // If the token starts with $ a variable is used
                let node = ASTNode::Variable(token[1..].to_string(), get_variable_map_instance());
                ast_vec.push(node.clone());
                stack.push(node);
            }else{
                match token.parse::<f64>() {
                    Ok(value) => {
                        let node = ASTNode::ConstantNumber(value);
                        ast_vec.push(node.clone());
                        stack.push(node);
                    },
                    Err(_) => {
                        //println!("{} is not a number", token);
                        match token.parse::<bool>(){
                            Ok(value) => {
                                let node = ASTNode::ConstantBool(value);
                                ast_vec.push(node.clone());
                                stack.push(node);
                            },
                            Err(_) => {
                                //println!("{} is not a boolean", token);
                                let node: ASTNode = ASTNode::ConstantString(token);
                                ast_vec.push(node.clone());
                                stack.push(node);
                            }
                        }
                    },
                }
            }
        }
    }

    let root = stack.pop().unwrap();

    Ok((ast_vec, root))
}

/// The shunting yard algorithm by Dijkstra transforms the infix logic expression into postfix.
fn shunting_yard_algorithm(tokens: Vec<String>) -> Result<VecDeque<String>, &'static str> {
    let mut stack: Vec<String> = vec![]; // Stack for operators
    let mut output_queue: VecDeque<String> = VecDeque::new();

    for token in tokens.iter() {
        println!("Stack: {:?}", stack);
        println!("Output Queue: {:?}", output_queue);
        if !is_operator(token){
            output_queue.push_back(token.clone());
        }else{
            match token.as_str() {
                "(" => {
                    stack.push(token.to_string());
                },
                ")" => {
                    while stack.last() != Some(&"(".to_owned()) {
                        output_queue.push_back(stack.pop().unwrap());
                        if stack.is_empty() {
                            return Err("Unmatched Parentheses: Empty Stack. No preceeding parentheses");
                        }
                    }
                    stack.pop(); // Remove parenthesis
                },
                _ => {

                    while !stack.is_empty() {
                        if stack.last().unwrap() == "(" {
                            break;
                        }
                        let op_precedence = operator_precedence(stack.last().unwrap());
                        let self_precedence = operator_precedence(token);
                        //println!("op_precedence: {:?}, self_precedence: {:?}", op_precedence, self_precedence);
                        if op_precedence >= self_precedence {
                            output_queue.push_back(stack.pop().unwrap());
                        }else{
                            break;
                        }
                    }
                    stack.push(token.to_string());
                }
            }
        }
    }

    while !stack.is_empty() {
        let val = stack.pop().unwrap();
        if val == "(" {
            return Err("Unmatched Parentheses: Leftover on Stack. More parentheses are opening than closing");
        }
        output_queue.push_back(val);
    }
    println!("Output Queue: {:?}", output_queue);

    Ok(output_queue)
}

/// Return the precedence level of the operator
fn operator_precedence(operator: &str) -> Option<u8> {
    match operator {
        "||" => Some(0),                        // Or
        "&&" => Some(1),                        // And
        "==" | "!=" => Some(2),                 // Equality
        "<" | ">" | "<=" | ">=" => Some(3),     // Comparison
        "+" | "-" => Some(4),                   // Addition, Subtraction
        "*" | "/" | "%" => Some(5),             // Multiplication, Division, and Modulo
        "!" | "neg" => Some(6),                 // Unary Operators
        "(" | ")" => Some(7),                   // Parentheses is highest level
        _ => None,                              // No precedence for other operators
    }
}


/// Is the token an Operator
fn is_operator(token: &str) -> bool{
    token == "+" || token == "-" || token == "*" || token == "/" || token == "%"
    || token == "||" || token == "&&" || token == "==" || token == "!=" || token == "!"
    || token == "<" || token == "<=" || token == ">" || token == ">=" || token == "(" 
    || token == ")" || token == "neg"
}

/// This macro builds an AST from a string
/// The input is a string with the infix notation separated by spaces
/// The return type is a tuple with the first entry being the full AST and the second entry being the root node
macro_rules! build_ast {
    ($str_pattern:expr) => {    
        parse_postfix(shunting_yard_algorithm($str_pattern.split(" ").map(|s| s.to_string()).collect::<Vec<String>>()).unwrap()).unwrap()
    };
}


#[test]
fn test_ast(){
    // Example: 5 + 5 > 17 - 15

    let exp_a = ASTNode::ConstantNumber(5.0);
    let exp_b = ASTNode::ConstantNumber(5.0);
    let exp_a_b = ASTNode::BinaryArithmetic(ArithmeticOperator::Add, Box::new(exp_a), Box::new(exp_b));

    let exp_c = ASTNode::ConstantNumber(17.0);
    let exp_d = ASTNode::ConstantNumber(15.0);
    let exp_c_d = ASTNode::BinaryArithmetic(ArithmeticOperator::Subtract, Box::new(exp_c), Box::new(exp_d));

    let exp_g = ASTNode::BinaryLogic(LogicOperator::Greater, Box::new(exp_a_b), Box::new(exp_c_d));
    
    let val = exp_g.evaluate().unwrap();
    let(const_type, value) = val.get_constant_info();
    println!("{}: {}", const_type, value);

    assert_eq!(value, "true");
}

#[test]
fn test_ast_macro(){
    let (ast, root) = build_ast!("5 == 5");

    let val = root.evaluate().unwrap();
    let(const_type, value) = val.get_constant_info();
    println!("{}: {}", const_type, value);
}

#[test]
fn test_shunting_yard(){
    // 5 + 5 > 17 - (5 - neg 10)
    let tokens = vec!["5".to_owned(), "+".to_owned(), "5".to_owned(), ">".to_owned(), "17".to_owned(), "-".to_owned(), "(".to_owned(), "5".to_owned(), "-".to_owned(), "neg".to_owned(), "10".to_owned(), ")".to_owned()];

    //let tokens: Vec<String> = vec!["5".to_owned(), ">".to_owned(), "(".to_owned(), "6".to_owned(), "+".to_owned(), "5".to_owned(), ")".to_owned()];

    let output = shunting_yard_algorithm(tokens);
    let output = output.unwrap();
    println!("Output: {:?}", output);
    for o in output.iter(){
        print!("{}", o);
    }
    println!("");
    
    let (ast, root) = parse_postfix(output).unwrap();

    println!("Root: {}", root.format());

    let val = root.evaluate().unwrap();
    let (const_type, value) = val.get_constant_info();

    assert_eq!(value, "false");
}

#[test]
fn test_all_operations(){
    // Greater
    let (ast, root) = build_ast!("5 > 4");

    let val = root.evaluate().unwrap();
    let(const_type, value) = val.get_constant_info();
    println!("Greater: {}: {}", const_type, value);
    assert_eq!(const_type, "Bool");
    assert_eq!(value, "true");

    // Less
    let (ast, root) = build_ast!("3 < 4");

    let val = root.evaluate().unwrap();
    let(const_type, value) = val.get_constant_info();
    println!("Less: {}: {}", const_type, value);
    assert_eq!(const_type, "Bool");
    assert_eq!(value, "true");

    // GreaterOrEqual
    let (ast, root) = build_ast!("5 >= 4");

    let val = root.evaluate().unwrap();
    let(const_type, value) = val.get_constant_info();
    println!("GreaterOrEqual: {}: {}", const_type, value);
    assert_eq!(const_type, "Bool");
    assert_eq!(value, "true");

    // LessOrEqual
    let (ast, root) = build_ast!("4 <= 4");

    let val = root.evaluate().unwrap();
    let(const_type, value) = val.get_constant_info();
    println!("LessOrEqual: {}: {}", const_type, value);
    assert_eq!(const_type, "Bool");
    assert_eq!(value, "true");

    // Equal
    let (ast, root) = build_ast!("5 == 5");

    let val = root.evaluate().unwrap();
    let(const_type, value) = val.get_constant_info();
    println!("Equal: {}: {}", const_type, value);
    assert_eq!(const_type, "Bool");
    assert_eq!(value, "true");

    // NotEqual
    let (ast, root) = build_ast!("5 != 5");

    let val = root.evaluate().unwrap();
    let(const_type, value) = val.get_constant_info();
    println!("NotEqual: {}: {}", const_type, value);
    assert_eq!(const_type, "Bool");
    assert_eq!(value, "false");

    // Not
    let (ast, root) = build_ast!("! true");
    
    let val = root.evaluate().unwrap();
    let(const_type, value) = val.get_constant_info();
    println!("Not: {}: {}", const_type, value);
    assert_eq!(const_type, "Bool");
    assert_eq!(value, "false");

    // Add
    let (ast, root) = build_ast!("5.5 + 5.0");

    let val = root.evaluate().unwrap();
    let(const_type, value) = val.get_constant_info();
    println!("Add: {}: {}", const_type, value);
    assert_eq!(const_type, "Number");
    assert_eq!(value, "10.5");

    // Subtract
    let (ast, root) = build_ast!("5 - 5");

    let val = root.evaluate().unwrap();
    let(const_type, value) = val.get_constant_info();
    println!("Subtract: {}: {}", const_type, value);
    assert_eq!(const_type, "Number");
    assert_eq!(value, "0");

    // Multiply
    let (ast, root) = build_ast!("5 * 5");

    let val = root.evaluate().unwrap();
    let(const_type, value) = val.get_constant_info();
    println!("Multiply: {}: {}", const_type, value);
    assert_eq!(const_type, "Number");
    assert_eq!(value, "25");

    // Divide
    let (ast, root) = build_ast!("5 / 5");

    let val = root.evaluate().unwrap();
    let(const_type, value) = val.get_constant_info();
    println!("Divide: {}: {}", const_type, value);
    assert_eq!(const_type, "Number");
    assert_eq!(value, "1");

    // Modulo
    let (ast, root) = build_ast!("5 % 5");

    let val = root.evaluate().unwrap();
    let(const_type, value) = val.get_constant_info();
    println!("Modulo: {}: {}", const_type, value);
    assert_eq!(const_type, "Number");
    assert_eq!(value, "0");

    // Negate
    let (ast, root) = build_ast!("neg 5");

    let val = root.evaluate().unwrap();
    let(const_type, value) = val.get_constant_info();
    println!("Negate: {}: {}", const_type, value);
    assert_eq!(const_type, "Number");
    assert_eq!(value, "-5");

}

#[test]
fn test_complex_ast(){
    // Test combination of Arithmetic and Logic Operators
    let (ast, root) = build_ast!("5 + 5 > 7");


    // 5 + 5 > 7
    // 5 -> Output Queue [5] Stack []
    // + -> Output Queue [5] Stack [+]
    // 5 -> Output Queue [5, 5] Stack [+]
    // > -> Output Queue [5, 5, +] Stack [>]
    // 7 -> Output Queue [5, 5, +, 7] Stack [>]
    // [5, 5, +, 7, >]

    // 5 + 5 > 7
    // 5 -> Output Queue [5] Stack []
    // + -> Output Queue [5] Stack [+]
    // 5 -> Output Queue [5, 5] Stack [+]
    // > -> Output Queue [5, 5] Stack [+, >]
    // 7 -> Output Queue [5, 5, 7] Stack [+, >]
    // [5, 5, 7, >, +]


    let val = root.evaluate().unwrap();
    let(const_type, value) = val.get_constant_info();
    println!("Add Greater: {}: {}", const_type, value);
    assert_eq!(const_type, "Bool");
    assert_eq!(value, "true");
}

#[test]
fn test_more_complex_ast(){
    let (ast, root) = build_ast!("( 17 * 3 ) % 10 == 1");

    let val = root.evaluate().unwrap();
    let(const_type, value) = val.get_constant_info();
    println!("{}: {}", const_type, value);
    assert_eq!(const_type, "Bool");
    assert_eq!(value, "true");
}

#[test]
fn test_variables(){

    let map = get_variable_map_instance();
    map.insert("x".to_owned(), "5".to_owned());

    let (ast, root) = build_ast!("$x == 5");

    let val = root.evaluate().unwrap();
    let (const_type, value) = val.get_constant_info();
    println!("{}: {}", const_type, value);
    assert_eq!(const_type, "Bool");
    assert_eq!(value, "true");
}

#[test]
fn test_str_var(){
    let map = get_variable_map_instance();
    map.insert("x".to_owned(), "airport".to_owned());

    let (ast, root) = build_ast!("$x == milestone");
    
    map.remove("x");
    map.insert("x".to_owned(), "milestone".to_owned());

    let val = root.evaluate().unwrap();
    let (const_type, value) = val.get_constant_info();
    println!("{}: {}", const_type, value);
    assert_eq!(const_type, "Bool");
    assert_eq!(value, "true");
}