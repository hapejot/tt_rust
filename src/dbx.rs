use rusqlite::{
    ffi::Error,
    types::{ToSqlOutput, Value},
    Connection, ErrorCode, ToSql,
};
use serde::{de::DeserializeOwned, Serialize};
use std::{
    fmt::{Display, Write},
    sync::{Arc, Mutex, MutexGuard},
};
use tracing::*;
pub mod de;
pub mod ser;

use crate::data::model::{DataModel, Table};

use self::ser::{CopyRule, CopyRuleLib};
use std::clone::Clone;
#[derive(Debug, Clone)]
pub struct DatabaseBuilder {}

#[derive(Clone, PartialEq)]
pub struct SqlValue(Value);

impl SqlValue {
    pub fn new<T: ToSql>(v: T) -> SqlValue {
        match v.to_sql() {
            Ok(ToSqlOutput::Owned(val)) => SqlValue(val.clone()),
            Ok(ToSqlOutput::Borrowed(val)) => SqlValue(val.into()),
            Ok(_) => todo!(),
            Err(_) => todo!(),
        }
    }

    fn to_sql(&self) -> ToSqlOutput<'_> {
        self.0.to_sql().unwrap()
    }
}

impl ToSql for SqlValue {
    fn to_sql(&self) -> rusqlite::Result<ToSqlOutput<'_>> {
        self.0.to_sql()
    }
}

impl From<&str> for SqlValue {
    fn from(value: &str) -> Self {
        SqlValue(Value::Text(value.to_string()))
    }
}

impl From<SqlValue> for String {
    fn from(value: SqlValue) -> Self {
        if let Value::Text(s) = value.0 {
            s
        } else {
            panic!("exctracting string value from a non-string.");
        }
    }
}

impl From<Value> for SqlValue {
    fn from(value: Value) -> Self {
        SqlValue(value)
    }
}

impl From<bool> for SqlValue {
    fn from(value: bool) -> Self {
        SqlValue(Value::Integer(if value { 1 } else { 0 }))
    }
}

impl From<u64> for SqlValue {
    fn from(value: u64) -> Self {
        SqlValue(Value::Integer(value as i64))
    }
}

#[derive(Clone)]
pub struct DBRow {
    table: Option<String>,
    values: Vec<(String, SqlValue)>,
}

impl DBRow {
    fn get(&self, k: &str) -> Option<&SqlValue> {
        if let Some((_, val)) = self.values.iter().find(|(key, _)| key == k) {
            Some(val)
        } else {
            None
        }
    }

    pub fn new() -> DBRow {
        DBRow {
            table: None,
            values: vec![],
        }
    }

    pub fn insert(&mut self, k: String, v: SqlValue) {
        self.values.push((k.clone(), v));
    }

    pub fn keys(&self) -> Vec<&str> {
        self.values.iter().map(|(k, _)| k.as_str()).collect()
    }

    pub fn exists(&self, k: &str) -> bool {
        self.values.iter().any(|(key, _)| key == k)
    }

    pub fn index(&self, k: &str) -> Option<usize> {
        self.values.iter().position(|(key, _)| key == k)
    }

    pub fn remove(&mut self, k: &str) {
        if let Some(pos) = self.index(k) {
            self.values.remove(pos);
        }
    }

    pub fn set(&mut self, k: &str, v: SqlValue) {
        self.remove(k);
        self.values.push((k.into(), v));
    }

    fn get_at(&self, idx: usize) -> &SqlValue {
        &self.values[idx].1
    }

    fn len(&self) -> usize {
        self.values.len()
    }

    fn key_at(&self, idx: usize) -> &str {
        self.values[idx].0.as_str()
    }

    fn table(&self) -> &str {
        match &self.table {
            Some(t) => t.as_str(),
            None => todo!(),
        }
    }
}

impl Display for DBRow {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut sep = '<';
        for (name, _) in self.values.iter() {
            write!(f, "{}{}", sep, name)?;
            sep = ',';
        }
        write!(f, ">")
    }
}

#[derive(Clone)]
pub struct Database {
    arc: Arc<DatabaseGuarded>,
}

pub struct DatabaseGuarded {
    mutex: Mutex<DatabaseImpl>,
}

