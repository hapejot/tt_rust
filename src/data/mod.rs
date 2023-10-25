use std::clone::Clone;
use std::collections::BTreeMap;
use std::fmt::Write;
use std::ops::Index;
use tracing::{error,warn,info, debug,trace};

use crate::dbx::SqlValue;

#[derive(Debug, Clone)]
pub enum Scalar {
    String(String),
}
impl Scalar {
    pub fn into_string(&self) -> String {
        match self {
            Scalar::String(s) => s.clone(),
        }
    }
}

// #[derive(Debug, Clone)]
// pub struct Structure {
//     pairs: Vec<(String, Value)>,
// }
// impl Structure {
//     pub fn new() -> Structure {
//         Structure { pairs: vec![] }
//     }

//     pub fn keys(&self) -> Vec<String> {
//         self.pairs.iter().map(|(x, _)| x.clone()).collect()
//     }

//     pub fn get(&self, k: &str) -> Value {
//         let mut result = Value::EmptyValue;
//         for (_key, val) in self.pairs.iter().filter(|(j, _)| j == k) {
//             result = val.clone();
//             break;
//         }
//         result
//     }

//     pub fn exists(&self, k: &str) -> bool {
//         self.pairs.iter().any(|(key, _)| key == k)
//     }

//     fn index(&self, k: &str) -> Option<usize> {
//         self.pairs.iter().position(|(key, _)| key == k)
//     }

//     pub fn remove(&mut self, k: &str) {
//         if let Some(pos) = self.index(k) {
//             self.pairs.remove(pos);
//         }
//     }

//     pub fn set(&mut self, k: &str, v: Value) {
//         self.remove(k);
//         self.pairs.push((k.into(), v.clone()));
//     }

//     fn get_at(&self, idx: usize) -> &Value {
//         &self.pairs[idx].1
//     }

//     fn len(&self) -> usize {
//         self.pairs.len()
//     }

//     fn key_at(&self, idx: usize) -> &str {
//         self.pairs[idx].0.as_str()
//     }
// }

// #[derive(Debug, Clone)]
// enum Value {
//     EmptyValue,
//     ScalarValue(Scalar),
//     VectorValue(Vec<Value>),
//     StructureValue(Structure),
// }

// impl From<&str> for Value {
//     fn from(value: &str) -> Self {
//         Value::ScalarValue(Scalar::String(String::from(value)))
//     }
// }


// impl From<bool> for Value {
//     fn from(value: bool) -> Self {
//         match value {
//             true => Value::ScalarValue(Scalar::String("1".to_string())),
//             false => Value::ScalarValue(Scalar::String("0".to_string())),
//         }
//     }
// }

// impl From<BTreeMap<String, Value>> for Value {
//     fn from(value: BTreeMap<String, Value>) -> Self {
//         Value::StructureValue(Structure {
//             pairs: value.iter().map(|(k, v)| (k.clone(), v.clone())).collect(),
//         })
//     }
// }

// impl From<BTreeMap<String, Value>> for Structure {
//     fn from(value: BTreeMap<String, Value>) -> Self {
//         Structure {
//             pairs: value.iter().map(|(k, v)| (k.clone(), v.clone())).collect(),
//         }
//     }
// }

// impl From<&BTreeMap<String, Value>> for Structure {
//     fn from(value: &BTreeMap<String, Value>) -> Self {
//         Structure {
//             pairs: value.iter().map(|(k, v)| (k.clone(), v.clone())).collect(),
//         }
//     }
// }

// impl<T> From<Vec<T>> for Value
// where
//     Value: From<T>,
// {
//     fn from(value: Vec<T>) -> Self {
//         Value::VectorValue(value.into_iter().map(|x| Value::from(x)).collect())
//     }
// }

// impl From<Structure> for Value {
//     fn from(value: Structure) -> Self {
//         Value::StructureValue(value)
//     }
// }

#[derive(Clone)]
pub enum WhereExpr {
    Equals(String, SqlValue),
}
#[derive(Clone)]
pub struct WhereCondition {
    all: Vec<WhereExpr>,
}

impl WhereCondition {
    pub fn new() -> Self {
        Self { all: vec![] }
    }
    pub fn and(mut self, expr: WhereExpr) -> Self {
        self.all.push(expr);
        self
    }

    fn get_sql(&self) -> String {
        let mut sep = "";
        let mut sql = String::new();
        if self.all.len() > 0 {
            write!(&mut sql, " WHERE ").unwrap();
            for x in self.all.iter() {
                match x {
                    WhereExpr::Equals(fld, v) => {
                        write!(&mut sql, "{}{} = ?", sep, fld).unwrap();
                    }
                    _ => todo!(),
                }
                sep = " AND ";
            }
        }
        sql
    }

    fn get_params(&self) -> Vec<SqlValue> {
        let mut p = vec![];
        for x in self.all.iter() {
            match x {
                WhereExpr::Equals(_, v) => {
                    p.push(v.clone());
                }
                _ => todo!(),
            }
        }
        p
    }
}

#[derive(Clone)]
pub struct Query {
    table: String,
    fields: Vec<String>,
    condition: WhereCondition,
}

impl Query {
    pub fn new(table: &str, fields: Vec<&str>, condition: WhereCondition) -> Self {
        Self {
            table: table.into(),
            fields: fields.into_iter().map(|x| x.into()).collect(),
            condition,
        }
    }

    pub fn get_sql(&self) -> String {
        let mut sep = "";
        let mut sql = String::new();
        write!(&mut sql, "SELECT ").unwrap();
        for x in self.fields.iter() {
            write!(&mut sql, "{}{}", sep, x).unwrap();
            sep = ",";
        }
        write!(&mut sql, " FROM {}", self.table).unwrap();
        let cond: String = self.condition.get_sql();
        write!(&mut sql, "{}", cond).unwrap();
        trace!("sql:{sql}");
        sql
    }

    pub fn get_params(&self) -> Vec<SqlValue> {
        self.condition.get_params()
    }

    pub(crate) fn fields(&self) -> core::slice::Iter<'_, String> {
        self.fields.iter()
    }
}

impl From<Query> for String {
    fn from(value: Query) -> Self {
        let mut sep = "";
        let mut sql = String::new();
        write!(&mut sql, "SELECT ").unwrap();
        for x in value.fields {
            write!(&mut sql, "{}{}", sep, x).unwrap();
            sep = ",";
        }
        write!(&mut sql, " FROM {}", value.table).unwrap();
        let cond: String = value.condition.into();
        write!(&mut sql, "{}", cond).unwrap();
        sql
    }
}

impl From<WhereCondition> for String {
    fn from(value: WhereCondition) -> Self {
        let mut sep = "";
        let mut sql = String::new();
        if value.all.len() > 0 {
            write!(&mut sql, " WHERE ").unwrap();
            for x in value.all {
                match x {
                    WhereExpr::Equals(fld, val) => {
                        write!(&mut sql, "{}{} = '{}'", sep, fld, String::from(val)).unwrap();
                    }
                    _ => todo!(),
                }
                sep = " AND ";
            }
        }
        sql
    }
}

