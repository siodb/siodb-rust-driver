// Copyright (C) 2019-2020 Siodb GmbH. All rights reserved.
// Use of this source code is governed by a license that can be found
// in the LICENSE file.

// TODO: Connection pool
// TODO: Prepared statements implementation (when Siodb supports it)

mod errors;
use errors::debug;
use errors::DriverError;

// ResultSet
mod results;
use results::ResultSet;
use results::Value;

// Standard
use std::convert::TryInto;
use std::fmt;
use std::fs;
use std::io::BufRead;
use std::net::Shutdown;
use std::net::TcpStream;
use std::os::unix::net::UnixStream;
use std::str::from_utf8;
use std::str::FromStr;

// DateTime
use chrono::prelude::*;

// Url for URI
use url::Url;

// OpenSSL
use bufstream::BufStream;
use native_tls::{TlsConnector, TlsStream};
use openssl::hash::MessageDigest;
use openssl::pkey::PKey;
use openssl::rsa::Rsa;
use openssl::sign::Signer;
use std::io::{Read, Write};

// Byte order
use byteorder::{ByteOrder, LittleEndian};

// Protobuf
mod ClientProtocol;
use ClientProtocol::{
    BeginSessionRequest, BeginSessionResponse, ClientAuthenticationRequest,
    ClientAuthenticationResponse, Command, ServerResponse,
};
mod ColumnDataType;
mod CommonTypes;

enum ConnBufStream {
    PlainBufUnixSocket(BufStream<UnixStream>),
    PlainBufStream(BufStream<TcpStream>),
    TlsBufStream(BufStream<TlsStream<TcpStream>>),
}

impl Read for ConnBufStream {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        match *self {
            ConnBufStream::PlainBufUnixSocket(ref mut stream) => stream.read(buf),
            ConnBufStream::PlainBufStream(ref mut stream) => stream.read(buf),
            ConnBufStream::TlsBufStream(ref mut stream) => stream.read(buf),
        }
    }
}

impl BufRead for ConnBufStream {
    fn fill_buf(&mut self) -> std::result::Result<&[u8], std::io::Error> {
        match *self {
            ConnBufStream::PlainBufUnixSocket(ref mut stream) => stream.fill_buf(),
            ConnBufStream::PlainBufStream(ref mut stream) => stream.fill_buf(),
            ConnBufStream::TlsBufStream(ref mut stream) => stream.fill_buf(),
        }
    }
    fn consume(&mut self, amt: usize) {
        match *self {
            ConnBufStream::PlainBufUnixSocket(ref mut stream) => stream.consume(amt),
            ConnBufStream::PlainBufStream(ref mut stream) => stream.consume(amt),
            ConnBufStream::TlsBufStream(ref mut stream) => stream.consume(amt),
        }
    }
}

impl Write for ConnBufStream {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        match *self {
            ConnBufStream::PlainBufUnixSocket(ref mut stream) => stream.write(buf),
            ConnBufStream::PlainBufStream(ref mut stream) => stream.write(buf),
            ConnBufStream::TlsBufStream(ref mut stream) => stream.write(buf),
        }
    }

    fn flush(&mut self) -> std::io::Result<()> {
        match *self {
            ConnBufStream::PlainBufUnixSocket(ref mut stream) => stream.flush(),
            ConnBufStream::PlainBufStream(ref mut stream) => stream.flush(),
            ConnBufStream::TlsBufStream(ref mut stream) => stream.flush(),
        }
    }
}

enum ConnStream {
    UnixSocketStream(UnixStream),
    TcpStream(TcpStream),
}

impl ConnStream {
    fn shutdown(&mut self) -> std::io::Result<()> {
        match *self {
            ConnStream::UnixSocketStream(ref mut stream) => stream.shutdown(Shutdown::Both),
            ConnStream::TcpStream(ref mut stream) => stream.shutdown(Shutdown::Both),
        }
    }
}

/// A connection to Siodb.
///
/// ## For example:
///
/// ```rust
///   let uri = "siodbs://root@localhost:50000?identity_file=/home/siodb/.ssh/id_rsa";
///   let mut siodb_conn = SiodbConn::new(&uri).expect(&format!("Error connecting to URI '{}'", uri));
/// ```
pub struct SiodbConn {
    scheme: String,
    host: String,
    port: u16,
    user: String,
    pkfile: String,
    trace: bool,
    stream: Option<ConnStream>,
    buf_stream: Option<ConnBufStream>,
    result_set: Option<ResultSet>,
}

