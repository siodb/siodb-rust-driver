// Copyright (C) 2019-2020 Siodb GmbH. All rights reserved.
// Use of this source code is governed by a license that can be found
// in the LICENSE file.

// Siodb
use crate::siodb::errors::DriverError;

// Protobuf
use crate::siodb::ClientProtocol::ServerResponse;

// Standard
use std::fmt;

// DateTime
use chrono::prelude::*;

pub struct ResultSet {
    pub server_response: ServerResponse,
    pub null_bit_mask_present: bool,
    pub null_bit_mask_byte_size: u8,
    pub end_of_row: bool,
    pub row_count: u64,
    pub current_row: Option<Vec<Option<Value>>>,
}

impl ResultSet {
    pub fn new(server_response: ServerResponse) -> Result<ResultSet, DriverError> {
        Ok(ResultSet {
            server_response: server_response,
            null_bit_mask_present: false,
            null_bit_mask_byte_size: 0,
            end_of_row: true,
            current_row: None,
            row_count: 0,
        })
    }
}

pub enum Value {
    // Bool(bool),
    Int8(i8),
    Uint8(u8),
    Int16(i16),
    Uint16(u16),
    Int32(i32),
    Uint32(u32),
    Int64(i64),
    Uint64(u64),
    Float(f32),
    Double(f64),
    Text(String),
    // Ntext(String),
    Binary(Vec<u8>),
    // Date(),
    // Time(),
    // TimeWithTz(),
    Timestamp(DateTime<Utc>),
    // TimestampWithTz(),
    // DateInternal(),
    // TimeInternal(),
    // Struct(),
    // Xml(),
    // Json(),
    // Uuid(),
    // Maw(),
    // Unknown(),
}

impl fmt::Display for Value {
    // This trait requires `fmt` with this exact signature.
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match &*self {
            Value::Int8(c) => write!(f, "{}", c),
            Value::Uint8(c) => write!(f, "{}", c),
            Value::Int16(c) => write!(f, "{}", c),
            Value::Uint16(c) => write!(f, "{}", c),
            Value::Int32(c) => write!(f, "{}", c),
            Value::Uint32(c) => write!(f, "{}", c),
            Value::Int64(c) => write!(f, "{}", c),
            Value::Uint64(c) => write!(f, "{}", c),
            Value::Float(c) => write!(f, "{}", c),
            Value::Double(c) => write!(f, "{}", c),
            Value::Text(c) => write!(f, "{}", c),
            Value::Binary(_) => write!(f, "Binary string"),
            Value::Timestamp(c) => write!(f, "{}", c),
        }
    }
}

impl fmt::Debug for Value {
    // This trait requires `fmt` with this exact signature.
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match &*self {
            Value::Int8(c) => write!(f, "Value: ->{:?}<-", c),
            Value::Uint8(c) => write!(f, "Value: ->{:?}<-", c),
            Value::Int16(c) => write!(f, "Value: ->{:?}<-", c),
            Value::Uint16(c) => write!(f, "Value: ->{:?}<-", c),
            Value::Int32(c) => write!(f, "Value: ->{:?}<-", c),
            Value::Uint32(c) => write!(f, "Value: ->{:?}<-", c),
            Value::Int64(c) => write!(f, "Value: ->{:?}<-", c),
            Value::Uint64(c) => write!(f, "Value: ->{:?}<-", c),
            Value::Float(c) => write!(f, "Value: ->{:?}<-", c),
            Value::Double(c) => write!(f, "Value: ->{:?}<-", c),
            Value::Text(c) => write!(f, "Value: ->{:?}<-", c),
            Value::Binary(c) => write!(f, "Value: ->{:?}<-", c),
            Value::Timestamp(c) => write!(f, "Value: ->{:?}<-", c),
        }
    }
}

impl Clone for Value {
    fn clone(&self) -> Value {
        match &*self {
            Value::Int8(c) => return Value::Int8(c.clone()),
            Value::Uint8(c) => return Value::Uint8(c.clone()),
            Value::Int16(c) => return Value::Int16(c.clone()),
            Value::Uint16(c) => return Value::Uint16(c.clone()),
            Value::Int32(c) => return Value::Int32(c.clone()),
            Value::Uint32(c) => return Value::Uint32(c.clone()),
            Value::Int64(c) => return Value::Int64(c.clone()),
            Value::Uint64(c) => return Value::Uint64(c.clone()),
            Value::Float(c) => return Value::Float(c.clone()),
            Value::Double(c) => return Value::Double(c.clone()),
            Value::Text(c) => return Value::Text(c.clone()),
            Value::Binary(c) => return Value::Binary(c.clone()),
            Value::Timestamp(c) => return Value::Timestamp(c.clone()),
        }
    }
}
