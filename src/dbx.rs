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

use self::ser::{CopyRule, CopyRuleLib};
use std::clone::Clone;
#[derive(Debug, Clone)]
pub struct DatabaseBuilder {
    tables: Vec<Table>,
    copy_rules: CopyRuleLib,
}

#[derive(Clone)]
pub struct SqlValue(Value);

impl SqlValue {
    pub fn new<T: ToSql>(v: T) -> SqlValue {
        match v.to_sql() {
            Ok(ToSqlOutput::Owned(val)) => SqlValue(val.clone()),
            Ok(_) => todo!(),
            Err(_) => todo!(),
        }
    }

    fn to_sql(&self) -> ToSqlOutput<'_> {
        self.0.to_sql().unwrap()
    }
}

// impl ToSql for &SqlValue {
//     fn to_sql(&self) -> rusqlite::Result<ToSqlOutput<'_>> {
//         self.0.to_sql()
//     }
// }

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
        DBRow { values: vec![] }
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
    m_ddic: Mutex<DataDictionary>,
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

#[derive(Debug, Clone)]
pub struct Table {
    name: String,
    fields: Vec<Field>,
}

#[derive(Debug, Clone)]
pub struct DataDictionary {
    tables: Vec<Table>,
    copy_rules: CopyRuleLib,
}

#[derive(Debug)]
pub struct DatabaseImpl {
    con: Option<Connection>,
}

impl Table {
    pub fn new(name: String, field_names: Vec<String>, key: Vec<String>) -> Self {
        let fields: Vec<Field> = field_names
            .iter()
            .map(|name| Field {
                name: name.into(),
                key: key.contains(name),
                exists: false,
                changed: false,
                datatype: String::new(),
                null: !key.contains(name),
                default: None,
            })
            .collect();
        Self { name, fields }
    }
    pub fn load_table_meta(&mut self, con: &Connection) {
        let mut s = con
            .prepare(format!("pragma table_info({:})", self.name).as_str())
            .expect("ok");
        let mut q = s.query(()).expect("ok");
        while let Ok(Some(r)) = q.next() {
            let field_name = r.get(1).unwrap();
            let fld = if let Some(f) = self.fields.iter_mut().find(|x| x.name == field_name) {
                f
            } else {
                let f = Field {
                    name: field_name,
                    key: false,
                    exists: true,
                    changed: false,
                    datatype: String::new(),
                    default: None,
                    null: true,
                };
                self.fields.push(f);
                self.fields.last_mut().unwrap()
            };

            let key = 1 == r.get(5).unwrap();
            fld.changed = fld.key != key;

            let has_null = 0 == r.get(3).unwrap();
            fld.changed = fld.null != has_null;

            fld.datatype = r.get(2).unwrap();
            fld.default = r.get(4).unwrap();
            fld.exists = true;
        }
    }

    fn key(&self) -> Vec<String> {
        let mut r = vec![];
        for x in self.fields.iter() {
            if x.key {
                r.push(x.name.clone())
            }
        }
        r
    }
}

impl Database {
    pub fn connect(&self, file: Option<&str>) {
        let con = match file {
            Some(path) => Connection::open(path).unwrap(),
            None => Connection::open_in_memory().unwrap(),
        };
        {
            let mut ddic = self.arc.m_ddic.lock().expect("ok");
            ddic.collect_tables(&con);
            // println!("ddic: {:#?}", ddic);
        }
        // let params = ();
        // con.execute(self.sql.as_str(), params).expect("msg");
        {
            let mut l = self.locked();
            l.con = Some(con);
        }
        println!("{:#?}", self);
    }

    pub fn is_connected(&self) -> bool {
        true
    }

    fn locked(&self) -> MutexGuard<'_, DatabaseImpl> {
        self.arc.mutex.lock().expect("lock")
    }

    pub fn activate_structure(&self) {
        {
            let ddic = self.arc.m_ddic.lock().expect("ok");
            let l = self.locked();
            for t in ddic.tables.iter() {
                let sql_cmd = if t.fields.iter().any(|x| x.exists) {
                    if t.fields.iter().any(|x| !x.exists) {
                        // change table
                        build_alter_table(t).expect("msg")
                    } else {
                        vec![]
                    }
                } else {
                    // create table
                    let str = build_create_table(t).expect("msg");
                    vec![str]
                };
                let c = l.con.as_ref().unwrap();
                for x in sql_cmd {
                    let r = c.execute(x.as_str(), ());
                    info!("execute sql: {:?}", r);
                }
            }
        }
        // let params = ();
        // con.execute(self.sql.as_str(), params).expect("msg");
        // {
        //     let mut l = self.locked();
        //     l.con = Some(con);
        // }
    }

    // pub fn new_structure(&self) -> Structure {
    //     Structure::new()
    // }

    pub fn modify_from(&self, table_name: String, row: DBRow) {
        let ddic = self.arc.m_ddic.lock().unwrap();
        info!("{} <- {}", table_name, row);
        match ddic.tables.iter().find(|x| x.name == table_name) {
            Some(tab) => {
                let x = self.locked();
                x.modify_from_upd_first(tab, ddic.copy_rules.clone(), row);
            }
            None => {
                panic!("unknown table {}", table_name);
            }
        }
    }

    pub fn select<T: DeserializeOwned>(&self, q: crate::data::Query) -> Vec<T> {
        let x = self.locked();
        x.select(q)
    }

    pub fn modify_from_ser<T>(&self, value: &T) -> Result<(), ser::Error>
    where
        T: Serialize,
    {
        let mut serializer =
            ser::SqlSerializer::new(self.arc.m_ddic.lock().unwrap().copy_rules.clone());
        value.serialize(&mut serializer)?;
        serializer.perform_copy_rules();
        for x in serializer.get_operations() {
            let row = serializer.context.get_row(x.get_index());
            // self.modify_from(x.get_table(), Structure::from(row));
        }
        Ok(())
    }
}