impl fmt::Debug for SiodbConn {
    // This trait requires `fmt` with this exact signature.
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "scheme: {} | host: {} | port: {} | user: {} | pkfile: {} | trace: {}",
            self.scheme, self.host, self.port, self.user, self.pkfile, self.trace,
        )
    }
}

impl SiodbConn {
    fn parse_uri(uri_str: &str) -> Result<SiodbConn, DriverError> {
        let uri = Url::parse(uri_str).expect(&format!("Unable to parse URI"));

        let pairs = uri.query_pairs();
        let mut pkfile = "~/.ssh/id_rsa".to_string();
        let mut trace = false;
        for pair in pairs {
            match pair.0 {
                _ if pair.0.to_string() == String::from("identity_file") => {
                    pkfile = pair.1.to_string()
                }
                _ if pair.0.to_string() == String::from("trace") => {
                    trace = bool::from_str(&pair.1.to_string()).unwrap_or(trace)
                }
                _ => return Err(DriverError::new(&format!("Unknow option: {}.", &pair.0))),
            }
        }

        // Derive values
        let mut scheme: String = "siodbs".to_string();
        if uri.scheme().len() > 0 {
            scheme = uri.scheme().to_string();
        }
        if scheme != "siodb" && scheme != "siodbs" && scheme != "siodbu" {
            return Err(DriverError::new(&format!(
                "Wrong protocol: '{}'. Should be 'siodb', 'siodbs' or 'siodbu'.",
                scheme
            )));
        }
        let mut host: String = "localhost".to_string();
        if scheme == "siodbu".to_string() {
            host = uri.to_file_path().unwrap().to_str().unwrap().try_into().unwrap();
        } else {
        if uri.host().is_some() {
            host = uri.host().unwrap().to_string();
        }}
        let port = uri.port().unwrap_or(50000);
        let mut user: String = "root".to_string();
        if uri.username().len() > 0 {
            user = uri.username().to_string();
        }

        Ok(SiodbConn {
            scheme,
            host,
            port,
            user,
            pkfile,
            trace,
            stream: None,
            buf_stream: None,
            result_set: None,
        })
    }

    /// Create a new authenticated connection to Siodb from an URI.
    pub fn new(uri_str: &str) -> Result<SiodbConn, DriverError> {
        let mut siodb_conn = SiodbConn::parse_uri(uri_str).unwrap();
        debug(siodb_conn.trace, &format!("siodb_conn: {:?}", siodb_conn));
        siodb_conn.connect()?;
        siodb_conn.authenticate()?;
        Ok(siodb_conn)
    }
    fn connect(&mut self) -> Result<(), DriverError> {
        match self.scheme.as_str() {
            "siodb" => {
                // TCP connection
                let stream = TcpStream::connect(format!("{}:{}", self.host, self.port))
                    .expect(&format!("Cannot connect to '{}:{}'", self.host, self.port));
                self.stream = Some(ConnStream::TcpStream(stream.try_clone().unwrap()));
                self.buf_stream = Some(ConnBufStream::PlainBufStream(BufStream::new(stream)));
            }
            "siodbs" => {
                // TLS connection
                // TODO: handle private key password
                // TODO: implement certificat validation
                let mut builder = TlsConnector::builder();
                builder.danger_accept_invalid_hostnames(true);
                builder.danger_accept_invalid_certs(true);
                let tls_connector = builder.build().unwrap();
                let stream = TcpStream::connect(format!("{}:{}", self.host, self.port))
                    .expect(&format!("Cannot connect to '{}:{}'", self.host, self.port));
                self.stream = Some(ConnStream::TcpStream(stream.try_clone().unwrap()));
                let stream = tls_connector.connect(&self.host, stream).unwrap();
                self.buf_stream = Some(ConnBufStream::TlsBufStream(BufStream::new(stream)));
            }
            "siodbu" => {
                // Unix socket connection
                let stream = UnixStream::connect(format!("{}", self.host))
                    .expect(&format!("Cannot connect to socket '{}'", self.host));
                self.stream = Some(ConnStream::UnixSocketStream(stream.try_clone().unwrap()));
                self.buf_stream = Some(ConnBufStream::PlainBufUnixSocket(BufStream::new(stream)));
            }
            _ => {
                return Err(DriverError::new(&format!("Protocol unknown.")));
            }
        }

        Ok(())
    }

