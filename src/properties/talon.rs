use std::borrow::BorrowMut;
use std::path::Path;
use std::fs;

#[derive(Debug, Clone)]
pub struct TalonFile {
    pub(crate)name: String,
    pub(crate)event: String,
    pub(crate)rules: String,
}

impl TalonFile {
    
    /// Build a new TalonFile
    pub fn new(name: &str, event: &str, rules: &str) -> Self {
        Self {
            name: name.to_string(),
            event: event.to_string(),
            rules: rules.to_string(),
        }
    }

    pub fn read(&self) -> Result<String, String> {
        Ok(format!("{}{}", self.event, self.rules))
    }

    pub fn read_from_file(path: &Path) -> Result<Self, &'static str> {
        let f = fs::read_to_string(path).expect("Unable to read file");
        let name  = path.file_name().unwrap().to_str().unwrap().to_string();
        let lines = f.split("\n");
        let mut event = "".to_string();
        let mut is_rule = false;
        let mut code_lines = vec![];
        for mut l in lines {
            println!("{}", l);
            if l.contains("event"){
                event = l.replace(r#"event":"#, "");
            }
            if l.contains("{"){
                is_rule = true;
                continue;
            }

            if l.contains("}"){
                is_rule = false;
                break;
            }

            if is_rule{
                code_lines.push(l);
            }
        }

        let rules = code_lines.join("\n");

        Ok(Self { event: event.to_string(), rules: rules, name: name })
        
    }

}

#[test]
fn test_talon_file(){
    let path = Path::new("/run/media/pascal/Volume/Fedora/Brigade/cross_chain_bug_mitigation/rules/generic_proof_creation.talon");
    println!("{}", path.display());
    let t = TalonFile::read_from_file(path).unwrap();
    println!("{:?}", t);

    assert_eq!(t.event, "ProofCreated(bytes32,address)");

}