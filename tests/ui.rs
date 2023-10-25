use std::collections::BTreeMap;

use serde_derive::Deserialize;
use tt_rust::ui::de::{Values, from_values};


#[derive(Deserialize, Debug)]
struct Address {
    street: String,
    city: Option<String>,
    name: Option<String>,
}

#[test]
fn deserialize_result() {
    let result: Values = vec![
        ("street".to_string(), "Alter Postweg".to_string()),
        ("city".to_string(), "Burgwedel".to_string()),
    ];

    let adr: Address = from_values(&result);
    assert_eq!("Alter Postweg", adr.street);
    assert_eq!(Some("Burgwedel".to_string()), adr.city);
    assert_eq!(None, adr.name);
    
    let adr2: BTreeMap<String,String> = from_values(&result);
    assert!(adr2.len() > 0);
}

