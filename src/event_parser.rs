use std::collections::BTreeMap;

use ethers::{types::H256, utils::keccak256};

#[derive(Debug, Clone)]
pub struct Event {
    pub name: String,
    pub params: Vec<EventParam>,
    pub topics: Vec<EventParam>,
    pub data: Vec<EventParam>,
}

impl Event {
    pub fn new(event: String) -> Self {
        // event = "Deposit(address indexed reverse, address indexed address , uint256 amount, uint16 indexed referral, uint256 timestamp);"
        let event = event.trim().replace("event", "");
        let name_with_body: Vec<&str> = event.split("(").collect();
        let name = name_with_body[0].trim().to_string();
        let body = name_with_body[1].replace(")", "").replace(";", "");
        let mut params = Vec::new();
        let mut topics = Vec::new();
        let mut data = Vec::new();

        let params_str_list: Vec<&str> = body.split(",").collect();
        let params_count: i32 = params_str_list.len().try_into().unwrap();
        for params_str in params_str_list {
            let triple: Vec<&str> = params_str.trim().split(" ").collect();
            assert!(triple.len() >= 2, "triple len incorrect");
            let name = triple[triple.len() - 1].trim().to_string();
            let evm_type = triple[0].trim().to_string();
            let indexed = triple[1] == "indexed"; // && !["string".to_string()].contains(&evm_type);
            let param = EventParam {
                name,
                indexed,
                evm_type,
            };

            params.push(param.clone());
            if indexed {
                topics.push(param);
            } else {
                data.push(param);
            }
        }

        let event = Event {
            name,
            params,
            topics,
            data,
        };
        return event;
    }

    pub fn to_signature(&self) -> String {
        let body: Vec<_> = self.params.iter().map(|x| x.to_signature()).collect();

        return self.name.to_string() + "(" + &body.join(",") + ")";
    }

    pub fn hash(&self) -> H256 {
        return H256::from(keccak256(self.to_signature().as_bytes()));
    }
}

#[derive(Debug, Clone)]
pub struct EventParam {
    pub name: String,
    pub indexed: bool,
    pub evm_type: String,
}

impl EventParam {
    pub fn to_string(&self) -> String {
        if self.indexed {
            return vec![
                self.evm_type.to_owned(),
                "indexed".to_string(),
                self.name.to_owned(),
            ]
            .join(" ");
        }

        return vec![self.evm_type.to_owned(), self.name.to_owned()].join(" ");
    }

    pub fn to_signature(&self) -> String {
        return self.evm_type.to_owned();
    }
}

// TODO: add struct arg support
struct Struct {
    order: Vec<String>,
    map: BTreeMap<String, StructParam>,
}

type StructParam = EventParam;

#[cfg(test)]
mod tests {
    // // Note this useful idiom: importing names from outer (for mod tests) scope.
    use super::*;

    use crate::event_parser::Event;

    #[test]
    fn test_() {
        let e = Event::new("Deposit(address indexed reverse, address indexed address , uint256 amount, uint16 indexed referral, uint256 timestamp);".to_owned());
        println!("{:?}", e);
        println!("{}", e.to_signature());
        // assert_eq!((1, 2), 3);
    }
}
