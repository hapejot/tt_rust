use rusqlite::Connection;
use std::{
    clone,
    fmt::Write,
    ptr::null,
    sync::{Arc, Mutex, MutexGuard},
};
#[derive(Debug, Clone)]
pub struct DatabaseBuilder {
    tables: Vec<Table>,
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
                datatype: String::new(),
                null: true,
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
            let pk: u8 = r.get(5).unwrap();
            let nulls_allowed = if 0 == r.get(3).unwrap() { true } else { false };
            let field_name = r.get(1).unwrap();
            if let Some(f) = self.fields.iter_mut().find(|x| x.name == field_name) {
            } else {
            }
            self.fields.push(Field {
                name: field_name,
                key: if pk == 1 { true } else { false },
                exists: true,
                datatype: r.get(2).unwrap(),
                default: r.get(4).unwrap(),
                null: nulls_allowed,
            });
        }
    }
}

impl Database {
    pub fn connect(&self) {
        let con = Connection::open("agent.sqlite").expect("open");
        {
            let mut ddic = self.arc.m_ddic.lock().expect("ok");
            ddic.collect_tables(&con);
            println!("ddic: {:#?}", ddic);
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

    fn sql_create_table(name: &String, fields: &[String]) {
        let mut sql = String::new();
        write!(&mut sql, "CREATE TABLE if not exists {} (", name).expect("write");
        for x in fields {
            write!(&mut sql, "{} TEXT,", x).expect("write");
        }
        write!(&mut sql, "primary key (id) );").expect("write");
    }
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
        Self { tables: vec![] }
    }

    pub fn build(&self) -> Database {
        let mut dd = DataDictionary { tables: vec![] };

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
}

impl DataDictionary {
    pub fn collect_tables(&mut self, con: &Connection) {
        let mut s = con
            .prepare(format!("select name from sqlite_schema where type = 'table'").as_str())
            .expect("ok");
        let mut q = s.query(()).expect("ok");
        while let Ok(Some(r)) = q.next() {
            let table_name: String = r.get(0).expect("ok");
            let mut t = if let Some(mut t) = self.tables.iter_mut().find(|x| x.name == table_name) { t
            } else {
                let t = Table::new(table_name, vec![], vec![]);
                
                self.tables.push(t);
                self.tables.last_mut().unwrap()
            };
            t.load_table_meta(con);
        }
    }
}
