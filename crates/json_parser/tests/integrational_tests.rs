#[cfg(test)]
mod tests {
    use json_parser::JsonParser;
    #[test]
    fn test_single_pair() {
        let code = r#"
            {
                "name": "John Doe",
            }
        "#;

        let mut perser = JsonParser::default();
        let json_value = perser.parse(code).unwrap();

        let name = json_value["name"].as_str().unwrap();

        assert_eq!(name, "John Doe");
    }

    #[test]
    fn test_multiple_pairs() {
        let code = r#"
            {
                "name": "John Doe",
                "age": 30,
                "is_student": false
            }
        "#;

        let mut perser = JsonParser::default();
        let json_value = perser.parse(code).unwrap();

        let name = json_value["name"].as_str().unwrap();
        let age = json_value["age"].as_number().unwrap();
        let is_student = json_value["is_student"].as_bool().unwrap();

        assert_eq!(name, "John Doe");
        assert_eq!(age, 30.0);
        assert!(!is_student);
    }

    #[test]
    fn test_nested_object() {
        let code = r#"
            {
                "name": "John Doe",
                "age": 30,
                "is_student": false,
                "address": {
                    "street": "123 Main St",
                    "city": "Springfield",
                    "state": "IL"
                },
            }
        "#;

        let mut perser = JsonParser::default();
        let json_value = perser.parse(code).unwrap();

        let name = json_value["name"].as_str().unwrap();
        let age = json_value["age"].as_number().unwrap();
        let is_student = json_value["is_student"].as_bool().unwrap();
        let address = json_value["address"].as_object().unwrap();
        let street = address["street"].as_str().unwrap();
        let city = address["city"].as_str().unwrap();
        let state = address["state"].as_str().unwrap();

        assert_eq!(name, "John Doe");
        assert_eq!(age, 30.0);
        assert!(!is_student);
        assert_eq!(street, "123 Main St");
        assert_eq!(city, "Springfield");
        assert_eq!(state, "IL");
    }

    #[test]
    fn test_array() {
        let code = r#"
            {
                "name": "John Doe",
                "age": 30,
                "is_student": false,
                "children": [
                    "Alice",
                    "Bob"
                ],
            }
        "#;

        let mut perser = JsonParser::default();
        let json_value = perser.parse(code).unwrap();

        let name = json_value["name"].as_str().unwrap();
        let age = json_value["age"].as_number().unwrap();
        let is_student = json_value["is_student"].as_bool().unwrap();
        let children = json_value["children"].as_array().unwrap();
        let child1 = children[0].as_str().unwrap();
        let child2 = children[1].as_str().unwrap();

        assert_eq!(name, "John Doe");
        assert_eq!(age, 30.0);
        assert!(!is_student);
        assert_eq!(child1, "Alice");
        assert_eq!(child2, "Bob");
    }

    #[test]
    fn test_invalid_json() {
        let code = r#"
            {
                "name": "John Doe",
                "age": 30,
                "is_student": false,
                "address": {
                    "street": "123 Main St",
                    "city": "Springfield",
                    "state": "IL"
                },
                "children": [
                    "Alice",
                    "Bob", 
                ],
            }
        "#; // Extra comma after "Bob" in array, obvious error

        let mut perser = JsonParser::default();
        let json_parse_result = perser.parse(code);

        println!("{:?}", json_parse_result);
        assert!(json_parse_result.is_err());
    }

    #[test]
    fn array_test() {
        let code = r#"
            {
                "students":
                [
                    {
                        "name": "John Doe",
                        "age": 30,
                        "is_student": true
                    },
                    {
                        "name": "Jane Doe",
                        "age": 28,
                        "is_student": true
                    }
                ]
            }
        "#;

        let mut perser = JsonParser::default();
        let json_value = perser.parse(code).unwrap();

        let students = &json_value["students"].as_array().unwrap();

        let student1 = &students[0].as_object().unwrap();
        let student2 = &students[1].as_object().unwrap();

        let name1 = student1["name"].as_str().unwrap();
        let age1 = student1["age"].as_number().unwrap();
        let is_student1 = student1["is_student"].as_bool().unwrap();

        let name2 = student2["name"].as_str().unwrap();
        let age2 = student2["age"].as_number().unwrap();
        let is_student2 = student2["is_student"].as_bool().unwrap();

        assert_eq!(name1, "John Doe");
        assert_eq!(age1, 30.0);
        assert!(is_student1);

        assert_eq!(name2, "Jane Doe");
        assert_eq!(age2, 28.0);
        assert!(is_student2);
    }
}
