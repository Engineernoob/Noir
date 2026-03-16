use std::{
    io::{self, BufRead, BufReader, Read, Write},
    process::ChildStdout,
};

use serde_json::Value;

pub fn read_message(reader: &mut BufReader<ChildStdout>) -> io::Result<Option<Value>> {
    let mut content_length = None;

    loop {
        let mut header = String::new();
        let bytes_read = reader.read_line(&mut header)?;

        if bytes_read == 0 {
            return Ok(None);
        }

        if header == "\r\n" {
            break;
        }

        if let Some((name, value)) = header.split_once(':') {
            if name.eq_ignore_ascii_case("Content-Length") {
                let parsed_length = value.trim().parse::<usize>().map_err(|err| {
                    io::Error::new(io::ErrorKind::InvalidData, format!("invalid length: {err}"))
                })?;

                content_length = Some(parsed_length);
            }
        }
    }

    let Some(content_length) = content_length else {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "missing Content-Length header",
        ));
    };

    let mut body = vec![0; content_length];
    reader.read_exact(&mut body)?;

    let message = serde_json::from_slice(&body).map_err(|err| {
        io::Error::new(
            io::ErrorKind::InvalidData,
            format!("invalid JSON-RPC body: {err}"),
        )
    })?;

    Ok(Some(message))
}

pub fn write_message(writer: &mut impl Write, value: &Value) -> io::Result<()> {
    let body = serde_json::to_vec(value).map_err(|err| {
        io::Error::new(
            io::ErrorKind::InvalidData,
            format!("failed to serialize JSON-RPC message: {err}"),
        )
    })?;

    let header = format!("Content-Length: {}\r\n\r\n", body.len());
    writer.write_all(header.as_bytes())?;
    writer.write_all(&body)?;
    writer.flush()
}
