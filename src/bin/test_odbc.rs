//! A program executing a query and printing the result as csv to standard out. Requires
//! `anyhow` and `csv` crate.

// use anyhow::Error;
use odbc_api::{buffers::TextRowSet, Cursor, Environment, ResultSetMetadata};
use std::{
    error::Error,
    io::{stdout},
};

/// Maximum number of rows fetched with one row set. Fetching batches of rows is usually much
/// faster than fetching individual rows.
const BATCH_SIZE: usize = 5000;

fn main() -> Result<(), Box<dyn Error>> {
    // Write csv to standard out
    let out = stdout();
    let mut writer = csv::Writer::from_writer(out);

    // If you do not do anything fancy it is recommended to have only one Environment in the
    // entire process.
    let environment = Environment::new()?;

    // Connect using a DSN. Alternatively we could have used a connection string
    let connection = environment.connect("app_server", "sa", "Kennwort01", odbc_api::ConnectionOptions::default())?;

    // Execute a one of query without any parameters.
    match connection.execute("SELECT * FROM t01", ())? {
        Some(mut cursor) => {
            // Write the column names to stdout
            let headline: Vec<String> = cursor.column_names()?.collect::<Result<_, _>>()?;
            writer.write_record(headline)?;

            // Use schema in cursor to initialize a text buffer large enough to hold the largest
            // possible strings for each column up to an upper limit of 4KiB.
            let mut buffers = TextRowSet::for_cursor(BATCH_SIZE, &mut cursor, Some(4096))?;
            // Bind the buffer to the cursor. It is now being filled with every call to fetch.
            let mut row_set_cursor = cursor.bind_buffer(&mut buffers)?;

            // Iterate over batches
            while let Some(batch) = row_set_cursor.fetch()? {
                // Within a batch, iterate over every row
                for row_index in 0..batch.num_rows() {
                    // Within a row iterate over every column
                    let record = (0..batch.num_cols())
                        .map(|col_index| batch.at(col_index, row_index).unwrap_or(&[]));
                    // Writes row as csv
                    writer.write_record(record)?;
                }
            }
        }
        None => {
            eprintln!("Query came back empty. No output has been created.");
        }
    }

    Ok(())
}
