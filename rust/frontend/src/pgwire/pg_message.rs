use std::io;
use std::io::{IoSlice, Write};

use byteorder::{BigEndian, ByteOrder};
/// Part of code learned from https://github.com/zenithdb/zenith/blob/main/zenith_utils/src/pq_proto.rs.
use bytes::{BufMut, Bytes, BytesMut};
use risingwave_common::array::Row;
use risingwave_common::error::Result;
use tokio::io::AsyncReadExt;
use tokio::net::TcpStream;

use crate::pgwire::pg_field_descriptor::PgFieldDescriptor;
use crate::pgwire::pg_result::StatementType;

/// Messages that can be sent from pg client to server. Implement `read`.
pub enum FeMessage {
    Ssl,
    Startup(FeStartupMessage),
    Query(FeQueryMessage),
    Terminate,
}

pub struct FeStartupMessage {}

/// Query message contains the string sql.
pub struct FeQueryMessage {
    pub sql_bytes: Bytes,
}

impl FeQueryMessage {
    pub fn get_sql(&self) -> &str {
        // Why there is a \0..
        match std::str::from_utf8(&self.sql_bytes[..]) {
            Ok(v) => v.trim_end_matches('\0'),
            Err(e) => panic!("Invalid UTF-8 sequence: {}", e),
        }
    }
}

impl FeMessage {
    /// Read one message from the stream.
    pub async fn read(stream: &mut TcpStream) -> Result<FeMessage> {
        let val = &[stream.read_u8().await?];
        let tag = std::str::from_utf8(val).unwrap();
        let len = stream.read_i32().await?;

        let payload_len = len - 4;
        let mut payload: Vec<u8> = vec![0; payload_len as usize];
        if payload_len > 0 {
            stream.read_exact(&mut payload).await?;
        }
        let sql_bytes = Bytes::from(payload);

        match tag {
            "Q" => Ok(FeMessage::Query(FeQueryMessage { sql_bytes })),
            "X" => Ok(FeMessage::Terminate),
            _ => {
                unimplemented!("Do not support other tags regular message yet")
            }
        }
    }
}

impl FeStartupMessage {
    /// Read startup message from the stream.
    pub async fn read(stream: &mut TcpStream) -> Result<FeMessage> {
        let len = stream.read_i32().await?;
        let protocol_num = stream.read_i32().await?;
        let payload_len = len - 8;
        let mut payload = vec![0; payload_len as usize];
        if payload_len > 0 {
            stream.read_exact(&mut payload).await?;
        }
        match protocol_num {
            196608 => Ok(FeMessage::Startup(FeStartupMessage {})),
            80877103 => Ok(FeMessage::Ssl),
            _ => unimplemented!(
                "Unsupported protocol number in start up msg {:?}",
                protocol_num
            ),
        }
    }
}