    /// Close the connection with Siodb.
    pub fn close(&mut self) -> Result<(), DriverError> {
        self.stream
            .as_mut()
            .unwrap()
            .shutdown()
            .expect(&format!("Error while closing connection."));

        Ok(())
    }
    fn authenticate(&mut self) -> Result<(), DriverError> {
        // Begin session request
        let mut begin_session_request = BeginSessionRequest::new();
        begin_session_request.set_user_name(self.user.as_str().to_string());
        debug(
            self.trace,
            &format!("begin_session_request: {:?}", begin_session_request),
        );
        self.write_message(5, &begin_session_request)?;

        // Read Session response
        let _begin_session_response = self.read_message::<BeginSessionResponse>(6).unwrap()?;

        if !_begin_session_response.get_session_started() {
            return Err(DriverError::new(&format!("Siodb session not started.")));
        }

        // Hash and Sign challenge
        let pkey = &self.pkfile;
        let contents =
            fs::read_to_string(pkey).expect(&format!("Error reading private key '{}'", pkey));
        let keypair = Rsa::private_key_from_pem(contents.as_bytes())
            .expect(&format!("Error loading private key"));
        let keypair =
            PKey::from_rsa(keypair).expect(&format!("Error loading private {} key", "RSA"));
        let mut signer = Signer::new(MessageDigest::sha512(), &keypair)
            .expect(&format!("Error creating signer"));
        let signature = signer
            .sign_oneshot_to_vec(_begin_session_response.get_challenge())
            .expect(&format!("Error signing challenge"));

        // Start authentication
        let mut client_authentication_request = ClientAuthenticationRequest::new();
        client_authentication_request.set_signature(signature);
        debug(
            self.trace,
            &format!(
                "client_authentication_request: {:?}",
                client_authentication_request
            ),
        );
        self.write_message(7, &client_authentication_request)?;

        // Read Session response
        let _client_authentication_response = self
            .read_message::<ClientAuthenticationResponse>(8)
            .unwrap()?;

        if !_client_authentication_response.get_authenticated() {
            return Err(DriverError::new(&format!("Siodb session not started.")));
        }

        Ok(())
    }
    fn write_message(
        &mut self,
        message_type: u32,
        message: &dyn protobuf::Message,
    ) -> Result<(), DriverError> {
        let mut output_stream = self.buf_stream.as_mut().unwrap();
        let mut coded_output_stream = protobuf::CodedOutputStream::new(&mut output_stream);

        coded_output_stream
            .write_raw_varint32(message_type)
            .expect(&format!("write_message | Codec error"));

        coded_output_stream
            .write_raw_varint32(message.compute_size())
            .expect(&format!("write_message | Codec error"));
        &message
            .write_to_with_cached_sizes(&mut coded_output_stream)
            .expect(&format!("write_message | Codec error"));
        coded_output_stream
            .flush()
            .expect(&format!("write_message | Codec error"));

        self.buf_stream
            .as_mut()
            .unwrap()
            .flush()
            .expect(&format!("write_message | Codec error"));

        Ok(())
    }
    fn read_message<M: protobuf::Message>(
        &mut self,
        message_type: u32,
    ) -> Result<protobuf::ProtobufResult<M>, DriverError> {
        let mut input_stream = self.buf_stream.as_mut().unwrap();
        let mut coded_input_stream =
            protobuf::CodedInputStream::from_buffered_reader(&mut input_stream);

        let message_type_received = coded_input_stream
            .read_raw_varint32()
            .expect(&format!("read_message | Codec error"));
        debug(self.trace, &format!("message_type: {:?}", message_type));
        if message_type != message_type_received {
            return Err(DriverError::new(&format!(
                "read_message | wrong message type received from Siodb: {}. Expected: {}.",
                message_type_received, message_type
            )));
        }
        let message = coded_input_stream
            .read_message()
            .expect(&format!("read_message | Codec error"));

        Ok(Ok(message))
    }
    /// Execute a statement in a connection.
    pub fn execute(&mut self, sql: String) -> Result<(), DriverError> {
        if self.result_set.is_some() && !self.result_set.as_mut().unwrap().end_of_row {
            return Err(DriverError::new(&format!(
                "execute | There is still data in the buffer."
            )));
        }

        // Send command
        let mut command = Command::new();
        command.set_request_id(1);
        command.set_text(sql);
        debug(self.trace, &format!("command: {:?}", command));
        self.write_message(1, &command)?;

        // Read server response
        self.result_set = Some(ResultSet::new(
            self.read_message::<ServerResponse>(2).unwrap()?,
        )?);
        debug(
            self.trace,
            &format!(
                "ServerResponse: {:?}",
                self.result_set.as_ref().unwrap().server_response
            ),
        );

        // Check if error arrives from Siodb server
        if self
            .result_set
            .as_ref()
            .unwrap()
            .server_response
            .message
            .len()
            > 0
        {
            let mut error_messages = String::new();
            for column in &self.result_set.as_ref().unwrap().server_response.message {
                error_messages = error_messages + &column.text.to_string();
            }
            return Err(DriverError::new(&format!(
                "execute | Error message(s) {}.",
                error_messages
            )));
        }

        // Check dataset presence
        let column_count = self
            .result_set
            .as_ref()
            .unwrap()
            .server_response
            .get_column_description()
            .len();

        if column_count > 0 {
            self.result_set.as_mut().unwrap().end_of_row = false;
            debug(
                self.trace,
                &format!(
                    "Dataset present in the the server's response with {} colmuns.",
                    self.result_set
                        .as_ref()
                        .unwrap()
                        .server_response
                        .get_column_description()
                        .len()
                ),
            );

            // Check if nullbitmask present
            for column in &self
                .result_set
                .as_ref()
                .unwrap()
                .server_response
                .column_description
            {
                if column.is_null {
                    self.result_set.as_mut().unwrap().null_bit_mask_present = true;
                    debug(self.trace, &format!("null_bit_mask_present: true."));
                    // Get nul bitmask byte size
                    if column_count % 8 == 0 {
                        self.result_set.as_mut().unwrap().null_bit_mask_byte_size =
                            (column_count / 8).try_into().unwrap();
                    } else {
                        self.result_set.as_mut().unwrap().null_bit_mask_byte_size =
                            (column_count / 8 + 1).try_into().unwrap();
                    }
                    debug(
                        self.trace,
                        &format!(
                            "null_bit_mask_byte_size: {}.",
                            self.result_set.as_mut().unwrap().null_bit_mask_byte_size
                        ),
                    );
                    break;
                }
            }
        }

        Ok(())
    }