#[derive(Debug, Clone)]
pub struct Field {
    name: String,
    datatype: String,
    key: bool,
    null: bool,
    exists: bool,
    changed: bool,
    default: Option<String>,
}

#[derive(Debug)]
pub struct DBField {
    name: String,
    datatype: String,
    default: Option<String>,
    key: bool,
    has_null: bool,
}

#[derive(Debug)]
pub struct DBTable {
    name: String,
    fields: Vec<DBField>,
}

#[derive(Debug)]
pub struct DataDictionary {}

#[derive(Debug)]
pub struct DatabaseImpl {
    con: Option<Connection>,
    tables: Vec<DBTable>,
}

impl DBTable {
    pub fn new(name: &str) -> Self {
        Self {
            name: name.to_string(),
            fields: vec![],
        }
    }

    pub fn load_table_meta(&mut self, con: &Connection) {
        info!("table: {}", self.name);
        let mut s = con
            .prepare(format!("pragma table_info({:})", self.name).as_str())
            .unwrap();
        let mut q = s.query(()).expect("ok");
        while let Ok(Some(r)) = q.next() {
            let db_field = DBField {
                name: r.get(1).unwrap(),
                datatype: r.get(2).unwrap(),
                default: r.get(4).unwrap(),
                key: 1 == r.get(5).unwrap(),
                has_null: 0 == r.get(3).unwrap(),
            };
            trace!("{:?}", db_field);
            self.fields.push(db_field);
        }
    }

    fn key(&self) -> Vec<&str> {
        self.fields
            .iter()
            .filter(|x| x.key)
            .map(|x| x.name.as_str())
            .collect()
    }
}

impl Database {
    pub fn new() -> Self {
        Self {
            arc: Arc::new(DatabaseGuarded {
                mutex: Mutex::new(DatabaseImpl {
                    con: None,
                    tables: vec![],
                }),
            }),
        }
    }

    pub fn connect(&self, file: Option<&str>) {
        let con = match file {
            Some(path) => Connection::open(path).unwrap(),
            None => Connection::open_in_memory().unwrap(),
        };

        let mut l = self.locked();
        l.con = Some(con);
        l.load_meta();
    }

    pub fn is_connected(&self) -> bool {
        true
    }

    fn locked(&self) -> MutexGuard<'_, DatabaseImpl> {
        self.arc.mutex.lock().unwrap()
    }

    pub fn activate_structure(&self, model: DataModel) {
        let mut x = self.locked();
        x.activate_structure(model);
    }

    // pub fn new_structure(&self) -> Structure {
    //     Structure::new()
    // }

    pub fn modify_from(&self, table_name: &str, row: &DBRow) {
        let x = self.locked();
        x.modify_from_upd_first(table_name, row);
    }

    pub fn select<T: DeserializeOwned>(&self, q: crate::data::Query) -> Vec<T> {
        let x = self.locked();
        x.select(q)
    }

    pub fn modify_from_ser<T>(&self, value: &T) -> Result<(), ser::Error>
    where
        T: Serialize,
    {
        let x = ser::serialize_row(value);
        for r in x {
            self.modify_from(r.table(), &r);
        }
        Ok(())
    }

    pub fn execute_query(&self, arg: &str) -> Vec<DBRow> {
        let x = self.locked();
        x.execute_query(arg)
    }

    pub fn tables(&self) -> Vec<String> {
        let x = self.locked();
        x.tables.iter().map(|x| x.name.clone()).collect()
    }
}

fn build_alter_table(t: &DBTable, t0: &Table) -> Result<Vec<String>, std::fmt::Error> {
    let mut result = vec![];
    // write!(&mut sql, "ALTER TABLE {} ", t.name)?;
    for x in t.fields.iter() {
        info!("field {} needs to be created.", x.name);
        let mut sql = String::new();
        write!(&mut sql, "ALTER TABLE {} ", t.name)?;
        write!(&mut sql, "ADD COLUMN {}", x.name)?;
        result.push(sql);
    }
    // info!("sql: {}", sql);
    Ok(result)
}