/// Message sent from server to psql client. Implement `write`.
#[derive(Debug)]
pub enum BeMessage<'a> {
    AuthenticationOk,
    CommandComplete(BeCommandCompleteMessage),
    // Single byte - used in response to SSLRequest/GSSENCRequest.
    EncryptionResponse,
    DataRow(&'a Row),
    ParameterStatus(BeParameterStatusMessage<'a>),
    ReadyForQuery,
    RowDescription(&'a [PgFieldDescriptor]),
}

#[derive(Debug)]
pub enum BeParameterStatusMessage<'a> {
    Encoding(&'a str),
    StandardConformingString(&'a str),
}

#[derive(Debug)]
pub struct BeCommandCompleteMessage {
    pub stmt_type: StatementType,
    pub rows_cnt: i32,
}

impl<'a> BeMessage<'a> {
    /// Write message to the given buf.
    pub fn write(buf: &mut BytesMut, message: &BeMessage<'_>) -> Result<()> {
        match message {
            // AuthenticationOk
            // +-----+----------+-----------+
            // | 'R' | int32(8) | int32(0)  |
            // +-----+----------+-----------+
            BeMessage::AuthenticationOk => {
                buf.put_u8(b'R');
                buf.put_i32(8);
                buf.put_i32(0);
            }

            // ParameterStatus
            // +-----+-----------+----------+------+-----------+------+
            // | 'S' | int32 len | str name | '\0' | str value | '\0' |
            // +-----+-----------+----------+------+-----------+------+
            //
            // At present there is a hard-wired set of parameters for which
            // ParameterStatus will be generated: they are:
            //  server_version,
            //  server_encoding,
            //  client_encoding,
            //  application_name,
            //  is_superuser,
            //  session_authorization,
            //  DateStyle,
            //  IntervalStyle,
            //  TimeZone,
            //  integer_datetimes,
            //  standard_conforming_string
            //
            // See: https://www.postgresql.org/docs/9.2/static/protocol-flow.html#PROTOCOL-ASYNC.
            BeMessage::ParameterStatus(param) => {
                use BeParameterStatusMessage::*;
                let [name, value] = match param {
                    Encoding(val) => [b"client_encoding", val.as_bytes()],
                    StandardConformingString(val) => {
                        [b"standard_conforming_strings", val.as_bytes()]
                    }
                };

                // Parameter names and values are passed as null-terminated strings
                let iov = &mut [name, b"\0", value, b"\0"].map(IoSlice::new);
                let mut buffer = [0u8; 64]; // this should be enough
                let cnt = buffer.as_mut().write_vectored(iov).unwrap();

                buf.put_u8(b'S');
                write_body(buf, |stream| {
                    stream.put_slice(&buffer[..cnt]);
                    Ok(())
                })
                .unwrap();
            }

            // CommandComplete
            // +-----+-----------+-----------------+
            // | 'C' | int32 len | str commandTag  |
            // +-----+-----------+-----------------+
            BeMessage::CommandComplete(cmd) => {
                let rows_cnt = cmd.rows_cnt;
                let stmt_type = cmd.stmt_type;
                let mut tag = "".to_owned();
                tag.push_str(&stmt_type.to_string());
                if stmt_type == StatementType::INSERT {
                    tag.push_str(" 0");
                }
                if stmt_type.is_command() {
                    tag.push(' ');
                    tag.push_str(&rows_cnt.to_string());
                }
                buf.put_u8(b'C');
                write_body(buf, |buf| {
                    write_cstr(buf, tag.as_bytes())?;
                    Ok(())
                })?;
            }

            // DataRow
            // +-----+-----------+--------------+--------+-----+--------+
            // | 'D' | int32 len | int16 colNum | column | ... | column |
            // +-----+-----------+--------------+----+---+-----+--------+
            //                                       |
            //                          +-----------+v------+
            //                          | int32 len | bytes |
            //                          +-----------+-------+
            BeMessage::DataRow(vals) => {
                buf.put_u8(b'D');
                write_body(buf, |buf| {
                    buf.put_u16(vals.size() as u16); // num of cols
                    for val_opt in &vals.0 {
                        if let Some(val) = val_opt {
                            let val_data = val.to_string();
                            buf.put_u32(val_data.len() as u32);
                            buf.put_slice(val_data.as_bytes());
                        } else {
                            buf.put_i32(-1);
                        }
                    }
                    Ok(())
                })
                .unwrap();
            }
            // RowDescription
            // +-----+-----------+--------------+-------+-----+-------+
            // | 'T' | int32 len | int16 colNum | field | ... | field |
            // +-----+-----------+--------------+----+--+-----+-------+
            //                                       |
            // +---------------+-------+-------+-----v-+-------+-------+-------+
            // | str fieldName | int32 | int16 | int32 | int16 | int32 | int16 |
            // +---------------+---+---+---+---+---+---+----+--+---+---+---+---+
            //                     |       |       |        |      |       |
            //                     v       |       v        v      |       v
            //                tableOID     |    typeOID  typeLen   |   formatCode
            //                             v                       v
            //                        colAttrNum               typeModifier
            BeMessage::RowDescription(row_descs) => {
                buf.put_u8(b'T');
                write_body(buf, |buf| {
                    buf.put_i16(row_descs.len() as i16); // # of fields
                    for pg_field in row_descs.iter() {
                        write_cstr(buf, pg_field.get_name().as_bytes())?;
                        buf.put_i32(pg_field.get_table_oid()); // table oid
                        buf.put_i16(pg_field.get_col_attr_num()); // attnum
                        buf.put_i32(pg_field.get_type_oid().as_number());
                        buf.put_i16(pg_field.get_type_len());
                        buf.put_i32(pg_field.get_type_modifier()); // typmod
                        buf.put_i16(pg_field.get_format_code()); // format code
                    }
                    Ok(())
                })?;
            }
            // ReadyForQuery
            // +-----+----------+---------------------------+
            // | 'Z' | int32(5) | byte1(transaction status) |
            // +-----+----------+---------------------------+
            BeMessage::ReadyForQuery => {
                buf.put_u8(b'Z');
                buf.put_i32(5);
                // TODO: add transaction status
                buf.put_u8(b'I');
            }

            BeMessage::EncryptionResponse => {
                buf.put_u8(b'N');
            }
        }

        Ok(())
    }
}

// Safe usize -> i32|i16 conversion, from rust-postgres
trait FromUsize: Sized {
    fn from_usize(x: usize) -> Result<Self>;
}

macro_rules! from_usize {
    ($t:ty) => {
        impl FromUsize for $t {
            #[inline]
            fn from_usize(x: usize) -> Result<$t> {
                if x > <$t>::max_value() as usize {
                    Err(
                        io::Error::new(io::ErrorKind::InvalidInput, "value too large to transmit")
                            .into(),
                    )
                } else {
                    Ok(x as $t)
                }
            }
        }
    };
}

from_usize!(i32);

/// Call f() to write body of the message and prepend it with 4-byte len as
/// prescribed by the protocol. First write out body value and fill length value as i32 in front of
/// it.
fn write_body<F>(buf: &mut BytesMut, f: F) -> Result<()>
where
    F: FnOnce(&mut BytesMut) -> Result<()>,
{
    let base = buf.len();
    buf.extend_from_slice(&[0; 4]);

    f(buf)?;

    let size = i32::from_usize(buf.len() - base)?;
    BigEndian::write_i32(&mut buf[base..], size);
    Ok(())
}

/// Safe write of s into buf as cstring (String in the protocol).
fn write_cstr(buf: &mut BytesMut, s: &[u8]) -> Result<()> {
    if s.contains(&0) {
        return Err(
            io::Error::new(io::ErrorKind::InvalidInput, "string contains embedded null").into(),
        );
    }
    buf.put_slice(s);
    buf.put_u8(0);
    Ok(())
}
