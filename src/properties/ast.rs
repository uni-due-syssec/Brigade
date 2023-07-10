use std::collections::VecDeque;

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

    pub fn from_str(string: &str) -> Result<LogicOperator, &'static str> {
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
            _ => Err("Invalid Logic Operator"),
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

    pub fn from_str(string: &str) -> Result<ArithmeticOperator, &'static str> {
        match string {
            "+" => Ok(ArithmeticOperator::Add),
            "-" => Ok(ArithmeticOperator::Subtract),
            "*" => Ok(ArithmeticOperator::Multiply),
            "/" => Ok(ArithmeticOperator::Divide),
            "%" => Ok(ArithmeticOperator::Modulo),
            "neg" => Ok(ArithmeticOperator::Negate),
            _ => Err("Invalid Arithmetic Operator"),
        }
    }
}

/// AST Node which contains leaves and branches for an Abstract Syntax Tree
#[derive(Debug, Clone)]
pub enum ASTNode {
    ConstantBool(bool),
    ConstantNumber(f64),
    ConstantString(String),
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
}

impl ASTNode {
    pub fn evaluate(&self) -> Result<ASTConstant, &'static str> {
        match self {
            ASTNode::ConstantBool(value) => Ok(ASTConstant::Bool(*value)),
            ASTNode::ConstantNumber(value) => Ok(ASTConstant::Number(*value)),
            ASTNode::ConstantString(value) => Ok(ASTConstant::String(value.clone())),
            ASTNode::UnaryArithmetic(operator, value) => { // Implementation of Unary Arithmetic Operations
                let val = value.evaluate()?;
                match operator { 
                    ArithmeticOperator::Negate => {
                        match val {
                            ASTConstant::Number(value) => Ok(ASTConstant::Number(-value)),
                            _ => Err("Not Implemented"),
                        }
                    },
                    _ => Err("Not Implemented"),
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
                                    _ => Err("Not Implemented"),
                                }
                            },
                            _ => Err("Not Implemented"),
                        }
                    }
                    _ => Err("Not Implemented"),
                }
            },
            ASTNode::UnaryLogic(operator, value) => {
                let val = value.evaluate()?;
                match val {
                    ASTConstant::Bool(value) => {
                        match operator {
                        LogicOperator::Not => Ok(ASTConstant::Bool(!value)),
                        _ => Err("Not Implemented"),
                        }
                    },
                    _ => Err("Not Implemented"),
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
                                    _ => Err("Not Implemented"),
                                }
                            },
                            _ => Err("Not Implemented"),
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
                                    _ => Err("Not Implemented"),
                                }
                            },
                            _ => Err("Not Implemented"),
                        }
                    },
                    ASTConstant::String(left) => {
                        match right {
                            ASTConstant::String(right) => {
                                match operator {
                                    LogicOperator::Equal => Ok(ASTConstant::Bool(left == right)),
                                    LogicOperator::NotEqual => Ok(ASTConstant::Bool(left != right)),
                                    _ => Err("Not Implemented"),
                                }
                            },
                            _ => Err("Not Implemented"),
                        }
                    }
                }
            }
            _ => Err("Not Implemented"),
        }
    }
}

macro_rules! build_ast {
    ($str_pattern:expr) => {
        let split_string = $str_pattern.split(" ").collect::<Vec<&str>>();
        let mut ast_vec: Vec<ASTNode> = vec![];
        for s in split_string.iter() {
            println!("{}", s);
            match s.parse::<i64>() {
                Ok(value) => {
                    ast_vec.push(ASTNode::ConstantNumber(value as f64));
                },
                Err(_) => {
                    println!("{} is not a number", s);
                    
                }
            }
        }
    };
}

fn parse_postfix(tokens: VecDeque<String>) -> Result<Vec<ASTNode>, &'static str>{
    let mut ast_vec: Vec<ASTNode> = vec![];
    let mut stack: Vec<ASTNode> = vec![];

    for token in tokens {
        if is_operator(token.as_str()) {
            todo!();
        }else{ // Parse Operator in respective type
            match token.parse::<i64>() {
                Ok(value) => {
                    let node = ASTNode::ConstantNumber(value as f64);
                    ast_vec.push(node.clone());
                    stack.push(node);
                },
                Err(_) => {
                    println!("{} is not a number", token);
                    match token.parse::<bool>(){
                        Ok(value) => {
                            let node = ASTNode::ConstantBool(value);
                        },
                        Err(_) => {
                            println!("{} is not a boolean", token);
                            let node: ASTNode = ASTNode::ConstantString(token);
                        }
                    }
                },
            }
        }
    }
    Ok(ast_vec)
}

fn shunting_yard_algorithm(tokens: Vec<String>) -> Result<VecDeque<String>, &'static str> {
    let mut stack: Vec<String> = vec![]; // Stack for operators
    let mut output_queue: VecDeque<String> = VecDeque::new();

    // for token in tokens.iter(){
    //     println!("Token: {}", token);
    //     if !is_operator(token){ // Operands directly to output queue
    //         println!("Token {} to Ouput", token);
    //         output_queue.push_back(token.to_string());
    //     }else { // Process Operators
    //         if token == "(" { // If token is opening parenthesis, push to stack
    //             println!("Pushed ( to Stack");
    //             stack.push(token.to_string());
    //         }else if token == ")" { // If token is closing parenthesis
    //             while let Some(top) = stack.pop() {
    //                 if top == "(" {
    //                     println!("Discarded (");
    //                     break;
    //                 } else {
    //                     output_queue.push_back(top);
    //                 }
    //             }
    //         }else{ // Process Operators
    //             while let Some(top) = stack.last() {
    //                 if let Some(token_precedence) = operator_precedence(token) {
    //                     if let Some(top_precedence) = operator_precedence(top) {
    //                         if token_precedence > top_precedence {
    //                             break;
    //                         }
    //                     }
    //                 }
    //                 output_queue.push_back(stack.pop().unwrap());
    //             }
    //             stack.push(token.to_string());
    //         }
    //     }
    // }

    // // Empty Stack
    // for token in stack {
    //     if token == "(" || token == ")" {
    //         return Err("Unmatched Parentheses: Left on Stack");
    //     }
    //     output_queue.push_back(token.clone());
    //     println!("Token {} to Ouput", token);
    // }
    // println!("Output Queue: {:?}", output_queue);
    

    for token in tokens.iter() {
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
                    while !stack.is_empty() && is_operator(stack.last().unwrap()) && operator_precedence(token) >= operator_precedence(stack.last().unwrap()) {
                        output_queue.push_back(stack.pop().unwrap());
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
    build_ast!("5 + 5 > 17 - 15");
}

#[test]
fn test_shunting_yard(){
    let tokens = vec!["5".to_owned(), "+".to_owned(), "5".to_owned(), ">".to_owned(), "17".to_owned(), "-".to_owned(), "(".to_owned(), "5".to_owned(), "-".to_owned(), "neg".to_owned(), "10".to_owned(), ")".to_owned()];

    let output = shunting_yard_algorithm(tokens);
    let output = output.unwrap();
    for o in output.iter(){
        print!("{}", o);
    }
    println!("");
    
    let expected_out = ["5", "5", "17", "+", "5", "10", "+", "-"];
}