fn build_create_table(t: &Table) -> Result<String, std::fmt::Error> {
    let mut sql = String::new();
    write!(&mut sql, "CREATE TABLE {} (", t.name())?;
    for x in t.fields() {
        write!(&mut sql, "{} {},", x.name, x.datatype)?;
    }
    write!(&mut sql, "primary key (")?;
    let mut sep = "";
    for x in t.key() {
        write!(&mut sql, "{}{}", sep, x)?;
        sep = ",";
    }
    write!(&mut sql, ") );")?;
    Ok(sql)
}

impl std::fmt::Debug for Database {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if let Ok(m) = self.arc.mutex.lock() {
            (*m).fmt(f)
        } else {
            f.debug_struct("Database").field("name", &"value").finish()
        }
    }
}

impl DatabaseBuilder {
    pub fn new() -> Self {
        Self {}
    }

    pub fn build(&self) -> Database {
        Database {
            arc: Arc::new(DatabaseGuarded {
                mutex: Mutex::new(DatabaseImpl {
                    con: None,
                    tables: vec![],
                }),
            }),
        }
    }
}

impl DataDictionary {
    pub fn collect_tables(&mut self, con: &Connection) {
        let mut s = con
            .prepare(format!("select name from sqlite_schema where type = 'table'").as_str())
            .unwrap();
        let mut q = s.query(()).unwrap();
        while let Ok(Some(r)) = q.next() {
            let table_name: String = r.get(0).unwrap();
        }

        //         let t = if let Some(t) = self.lookup_table(table_name.as_str()) {
        //             t
        //         } else {
        //             let t = Table::new(table_name.as_str(), vec![], vec![]);

        //             self.tables.push(t);
        //             self.tables.last_mut().unwrap()
        //         };
        //         t.load_table_meta(con);
        //     }
    }
}

impl DatabaseImpl {
    /// if the primary key is satisfied first try to insert
    /// if this returns an error, try to update.
    ///
    /// if the primary key is not satisfied just use an insert since an update
    /// could update more than one row.
    ///
    fn modify_from(&self, table_name: &str, row: DBRow) {
        let (sql_ins, params) = create_insert_statement_from(&table_name, &row);
        if let Some(con) = &self.con {
            let mut stmt = con.prepare(sql_ins.as_str()).unwrap();
            match stmt.execute(rusqlite::params_from_iter(params)) {
                Ok(_) => {}
                Err(rusqlite::Error::SqliteFailure(
                    Error {
                        code: ErrorCode::ConstraintViolation,
                        extended_code: 1555,
                    },
                    _,
                )) => {
                    println!("PK already exists");
                    let (sql_upd, params) = create_update_statement_from(table_name, &[], &row);
                    let mut stmt = con.prepare(sql_upd.as_str()).unwrap();

                    match stmt.execute(rusqlite::params_from_iter(params)) {
                        Ok(x) => {
                            assert_eq!(x, 1);
                        }
                        Err(_) => todo!(),
                    }
                }
                _ => panic!(),
            }
        }
    }

    fn modify_from_upd_first(&self, table_name: &str, row: &DBRow) {
        if let Some(con) = &self.con {
            let tab = self.table(table_name);
            let key = tab.key();
            assert!(key.len() > 0, "no keys found in {:?}", tab);
            let (sql_upd, params) = create_update_statement_from(table_name, &key, &row);
            info!("sql upd: {}", sql_upd);
            let mut stmt = con.prepare(sql_upd.as_str()).unwrap();
            match stmt.execute(rusqlite::params_from_iter(params)) {
                Ok(1) => {}
                Ok(x) => {
                    info!("update {} rows.", x);
                    let (sql_ins, params) = create_insert_statement_from(&table_name, &row);

                    let mut stmt = con.prepare(sql_ins.as_str()).unwrap();
                    match stmt.execute(rusqlite::params_from_iter(params)) {
                        Ok(_) => {}
                        _ => panic!(),
                    }
                }
                Err(_) => todo!(),
            }
        }
    }

