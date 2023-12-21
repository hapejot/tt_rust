//! A program executing a query and printing the result as csv to standard out. Requires
//! `anyhow` and `csv` crate.

// use anyhow::Error;
use odbc_api::{
    buffers::{AnyBuffer, BufferDesc, ColumnarAnyBuffer, TextRowSet},
    Cursor, Environment, ResultSetMetadata,
};
use std::{error::Error, io::stdout};

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
    let connection = environment.connect(
        "app_server",
        "sa",
        "Kennwort01",
        odbc_api::ConnectionOptions::default(),
    )?;

    // Execute a one of query without any parameters.
    match connection.execute("SELECT * FROM t01", ())? {
        Some(mut cursor) => {
            // Write the column names to stdout
            let headline: Vec<String> = cursor.column_names()?.collect::<Result<_, _>>()?;
            writer.write_record(headline)?;

            let mut descs = vec![];
            let n = cursor.num_result_cols().unwrap() as u16;
            for idx in 1..=n {
                descs.push(
                    BufferDesc::from_data_type(cursor.col_data_type(idx).unwrap(), true).unwrap(),
                );
            }
            // Use schema in cursor to initialize a text buffer large enough to hold the largest
            // possible strings for each column up to an upper limit of 4KiB.
            let mut buffers = ColumnarAnyBuffer::from_descs(10, descs);
            // Bind the buffer to the cursor. It is now being filled with every call to fetch.
            let mut row_set_cursor = cursor.bind_buffer(&mut buffers)?;

            // Iterate over bat
            while let Some(batch) = row_set_cursor.fetch()? {
                // Within a batch, iterate over every row
                for row_index in 0..batch.num_rows() {
                    // Within a row iterate over every column
                    for col_idx in 0..batch.num_cols() {
                        let a1 = batch.column(col_idx);
                        match a1 {
                            odbc_api::buffers::AnySlice::Text(t) => {
                                let txt = String::from_utf8(t.get(row_index).unwrap().to_vec());
                                println!("{:?}", txt)
                            }
                            odbc_api::buffers::AnySlice::WText(_) => todo!(),
                            odbc_api::buffers::AnySlice::Binary(_) => todo!(),
                            odbc_api::buffers::AnySlice::Date(_) => todo!(),
                            odbc_api::buffers::AnySlice::Time(_) => todo!(),
                            odbc_api::buffers::AnySlice::Timestamp(_) => todo!(),
                            odbc_api::buffers::AnySlice::F64(_) => todo!(),
                            odbc_api::buffers::AnySlice::F32(_) => todo!(),
                            odbc_api::buffers::AnySlice::I8(_) => todo!(),
                            odbc_api::buffers::AnySlice::I16(_) => todo!(),
                            odbc_api::buffers::AnySlice::I32(_) => todo!(),
                            odbc_api::buffers::AnySlice::I64(_) => todo!(),
                            odbc_api::buffers::AnySlice::U8(_) => todo!(),
                            odbc_api::buffers::AnySlice::Bit(_) => todo!(),
                            odbc_api::buffers::AnySlice::NullableDate(_) => todo!(),
                            odbc_api::buffers::AnySlice::NullableTime(_) => todo!(),
                            odbc_api::buffers::AnySlice::NullableTimestamp(t) => {
                                println!("{:?}", t.skip(row_index).next().unwrap())
                            },
                            odbc_api::buffers::AnySlice::NullableF64(_) => todo!(),
                            odbc_api::buffers::AnySlice::NullableF32(f) => {
                                let x = f.skip(row_index).next().unwrap();
                                println!("{:?}", x);
                            },
                            odbc_api::buffers::AnySlice::NullableI8(_) => todo!(),
                            odbc_api::buffers::AnySlice::NullableI16(n) => {
                                let x = n.skip(row_index).next().unwrap();
                                println!("{:?}", x);
                                println!("{:#?}", n);
                            },
                            odbc_api::buffers::AnySlice::NullableI32(_) => todo!(),
                            odbc_api::buffers::AnySlice::NullableI64(n) => {
                                let x = n.skip(row_index).next().unwrap();
                                println!("{:?}", x);
                            },
                            odbc_api::buffers::AnySlice::NullableU8(_) => todo!(),
                            odbc_api::buffers::AnySlice::NullableBit(_) => todo!(),
                        }
                    }
                }
            }
        }
        None => {
            eprintln!("Query came back empty. No output has been created.");
        }
    }

    Ok(())
}