    /// Execute a query in a connection, return the first row and discard the others.
    pub fn query_row(&mut self, sql: String) -> Option<Vec<Option<Value>>> {
        let mut row: Option<Vec<Option<Value>>> = None;
        self.execute(sql).unwrap();
        if self.next().unwrap() {
            row = Some(self.scan().to_vec());
        }
        // Skip others rows if any.
        while self.next().unwrap() {}
        row
    }
    /// Execute a query in a connection.
    pub fn query(&mut self, sql: String) -> Result<(), DriverError> {
        self.execute(sql)
    }
    /// Read the next row from the result set.
    pub fn next(&mut self) -> Result<bool, DriverError> {
        let mut row = Vec::<Option<Value>>::new();
        let mut input_stream = self.buf_stream.as_mut().unwrap();
        let mut coded_input_stream =
            protobuf::CodedInputStream::from_buffered_reader(&mut input_stream);

        debug(self.trace, &format!("ResultSet.next() | ---"));

        if self.result_set.as_ref().unwrap().end_of_row {
            return Ok(false);
        }

        let row_length = coded_input_stream
            .read_raw_varint32()
            .expect(&format!("Codec error"));
        debug(self.trace, &format!("Row bytes row_length: {}", row_length));
        if row_length == 0 {
            self.result_set.as_mut().unwrap().end_of_row = true;
            return Ok(false);
        } else {
            self.result_set.as_mut().unwrap().row_count += 1;
        }

        // Read null Bitmask to figure out null value which are not streamed.
        let mut bit_mask: Vec<u8> = Vec::new();
        if self.result_set.as_ref().unwrap().null_bit_mask_present {
            bit_mask = coded_input_stream
                .read_raw_bytes(self.result_set.as_ref().unwrap().null_bit_mask_byte_size as u32)
                .unwrap();
            debug(
                self.trace,
                &format!("ResultSet.next() | Bitmask value: {:?}.", bit_mask),
            );
        }

        // Read Row data
        let mut is_null: u8 = 0;
        for (idx, column) in self
            .result_set
            .as_ref()
            .unwrap()
            .server_response
            .column_description
            .iter()
            .enumerate()
        {
            if self.result_set.as_ref().unwrap().null_bit_mask_present {
                let mask = 1 << (idx % 8);
                is_null = (bit_mask[idx / 8] & mask) >> (idx % 8);
                debug(
                    self.trace,
                    &format!(
                        "ResultSet.next() | Is that cell (id: {:?} ) null?: {:?}.",
                        idx, is_null
                    ),
                );
            }

            if is_null == 1 {
                row.push(None)
            } else {
                debug(
                    self.trace,
                    &format!("read_data | data type: {:?}.", column.field_type),
                );
                match column.field_type {
                    ColumnDataType::ColumnDataType::COLUMN_DATA_TYPE_INT8 => row.push(Some(
                        Value::Int8(coded_input_stream.read_raw_bytes(1).unwrap()[0] as i8),
                    )),
                    ColumnDataType::ColumnDataType::COLUMN_DATA_TYPE_UINT8 => row.push(Some(
                        Value::Uint8(coded_input_stream.read_raw_bytes(1).unwrap()[0] as u8),
                    )),
                    ColumnDataType::ColumnDataType::COLUMN_DATA_TYPE_INT16 => {
                        row.push(Some(Value::Int16(LittleEndian::read_i16(
                            &coded_input_stream.read_raw_bytes(2).unwrap(),
                        ))))
                    }
                    ColumnDataType::ColumnDataType::COLUMN_DATA_TYPE_UINT16 => {
                        row.push(Some(Value::Uint16(LittleEndian::read_u16(
                            &coded_input_stream.read_raw_bytes(2).unwrap(),
                        ))))
                    }
                    ColumnDataType::ColumnDataType::COLUMN_DATA_TYPE_INT32 => row.push(Some(
                        Value::Int32(coded_input_stream.read_raw_varint32().unwrap() as i32),
                    )),
                    ColumnDataType::ColumnDataType::COLUMN_DATA_TYPE_UINT32 => row.push(Some(
                        Value::Uint32(coded_input_stream.read_raw_varint32().unwrap()),
                    )),

                    ColumnDataType::ColumnDataType::COLUMN_DATA_TYPE_FLOAT => {
                        row.push(Some(Value::Float(coded_input_stream.read_float().unwrap())))
                    }
                    ColumnDataType::ColumnDataType::COLUMN_DATA_TYPE_DOUBLE => row.push(Some(
                        Value::Double(coded_input_stream.read_double().unwrap()),
                    )),

                    ColumnDataType::ColumnDataType::COLUMN_DATA_TYPE_INT64 => row.push(Some(
                        Value::Int64(coded_input_stream.read_raw_varint64().unwrap() as i64),
                    )),
                    ColumnDataType::ColumnDataType::COLUMN_DATA_TYPE_UINT64 => row.push(Some(
                        Value::Uint64(coded_input_stream.read_raw_varint64().unwrap()),
                    )),
                    ColumnDataType::ColumnDataType::COLUMN_DATA_TYPE_TEXT => {
                        let data_length = coded_input_stream.read_raw_varint32().unwrap();
                        row.push(Some(Value::Text(
                            from_utf8(&coded_input_stream.read_raw_bytes(data_length).unwrap())
                                .unwrap()
                                .to_string(),
                        )));
                    }
                    ColumnDataType::ColumnDataType::COLUMN_DATA_TYPE_BINARY => {
                        let data_length = coded_input_stream.read_raw_varint32().unwrap();
                        row.push(Some(Value::Binary(
                            coded_input_stream.read_raw_bytes(data_length).unwrap(),
                        )));
                    }
                    ColumnDataType::ColumnDataType::COLUMN_DATA_TYPE_TIMESTAMP => {
                        let has_time_part: u8;
                        let year: i32;
                        let month: u8;
                        let day_of_week: u8;
                        let day_of_month: u8;
                        let mut hours: u8 = 0;
                        let mut minutes: u8 = 0;
                        let mut seconds: u8 = 0;
                        let mut nano: u32 = 0;
                        // Get date part, 4 first bytes
                        let date = coded_input_stream.read_raw_bytes(4).unwrap();
                        debug(
                            self.trace,
                            &format!(
                                "Binary timestamp: {:08b} {:08b} {:08b} {:08b} ",
                                date[0], date[1], date[2], date[3]
                            ),
                        );
                        has_time_part = date[0] & 0b0000_0001;
                        day_of_week = (date[0] & 0b0000_1110) >> 1;
                        day_of_month =
                            (((date[0] & 0b1111_0000) >> 4) + ((date[1] & 0b0000_0001) << 4)) + 1;
                        month = ((date[1] & 0b0001_1110) >> 1) + 1;
                        let year_bytes = [
                            0b0000_0000,
                            (date[3] & 0b1110_0000) >> 5,
                            ((date[2] & 0b1110_0000) >> 5) + ((date[3] & 0b0001_1111) << 3),
                            ((date[1] & 0b1110_0000) >> 5) + ((date[2] & 0b0001_1111) << 3),
                        ];
                        year = unsafe { std::mem::transmute::<[u8; 4], i32>(year_bytes) }.to_be();
                        debug(
                            self.trace,
                            &format!(
                                "hasTimePart: {:?} | dayOfWeek: {:?} | dayOfMonth: {:?} | month: {:?} | year: {:?} ",
                                has_time_part, day_of_week, day_of_month, month, year
                            ),
                        );
                        if has_time_part == 1 {
                            // Get time part, 6 last bytes
                            let time = coded_input_stream.read_raw_bytes(6).unwrap();
                            let nano_bytes = [
                                ((time[3] & 0b0111_1110) >> 1),
                                ((time[2] & 0b1111_1110) >> 1) + ((time[3] & 0b0000_0001) << 7),
                                ((time[1] & 0b1111_1110) >> 1) + ((time[2] & 0b0000_0001) << 7),
                                ((time[0] & 0b1111_1110) >> 1) + ((time[1] & 0b0000_0001) << 7),
                            ];
                            nano =
                                unsafe { std::mem::transmute::<[u8; 4], u32>(nano_bytes) }.to_be();
                            seconds =
                                ((time[3] & 0b1000_0000) >> 7) + ((time[4] & 0b0001_1111) << 1);
                            minutes =
                                ((time[4] & 0b1110_0000) >> 5) + ((time[5] & 0b0000_0111) << 3);
                            hours = (time[5] & 0b1111_1000) >> 3;
                            debug(
                                self.trace,
                                &format!(
                                    "hours: {:?} | minutes: {:?} | seconds: {:?} | nano: {:?} | nano_bytes: {:?}",
                                    hours, minutes, seconds as u32, nano, nano_bytes
                                ),
                            );
                        }
                        row.push(Some(Value::Timestamp(
                            Utc.ymd(year, month.into(), day_of_month.into())
                                .and_hms_nano(hours.into(), minutes.into(), seconds.into(), nano),
                        )));
                    }
                    _ => {
                        return Err(DriverError::new(&format!(
                            "read_data | Unknow data type: {:?}.",
                            column.field_type
                        )))
                    }
                }
            }
        }

        self.result_set.as_mut().unwrap().current_row = Some(row);

        Ok(true)
    }

    /// Return last row fetched from next().
    pub fn scan(&self) -> &Vec<Option<Value>> {
        self.result_set
            .as_ref()
            .unwrap()
            .current_row
            .as_ref()
            .unwrap()
    }

    /// Return the total number of rows read to far from next().
    pub fn get_row_count(&mut self) -> u64 {
        self.result_set.as_ref().unwrap().row_count
    }

    /// Return the number of affected rows from the previous statement.
    pub fn get_affected_row_count(&mut self) -> u64 {
        if self
            .result_set
            .as_ref()
            .unwrap()
            .server_response
            .get_has_affected_row_count()
        {
            self.result_set
                .as_ref()
                .unwrap()
                .server_response
                .get_affected_row_count()
        } else {
            0
        }
    }
}