    pub fn select<T>(&self, q: crate::data::Query) -> Vec<T>
    where
        T: serde::de::DeserializeOwned,
    {
        let mut result: Vec<T> = vec![];
        if let Some(con) = &self.con {
            let mut stmt = con.prepare(q.get_sql().as_str()).unwrap();
            let sql_result = stmt.query(rusqlite::params_from_iter(q.get_params()));
            match sql_result {
                Ok(mut rows) => {
                    while let Some(row) = rows.next().unwrap() {
                        result.push(de::from_row(row).unwrap());
                        // for c in 0..&stmt.column_count() {}
                    }
                }
                Err(x) => {
                    error!("SELECT ERROR {:#?}", x);
                }
            }
        };
        result
    }

    pub fn execute_query(&self, arg: &str) -> Vec<DBRow> {
        let mut result: Vec<DBRow> = vec![];
        if let Some(con) = &self.con {
            let mut stmt_m = con.prepare(arg).unwrap();
            let sql_result = stmt_m.query([]);
            // let n = sql_result.column_count();
            match sql_result {
                Ok(mut rows) => {
                    let names = rows
                        .as_ref()
                        .unwrap()
                        .column_names()
                        .iter()
                        .map(|x| x.to_string())
                        .collect::<Vec<_>>();
                    while let Some(row) = rows.next().unwrap() {
                        let mut res_row = DBRow::new();
                        for idx in 0..names.len() {
                            let v = Value::from(row.get::<_, Value>(idx).unwrap());
                            res_row.insert(names[idx].clone(), SqlValue(v));
                        }
                        result.push(res_row);
                    }
                }
                Err(x) => {
                    error!("SELECT ERROR {:#?}", x);
                }
            }
        };
        result
    }

    pub fn collect_tables(&mut self) {
        if let Some(con) = &self.con {
            let mut s = con
                .prepare(format!("select name from sqlite_schema where type = 'table'").as_str())
                .unwrap();
            let mut q = s.query(()).unwrap();
            while let Ok(Some(r)) = q.next() {
                let table_name: String = r.get(0).unwrap();
                self.tables.push(DBTable::new(table_name.as_str()));
            }
        }
    }

    pub fn load_meta(&mut self) {
        self.collect_tables();
        if let Some(con) = &self.con {
            for t in self.tables.iter_mut() {
                t.load_table_meta(con);
            }
        }
    }

    pub fn activate_structure(&mut self, model: DataModel) {
        if let Some(con) = &self.con {
            for t in model.tables() {
                info!("activate table {}", t.name());
                let src = build_create_table(t).unwrap();
                info!("{}", src);
                con.execute(src.as_str(), []).unwrap();
            }
        }
        self.load_meta();
    }

    fn table(&self, table_name: &str) -> &DBTable {
        let r = self.tables.iter().find(|x| x.name == table_name).unwrap();
        r
    }
}

fn create_update_statement_from<'a>(
    table_name: &str,
    key: &[&str],
    row: &'a DBRow,
) -> (String, Vec<ToSqlOutput<'a>>) {
    let mut sql = String::new();
    write!(&mut sql, "UPDATE {} SET ", table_name).unwrap();
    let mut sep = "";
    let mut params = vec![];
    for (k, v) in row.values.iter() {
        let sqlv = v.to_sql();
        write!(&mut sql, "{}{} = ?", sep, k).unwrap();
        sep = ",";
        params.push(sqlv);
    }
    write!(&mut sql, " WHERE ").unwrap();
    sep = "";
    for k in key.iter() {
        if let Some(sqlv) = row.get(k) {
            write!(&mut sql, "{}{} = ?", sep, k).unwrap();
            sep = " AND ";
            params.push(sqlv.to_sql());
        }
    }
    (sql, params)
}

fn create_insert_statement_from<'a>(arg: &str, s: &'a DBRow) -> (String, Vec<ToSqlOutput<'a>>) {
    let mut sql = String::new();
    write!(&mut sql, "INSERT INTO {}(", arg).unwrap();
    let mut sep = "";
    for (k, _) in s.values.iter() {
        write!(&mut sql, "{}{}", sep, k).unwrap();
        sep = ",";
    }
    write!(&mut sql, ") VALUES (").unwrap();
    sep = "";
    let mut params = vec![];
    for (k, v) in s.values.iter() {
        write!(&mut sql, "{}?", sep).unwrap();
        sep = ",";
        params.push(v.to_sql());
    }

    write!(&mut sql, ")").unwrap();
    // println!("insert: {}", sql);
    (sql, params)
}
