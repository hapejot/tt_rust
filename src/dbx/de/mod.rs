use serde::de::DeserializeOwned;
use derow::Deserializer;
mod derow;



pub fn from_row<T>(row: &rusqlite::Row) -> crate::error::Result<T>
where
    T: DeserializeOwned,
{
	let columns = row.as_ref().column_names().iter().map(|x| x.to_string()).collect::<Vec<_>>();

    let de = Deserializer::from_row(row, &columns);
    let r = T::deserialize(de);
    r
}

// pub fn from_statement<'stmt>(stmt: )

#[cfg(test)]
mod testing {
    use serde_derive::Deserialize;

    use rusqlite::{params, Connection, Result};

    use crate::dbx::de::{from_row};

    // use super::StatementStream;

    #[derive(Deserialize, Debug, PartialEq)]
    struct Person {
        id: i32,
        name: Option<String>,
        data: Option<Vec<u8>>,
    }

    #[test]
    fn test_statement_stream() {
        let r = statement_stream().unwrap();
        assert_eq!(
            vec![Person {
                id: 1,
                name: Some("Steven".to_string()),
                data: Some(vec![1,2,3]),
            }],
            r
        );
    }

    fn statement_stream() -> Result<Vec<Person>, Box<dyn error::Error>> {
        use rusqlite::*;
        let conn = setup_data();
        let mut stmt = conn.prepare("SELECT id, name, data FROM person where name = :name")?;
        let mut rows = stmt.query(named_params! { ":name": "Steven" })?;
        let mut result = vec![];
        while let Some(row) = rows.next()? {
            result.push(from_row(row).unwrap());
        }
        Ok(result)
    }

    #[test]
    fn main() -> Result<()> {
        let conn = setup_data();

        let mut stmt = conn.prepare("SELECT id, name, data FROM person")?;
        // let x: Person = from_statement(stmt).unwrap();
        // let person_iter = stmt.query_map([], |row| {
        //     Ok(Person {
        //         id: row.get(0)?,
        //         name: row.get(1)?,
        //         data: row.get(2)?,
        //     })
        // })?;

        // for person in person_iter {
        //     println!("Found person {:?}", person.unwrap());
        // }
        Ok(())
    }

    fn setup_data() -> Connection {
        let conn = Connection::open_in_memory().unwrap();
        conn.execute(
            "CREATE TABLE person (
                   id   INTEGER PRIMARY KEY,
                   name TEXT NOT NULL,
                   data BLOB
               )",
            (), // empty list of parameters.
        )
        .unwrap();
        conn.execute(
            "INSERT INTO person (name, data) VALUES (?1, ?2)",
            ("Steven", vec![1,2,3]),
        )
        .unwrap();
        conn
    }
}