fn build_alter_table(t: &Table) -> Result<Vec<String>, std::fmt::Error> {
    let mut result = vec![];
    // write!(&mut sql, "ALTER TABLE {} ", t.name)?;
    for x in t.fields.iter().filter(|y| !y.exists) {
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
    write!(&mut sql, "CREATE TABLE {} (", t.name)?;
    for x in t.fields.iter() {
        write!(&mut sql, "{} {},", x.name, x.datatype)?;
    }
    write!(&mut sql, "primary key (")?;
    let mut sep = "";
    for x in t.fields.iter().filter(|y| y.key) {
        write!(&mut sql, "{}{}", sep, x.name)?;
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
        Self {
            tables: vec![],
            copy_rules: CopyRuleLib::new(),
        }
    }

    pub fn build(&self) -> Database {
        let mut dd = DataDictionary {
            tables: vec![],
            copy_rules: self.copy_rules.clone(),
        };

        for x in self.tables.iter() {
            dd.tables.push(x.clone());
        }

        Database {
            arc: Arc::new(DatabaseGuarded {
                mutex: Mutex::new(DatabaseImpl { con: None }),
                m_ddic: Mutex::new(dd),
            }),
        }
    }

    pub fn table(&mut self, name: String, fields: &[String], key: &[String]) -> &mut Self {
        self.tables
            .push(Table::new(name, fields.to_vec(), key.to_vec()));
        self
    }

    pub fn with_table(&mut self, t: Table) -> &mut Self {
        self.tables.push(t);
        self
    }

    pub fn copy_rule(&mut self, name: String, rule: CopyRule) -> &mut Self {
        self.copy_rules.add(name, rule);
        self
    }

    pub fn with_many_to_many(&mut self, _name1: &str, _name2: &str) -> &mut Self {
        self
    }
}

impl DataDictionary {
    pub fn collect_tables(&mut self, con: &Connection) {
        let mut s = con
            .prepare(format!("select name from sqlite_schema where type = 'table'").as_str())
            .expect("ok");
        let mut q = s.query(()).expect("ok");
        while let Ok(Some(r)) = q.next() {
            let table_name: String = r.get(0).expect("ok");
            let t = if let Some(t) = self.tables.iter_mut().find(|x| x.name == table_name) {
                t
            } else {
                let t = Table::new(table_name, vec![], vec![]);

                self.tables.push(t);
                self.tables.last_mut().unwrap()
            };
            t.load_table_meta(con);
        }
    }
}

impl DatabaseImpl {
    /// if the primary key is satisfied first try to insert
    /// if this returns an error, try to update.
    ///
    /// if the primary key is not satisfied just use an insert since an update
    /// could update more than one row.
    ///
    fn modify_from(&self, table: &Table, _copy_rules: CopyRuleLib, row: DBRow) {
        let (sql_ins, params) = create_insert_statement_from(&table.name, &row);
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
                    let (sql_upd, params) = create_update_statement_from(table, &row);
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

    fn modify_from_upd_first(&self, table: &Table, _copy_rules: CopyRuleLib, row: DBRow) {
        if let Some(con) = &self.con {
            let (sql_upd, params) = create_update_statement_from(table, &row);
            let mut stmt = con.prepare(sql_upd.as_str()).unwrap();
            match stmt.execute(rusqlite::params_from_iter(params)) {
                Ok(1) => {}
                Ok(x) => {
                    info!("update {} rows.", x);
                    let (sql_ins, params) = create_insert_statement_from(&table.name, &row);

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
}

fn create_update_statement_from<'a>(
    table: &Table,
    row: &'a DBRow,
) -> (String, Vec<ToSqlOutput<'a>>) {
    let mut sql = String::new();
    write!(&mut sql, "UPDATE {} SET ", table.name).unwrap();
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
    for k in table.key().iter() {
        if let Some(sqlv) = row.get(k) {
            write!(&mut sql, "{}{} = ?", sep, k).unwrap();
            sep = " AND ";
            params.push(sqlv.to_sql());
        }
    }
    // println!("insert: {}", sql);
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
