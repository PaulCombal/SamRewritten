/* Copyright (c) 2024 Rick (rick 'at' gibbed 'dot' us)
 *
 * This software is provided 'as-is', without any express or implied
 * warranty. In no event will the authors be held liable for any damages
 * arising from the use of this software.
 *
 * Permission is granted to anyone to use this software for any purpose,
 * including commercial applications, and to alter it and redistribute it
 * freely, subject to the following restrictions:
 *
 * 1. The origin of this software must not be misrepresented; you must not
 *    claim that you wrote the original software. If you use this software
 *    in a product, an acknowledgment in the product documentation would
 *    be appreciated but is not required.
 *
 * 2. Altered source versions must be plainly marked as such, and must not
 *    be misrepresented as being the original software.
 *
 * 3. This notice may not be removed or altered from any source
 *    distribution.
 */

use crate::backend::types::KeyValueEncoding;
use std::collections::HashMap;
use std::error::Error;
use std::fmt;
use std::fs::File;
use std::io::{Read, Seek};
use std::path::Path;
use std::sync::LazyLock;

#[derive(Debug)]
pub enum KeyValueError {
    Io(std::io::Error),
    Format(String),
    UnsupportedType(KeyValueType),
}

impl fmt::Display for KeyValueError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            KeyValueError::Io(err) => write!(f, "IO error: {}", err),
            KeyValueError::Format(msg) => write!(f, "Format error: {}", msg),
            KeyValueError::UnsupportedType(typ) => write!(f, "Unsupported type: {:?}", typ),
        }
    }
}

impl Error for KeyValueError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            KeyValueError::Io(err) => Some(err),
            _ => None,
        }
    }
}

impl From<std::io::Error> for KeyValueError {
    fn from(err: std::io::Error) -> Self {
        KeyValueError::Io(err)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KeyValueType {
    None = 0,
    String = 1,
    Int32 = 2,
    Float32 = 3,
    Pointer = 4,
    WideString = 5,
    Color = 6,
    UInt64 = 7,
    End = 8,
}

impl TryFrom<u8> for KeyValueType {
    type Error = KeyValueError;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(KeyValueType::None),
            1 => Ok(KeyValueType::String),
            2 => Ok(KeyValueType::Int32),
            3 => Ok(KeyValueType::Float32),
            4 => Ok(KeyValueType::Pointer),
            5 => Ok(KeyValueType::WideString),
            6 => Ok(KeyValueType::Color),
            7 => Ok(KeyValueType::UInt64),
            8 => Ok(KeyValueType::End),
            _ => Err(KeyValueError::Format(format!(
                "Invalid KeyValueType: {}",
                value
            ))),
        }
    }
}

#[derive(Debug, Clone)]
pub enum KeyValueData {
    None,
    String(String),
    Int32(i32),
    Float32(f32),
    UInt64(u64),
    Color(u32),
}

#[derive(Debug, Clone)]
pub struct KeyValue {
    pub name: String,
    pub data: KeyValueData,
    pub children: HashMap<String, KeyValue>,
    pub valid: bool,
}

impl KeyValue {
    fn invalid() -> &'static Self {
        static INVALID: LazyLock<KeyValue> = LazyLock::new(|| KeyValue {
            name: "<invalid>".to_owned(),
            data: KeyValueData::None,
            children: HashMap::new(),
            valid: false,
        });

