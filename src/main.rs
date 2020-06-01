// Copyright (C) 2019-2020 Siodb GmbH. All rights reserved.
// Use of this source code is governed by a license that can be found
// in the LICENSE file.

// Siodb lib crate
mod siodb;
use crate::siodb::SiodbConn;

// Standard
use std::time::Instant;

fn main() -> Result<(), std::io::Error> {
    // TLS connection (default)
    let uri = "siodbs://root@localhost:50000?identity_file=/home/nico/root_id_rsa";
    // TCP plain text connection
    //let uri = "siodb://root@localhost:50000?identity_file=/home/nico/root_id_rsa";
    // Local Unix socket connection
    //let uri = "siodbu:/run/siodb/siodb.socket?identity_file=/home/siodb/.ssh/id_rsa";
    let mut siodb_conn = SiodbConn::new(&uri).expect(&format!("Error connecting to URI '{}'", uri));

    if siodb_conn
        .query_row("select name from sys_databases where name = 'TEST_DB'".to_string())
        .is_none()
    {
        siodb_conn
            .execute("CREATE DATABASE test_db".to_string())
            .expect(&format!("Database creation error."));
    }

    if siodb_conn
        .query_row("select name from test_db.sys_tables where name = 'TEST_TABLE'".to_string())
        .is_none()
    {
        siodb_conn
            .execute(
                "CREATE TABLE test_db.test_table
                (
                   ctinyintmin  TINYINT,
                   ctinyintmax  TINYINT,
                   ctinyuint    TINYUINT,
                   csmallintmin SMALLINT,
                   csmallintmax SMALLINT,
                   csmalluint   SMALLUINT,
                   cintmin      INT,
                   cintmax      INT,
                   cuint        UINT,
                   cbigintmin   BIGINT,
                   cbigintmax   BIGINT,
                   cbiguint     BIGUINT,
                   cfloatmin    FLOAT,
                   cfloatmax    FLOAT,
                   cdoublemin   DOUBLE,
                   cdoublemax   DOUBLE,
                   ctext        TEXT,
                   cts          TIMESTAMP
                )"
                .to_string(),
            )
            .expect(&format!("Table creation error."));
    }

    siodb_conn
        .execute(
            "INSERT INTO test_db.test_table
             VALUES      ( -128,
                           127,
                           255,
                           -32768,
                           32767,
                           65535,
                           -2147483648,
                           2147483647,
                           4294967295,
                           -9223372036854775808,
                           9223372036854775807,
                           18446744073709551615,
                           222.222,
                           222.222,
                           222.222,
                           222.222,
                           '汉字',
                           CURRENT_TIMESTAMP ) "
                .to_string(),
        )
        .expect(&format!("Insertion error."));

    println!("Affected row(s): {}", siodb_conn.get_affected_row_count());

    let start = Instant::now();

    siodb_conn
        .query("select * from test_db.test_table".to_string())
        .expect(&format!("Query error"));

    while siodb_conn.next().unwrap() {
        for data in siodb_conn.scan() {
            if data.is_none() {
                println!("Value: Null");
            } else {
                println!("Value: {}", data.as_ref().unwrap());
            }
        }
    }

    let duration = start.elapsed();
    println!(
        "Row(s): {} | Elapsed time: {:?}",
        siodb_conn.get_row_count(),
        duration
    );

    siodb_conn.close().unwrap();

    Ok(())
}
