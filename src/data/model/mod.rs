pub struct DataModel {
    name: String,
    tables: Vec<Table>,
}

impl DataModel {
    pub fn new(name: &str) -> Self {
        Self {
            name: name.into(),
            tables: vec![],
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
}

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
