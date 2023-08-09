use std::fs;
use std::path::PathBuf;

use serde::Deserialize;
use serde::Serialize;

use super::ast::*;

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DefinitionFile {
    pub event: String,
    #[serde(rename = "chain_name")]
    pub chain_name: String,
    pub properties: DefinitionProperties,
    pub pattern: Vec<String>,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DefinitionProperties {
    #[serde(rename = "list_of_accounts")]
    pub list_of_accounts: String,
    #[serde(rename = "additional_variables")]
    pub additional_variables: String,
}

pub struct DefinitionParser{
    pub(crate) abstract_syntax_tree: Vec<ASTNode>,
    pub(crate) root: ASTNode,
}

impl DefinitionParser{
    pub fn from_path(path: PathBuf) -> Result<Self, &'static str>{
        let f = fs::read_to_string(path).unwrap();
        let def_file: DefinitionFile = serde_json::from_str(f.as_str()).unwrap();

        // Join strings into a single string for parsing the tree
        let pattern = def_file.pattern.join(" && ");

        let (ast, root) = build_ast!(pattern);
        Ok(Self{
            abstract_syntax_tree: ast,
            root: root,
        })
    }
}

#[test]
fn test_pattern(){
    let def = DefinitionParser::from_path(PathBuf::from("properties\\test_definition.json")).unwrap();
    let val = def.root.evaluate().unwrap();

    let (const_type, value) = val.get_constant_info();
    println!("{}: {}", const_type, value);
    assert_eq!(const_type, "Bool");
    assert_eq!(value, "true");
}