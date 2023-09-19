use std::collections::BTreeMap;
use std::fmt::Write;

pub struct Index (usize);

pub trait Vector {
    fn insert(&mut self, idx: Index, v: Value);
    fn get(&self, idx: Index) -> &Value;
}

pub trait Structure {
    fn get(&self, name: &str) -> &Value;
    fn set(&mut self, name: &str, value: Value);
    fn keys(&self) -> Vec<String>;
}

pub trait Scalar {
    fn into_string(&self) -> String;
}

pub enum Value {
    Scalar(Box<dyn Scalar>),
    Vector(Box<dyn Vector>),
    Structure(Box<dyn Structure>),
}

impl Scalar for String {
    fn into_string(&self) -> String {
        self.clone()
    }
}

impl From<&str> for Value {
    fn from(value: &str) -> Self {
        Value::Scalar(Box::new(String::from(value)))
    }
}

impl Structure for BTreeMap<String, Value> {
    fn get(&self, name: &str) -> &Value {
        self.get(name).unwrap()
    }

    fn set(&mut self, name: &str, value: Value) {
        self.insert(String::from(name), value);
    }

    fn keys(&self) -> Vec<String> {
        self.keys().into_iter().map(|x| x.clone()).collect()
    }
}

impl From<BTreeMap<String, Value>> for Value {
    fn from(value: BTreeMap<String, Value>) -> Self {
        Value::Structure(Box::new(value))
    }
}

pub enum WhereExpr {
    Equals(String, Value),
}
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
                    WhereExpr::Equals(fld, Value::Scalar(_)) => {
                        write!(&mut sql, "{}{} = ?", sep, fld).unwrap();
                    }
                    _ => todo!(),
                }
                sep = " AND ";
            }
        }
        sql
    }

    fn get_params(&self) -> Vec<String> {
        let mut p = vec![];
        for x in self.all.iter() {
            match x {
                WhereExpr::Equals(_, Value::Scalar(v)) => {
                    p.push(v.into_string());
                }
                _ => todo!(),
            }
        }
        p
    }
}

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
        sql
    }

    pub fn get_params(&self) -> Vec<String> {
        self.condition.get_params()
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
                    WhereExpr::Equals(fld, Value::Scalar(val)) => {
                        write!(&mut sql, "{}{} = '{}'", sep, fld, val.into_string()).unwrap();
                    }
                    _ => todo!(),
                }
                sep = " AND ";
            }
        }
        sql
    }
}


impl<X> Vector for Vec<X> {
    fn insert(&mut self, idx: Index, v: Value) {
        todo!()
    }

    fn get(&self, idx: Index) -> &Value {
        todo!()
    }
}