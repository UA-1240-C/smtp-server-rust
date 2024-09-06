use error_handler::{Error, JsonErrorType};

use tree_sitter::Node;
use std::ops::Index;
use std::collections::HashMap;

// Enum to represent JSON values
#[derive(Debug)]
pub enum JsonValue {
    Object(HashMap<String, JsonValue>),
    Array(Vec<JsonValue>),
    String(String),
    Number(f64),
    Bool(bool),
    Null,
}

impl JsonValue {
    pub fn as_str(&self) -> Option<&str> {
        if let JsonValue::String(s) = self {
            Some(s)
        } else {
            None
        }
    }

    pub fn as_number(&self) -> Option<f64> {
        if let JsonValue::Number(n) = self {
            Some(*n)
        } else {
            None
        }
    }

    pub fn as_array(&self) -> Option<&Vec<JsonValue>> {
        if let JsonValue::Array(arr) = self {
            Some(arr)
        } else {
            None
        }
    }

    pub fn as_object(&self) -> Option<&HashMap<String, JsonValue>> {
        if let JsonValue::Object(obj) = self {
            Some(obj)
        } else {
            None
        }
    }

    pub fn as_bool(&self) -> Option<bool> {
        if let JsonValue::Bool(b) = self {
            Some(*b)
        } else {
            None
        }
    }
}

// Implement indexing for JsonValue to use the [] operator
impl Index<&str> for JsonValue {
    type Output = JsonValue;

    fn index(&self, key: &str) -> &Self::Output {
        match self {
            JsonValue::Object(map) => map.get(key).unwrap_or(&JsonValue::Null),
            _ => &JsonValue::Null,
        }
    }
}

impl Index<usize> for JsonValue {
    type Output = JsonValue;

    fn index(&self, index: usize) -> &Self::Output {
        match self {
            JsonValue::Array(arr) => arr.get(index).unwrap_or(&JsonValue::Null),
            _ => &JsonValue::Null,
        }
    }
}

pub struct JsonParser {
    parser: tree_sitter::Parser,
}

impl JsonParser {
    pub fn parse(&mut self, code: &str) -> std::result::Result<JsonValue, Error> {
        let tree = self.parser.parse(code, None).ok_or(Error::JsonError(JsonErrorType::ParseError))?;
        let root_node = tree.root_node();

        let json_obj = root_node.child(0).ok_or(Error::JsonError(JsonErrorType::UnreachableChild))?;
        let json_value = Self::parse_json_node(json_obj, code)?;

        Ok(json_value)
    }

    pub fn parse_json_node(node: Node, code: &str) -> std::result::Result<JsonValue, Error> {
        match node.kind() {
            "object" => {
                let mut object = HashMap::new();
                let mut cursor = node.walk();
                if cursor.goto_first_child() {
                    loop {
                        if cursor.node().kind() == "pair" {
                            let pair_node = cursor.node();
                            let (key, value) = Self::parse_pair(pair_node, code)?;
                            object.insert(key, value);
                        }
                        if !cursor.goto_next_sibling() {
                            break;
                        }
                    }
                }
                Ok(JsonValue::Object(object))
            }
            "array" => {
                let mut array = Vec::new();
                let mut cursor = node.walk();
                if cursor.goto_first_child() {
                    loop {
                        let child_node = cursor.node();
                        if child_node.is_named() {
                            array.push(Self::parse_json_node(child_node, code)?);
                        }
                        if !cursor.goto_next_sibling() {
                            break;
                        }
                    }
                }
                Ok(JsonValue::Array(array))
            }
            "string" => {
                let value = &code[node.start_byte() + 1..node.end_byte() - 1]; // Remove quotes
                Ok(JsonValue::String(value.to_string()))
            }
            "number" => {
                let value = &code[node.start_byte()..node.end_byte()];
                Ok(JsonValue::Number(value.parse()?))
            }
            "true" => Ok(JsonValue::Bool(true)),
            "false" => Ok(JsonValue::Bool(false)),
            "null" => Ok(JsonValue::Null),
            _ => Err(Error::JsonError(JsonErrorType::ParseError)), // This line will catch any unexpected node kinds
        }
    }

    pub fn parse_pair(node: Node, code: &str) -> Result<(String, JsonValue), Error> {
        let mut cursor = node.walk();
        cursor.goto_first_child();
        let key_node = cursor.node();
        let key = &code[key_node.start_byte() + 1..key_node.end_byte() - 1]; // Remove quotes from the key
        cursor.goto_next_sibling(); // Skip the colon
        cursor.goto_next_sibling(); // Move to the value node
        let value_node = cursor.node();
        let value = Self::parse_json_node(value_node, code)?;
        Ok((key.to_string(), value))
    }
}

impl Default for JsonParser {
    fn default() -> Self {
        let mut parser = tree_sitter::Parser::new();
        let language = tree_sitter_json::language();
        parser.set_language(language).expect("Error loading JSON parser");
        JsonParser { parser }
    }
}

#[cfg(test)]
mod tests {
    use super::JsonParser;
    use tree_sitter::Parser;

    #[test]
    fn parse_pair_test() {
        let code = r#""key": "value""#;
        let mut parser = Parser::new();
        let language = tree_sitter_json::language();
        parser.set_language(language).expect("Error loading JSON parser");
        let tree = parser.parse(code, None).unwrap();
        let root_node = tree.root_node();

        let json_node = JsonParser::parse_pair(root_node, code).unwrap();
        assert_eq!(json_node.0, "key");
        assert_eq!(json_node.1.as_str(), Some("value"));
    }
}
