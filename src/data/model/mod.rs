use crate::dbx::ser::{CopyRuleLib, FieldCopyRule};

use self::meta::Meta;

pub mod meta {
    #[derive(Debug, Clone)]
    pub struct Relation {
        pub id: String,
        pub from: String,
        pub to: String,
        pub name: String,
        pub kind: RelationKind,
        pub fields: Vec<(String, String)>,
    }

    #[derive(Debug, Clone)]
    pub enum RelationKind {
        One,
        Many,
        ManyMany(String),
    }
    #[derive(Debug,Clone)]
    pub struct Meta {
        relations: Vec<Relation>,
    }

    impl Meta {
        pub fn new() -> Self {
            Self { relations: vec![] }
        }

        pub fn define_relation(
            &mut self,
            kind: RelationKind,
            from: &str,
            name: &str,
            to: &str,
        ) -> String {
            let id = format!("{}:{}", from, name);
            self.relations.push(Relation {
                id: id.clone(),
                from: from.into(),
                to: to.into(),
                name: name.into(),
                kind,
                fields: vec![],
            });
            id
        }

        pub fn get_relation(&self, from: &str, name: &str) -> Option<&Relation> {
            let mut result = None;
            for x in self.relations.iter() {
                if x.from == from && x.name == name {
                    result = Some(x);
                    break;
                };
            }
            result
        }

        pub fn map_field(&mut self, id: &str, from_field: &str, to_field: &str) {
            for x in self.relations.iter_mut() {
                if x.id == id {
                    x.fields.push((from_field.into(), to_field.into()));
                    break;
                };
            }
        }
    }
}

#[derive(Debug,Clone)]
pub struct DataModel {
    name: String,
    tables: Vec<Table>,
    meta: Meta,
}

impl DataModel {
    pub fn new(name: &str) -> Self {
        Self {
            name: name.into(),
            tables: vec![],
            meta: Meta::new(),
        }
    }

    pub fn table(mut self, tab: Table) -> Self {
        self.tables.push(tab);
        self
    }

    pub fn tables(&self) -> std::slice::Iter<'_, Table> {
        self.tables.iter()
    }

    pub fn name(&self) -> &str {
        self.name.as_str()
    }

    pub fn meta(&self) -> &Meta {
        &self.meta
    }

    pub fn set_meta(&mut self, meta: Meta) {
        self.meta = meta;
    }
}

#[derive(Debug,Clone)]
pub struct Table {
    name: String,
    fields: Vec<Field>,
}

impl Table {
    pub fn new(name: &str) -> Self {
        Self {
            name: name.into(),
            fields: vec![],
        }
    }

    pub fn field(mut self, arg: &str, key: bool, datatype: &str) -> Self {
        self.fields.push(Field {
            name: arg.into(),
            key,
            datatype: String::from(datatype),
        });
        self
    }

    pub fn name(&self) -> &str {
        self.name.as_str()
    }

    pub(crate) fn fields(&self) -> std::slice::Iter<'_, Field> {
        self.fields.iter()
    }

    pub fn key(&self) -> impl Iterator<Item = &str> {
        self.fields
            .iter()
            .filter(|x| x.key)
            .map(|x| x.name.as_str())
    }
}

#[derive(Debug,Clone)]
pub struct Field {
    pub name: String,
    pub key: bool,
    pub datatype: String,
}

impl Field {
    pub fn new(name: &str, key: bool) -> Self {
        Self {
            name: name.into(),
            datatype: String::from("string"),
            key,
        }
    }
}
