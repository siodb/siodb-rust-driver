# Rust driver for Siodb

A simple driver for Siodb written in pure Rust.

## Features

- Support of URI
- Connections to Siodb (TLS, TCP, Unix socket)
- Authentication to Siodb
- Query execution
- DML execution

## Installation

Add the crate dependency with the version you desire into `Cargo.toml`:

```
[dependencies]
siodb = "*"
```

## Quick start

### Docker
Start Siodb in a container and get the RSA key for root user locally:

```bash
docker run -p 127.0.0.1:50000:50000/tcp --name siodb siodb/siodb
docker exec -it siodb cat /home/siodb/.ssh/id_rsa > ~/root_id_rsa
```

### Cloud

[![Deploy to Hidora](https://raw.githubusercontent.com/siodb/siodb-jelastic/master/images/deploy-to-hidora.png)](https://siodb.hidora.com)

*No credit card required. Free Trial.

## Example

```rust
    let uri = "siodbs://root@localhost:50000?identity_file=/home/nico/root_id_rsa";

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

    println!("Row(s): {}", siodb_conn.get_row_count());

    siodb_conn.close().unwrap();
```

## URI

To identify a Siodb resource, the driver use the
[URI format](https://en.wikipedia.org/wiki/Uniform_Resource_Identifier).

For TLS connection (default):

```
siodbs://root@localhost:50000?identity_file=/home/siodb/.ssh/id_rsa
```

For TCP plain text connection:

```
siodb://root@localhost:50000?identity_file=/home/siodb/.ssh/id_rsa
```

For Unix socket connection:

```
siodbu:/run/siodb/siodb.socket?identity_file=/home/siodb/.ssh/id_rsa
```

The above example will connect you to the localhost with port number `50000`.
The driver will do the authentication with the Siodb user root and the identity file `/home/siodb/.ssh/id_rsa`.

### Options

- identity_file: the path to the RSA private key.
- trace: to trace everything within the driver to sdtout.

## Support Siodb

Do you like this project? Tell it by clicking the star 🟊 on the top right of this page ☝☝

## Documentation

We write the Siodb documentation in Markdow and it is available in the folder `docs/users/docs`.
If you prefer a more user friendly format, the same documentation is
available online [here]( https://docs.siodb.io).

## Contribution

Please refer to the [Contributing file](CONTRIBUTING.md).

## Support

- Report your issue with Siodb 👉 [here](https://github.com/siodb/siodb/issues/new).
- Report your issue with the driver 👉 [here](https://github.com/siodb/siodb-rust-driver/issues/new).
- Ask a question 👉 [here](https://stackoverflow.com/questions/tagged/siodb).
- Siodb Slack space 👉 [here](https://join.slack.com/t/siodb-squad/shared_invite/zt-e766wbf9-IfH9WiGlUpmRYlwCI_28ng).

## Follow Siodb

- [Twitter](https://twitter.com/Sio_db)
- [Linkedin](https://www.linkedin.com/company/siodb)

## License

Licensed under [Apache License version 2.0](https://www.apache.org/licenses/LICENSE-2.0).