        &INVALID
    }

    pub fn root() -> Self {
        Self {
            name: "<root>".to_string(),
            data: KeyValueData::None,
            children: HashMap::new(),
            valid: true,
        }
    }

    pub fn get(&self, key: &str) -> &KeyValue {
        // self.children.get(key).cloned().unwrap_or_else(Self::invalid)
        self.children.get(key).unwrap_or(Self::invalid())
    }

    pub fn as_string(&self, default: &str) -> String {
        if !self.valid {
            return default.to_string();
        }

        match &self.data {
            KeyValueData::String(s) => s.clone(),
            KeyValueData::Int32(i) => i.to_string(),
            KeyValueData::Float32(f) => f.to_string(),
            KeyValueData::UInt64(u) => u.to_string(),
            KeyValueData::Color(c) => c.to_string(),
            KeyValueData::None => default.to_string(),
        }
    }

    pub fn as_i32(&self, default: i32) -> i32 {
        if !self.valid {
            return default;
        }

        match &self.data {
            KeyValueData::String(s) => s.parse().unwrap_or(default),
            KeyValueData::Int32(i) => *i,
            KeyValueData::Float32(f) => *f as i32,
            KeyValueData::UInt64(u) => (*u & 0xFFFFFFFF) as i32,
            _ => default,
        }
    }

    pub fn as_f32(&self, default: f32) -> f32 {
        if !self.valid {
            return default;
        }

        match &self.data {
            KeyValueData::String(s) => s.parse().unwrap_or(default),
            KeyValueData::Int32(i) => *i as f32,
            KeyValueData::Float32(f) => *f,
            KeyValueData::UInt64(u) => (*u & 0xFFFFFFFF) as f32,
            _ => default,
        }
    }

    pub fn as_bool(&self, default: bool) -> bool {
        if !self.valid {
            return default;
        }

        match &self.data {
            KeyValueData::String(s) => s.parse::<i32>().map(|v| v != 0).unwrap_or(default),
            KeyValueData::Int32(i) => *i != 0,
            KeyValueData::Float32(f) => *f != 0.0,
            KeyValueData::UInt64(u) => *u != 0,
            _ => default,
        }
    }

    pub fn load_as_binary<P: AsRef<Path>>(path: P) -> Result<Self, KeyValueError> {
        let mut file = File::open(path)?;
        let mut kv = Self::root();
        kv.read_as_binary(&mut file)?;
        Ok(kv)
    }

    pub fn read_as_binary<R: Read + Seek>(&mut self, input: &mut R) -> Result<(), KeyValueError> {
        loop {
            let mut type_byte = [0u8];
            input.read_exact(&mut type_byte)?;
            let kv_type = KeyValueType::try_from(type_byte[0])?;

            if kv_type == KeyValueType::End {
                break;
            }

            let name = Self::read_string_unicode(input)?;
            let mut current = KeyValue {
                name,
                data: KeyValueData::None,
                children: HashMap::new(),
                valid: true,
            };

            match kv_type {
                KeyValueType::None => {
                    current.read_as_binary(input)?;
                }
                KeyValueType::String => {
                    current.data = KeyValueData::String(Self::read_string_unicode(input)?);
                }
                KeyValueType::WideString => {
                    return Err(KeyValueError::UnsupportedType(KeyValueType::WideString));
                }
                KeyValueType::Int32 => {
                    let mut buf = [0u8; 4];
                    input.read_exact(&mut buf)?;
                    current.data = KeyValueData::Int32(i32::from_le_bytes(buf));
                }
                KeyValueType::UInt64 => {
                    let mut buf = [0u8; 8];
                    input.read_exact(&mut buf)?;
                    current.data = KeyValueData::UInt64(u64::from_le_bytes(buf));
                }
                KeyValueType::Float32 => {
                    let mut buf = [0u8; 4];
                    input.read_exact(&mut buf)?;
                    current.data = KeyValueData::Float32(f32::from_le_bytes(buf));
                }
                KeyValueType::Color | KeyValueType::Pointer => {
                    let mut buf = [0u8; 4];
                    input.read_exact(&mut buf)?;
                    current.data = KeyValueData::Color(u32::from_le_bytes(buf));
                }
                KeyValueType::End => unreachable!(),
            }

            self.children.insert(current.name.clone(), current);
        }

        // Fuck it
        // There is no equivalent to C++ istream::peek. No sanity check this time.
        // I could check the file size and compare cursor position, but not today

        Ok(())
    }

    fn read_string_internal_dynamic(
        input: &mut dyn Read,
        encoding: KeyValueEncoding,
        end: char,
    ) -> Result<String, KeyValueError> {
        let character_size = match encoding {
            KeyValueEncoding::Utf8 => 1,
        };

        let character_end = end.to_string();
        let mut i = 0;
        let mut data = vec![0u8; 128 * character_size];

        loop {
            if i + character_size > data.len() {
                data.resize(data.len() + (128 * character_size), 0);
            }

            let read = input.read(&mut data[i..i + character_size])?;
            if read != character_size {
                return Err(KeyValueError::Io(std::io::Error::new(
                    std::io::ErrorKind::UnexpectedEof,
                    "Failed to read expected number of bytes",
                )));
            }

            let slice = &data[i..i + character_size];
            let s = match encoding {
                KeyValueEncoding::Utf8 => std::str::from_utf8(slice).unwrap_or(""),
            };

            if s == character_end {
                break;
            }

            i += character_size;
        }

        if i == 0 {
            return Ok(String::new());
        }

        match encoding {
            KeyValueEncoding::Utf8 => Ok(String::from_utf8(data[..i].to_vec()).unwrap_or_default()),
        }
    }

    pub fn read_string_unicode(input: &mut dyn Read) -> Result<String, KeyValueError> {
        Self::read_string_internal_dynamic(input, KeyValueEncoding::Utf8, '\0')
    }
}

impl fmt::Display for KeyValue {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if !self.valid {
            return write!(f, "<invalid>");
        }

        if matches!(self.data, KeyValueData::None) && !self.children.is_empty() {
            return write!(f, "{}", self.name);
        }

        write!(f, "{} = {}", self.name, self.as_string(""))
    }
}
