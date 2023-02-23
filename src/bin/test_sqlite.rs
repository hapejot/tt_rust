extern crate sqlite;

static mut DB: Option<sqlite::Connection> = None;

fn main() {
    unsafe {
        DB = Some(sqlite::open("test.db").expect("Error opening test.db"));
        println!("Database Opened OK");
    };
    create_table();
    println!("Completed");
}

// Create Table
fn create_table() {
    let sql = "CREATE TABLE IF NOT EXISTS TEMP2 (ikey INTEGER PRIMARY KEY NOT NULL)";
    unsafe {
        if let Some(db) = &DB {
            match db.execute(sql) {
                Ok(_) => println!("Table created"),
                Err(err) => println!("Exec of Sql failed : {}\nSql={}", err, sql),
            }
        }
    }
}
