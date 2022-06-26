use std::{
    future::Future,
    pin::Pin,
    task::{Context, Poll},
};

use actix_web::{
    dev::Payload,
    error::PayloadError,
    http::{header::CONTENT_LENGTH, StatusCode},
    web::{Buf, BytesMut},
    FromRequest, HttpMessage, HttpRequest, ResponseError,
};
use futures_core::{ready, Stream as _};
use serde::de::DeserializeOwned;

const DEFAULT_LIMIT: usize = 2_097_152; // 2 MB

/// CSV extractor that extracts typed data from a request body.
#[derive(Debug)]
pub struct Csv<T>(Vec<T>);

impl<T> Csv<T> {
    pub fn into_inner(self) -> Vec<T> {
        self.0
    }
}

impl<T: DeserializeOwned> FromRequest for Csv<T> {
    type Error = CsvExtractError;
    type Future = CsvExtractFut<T>;

    #[inline]
    fn from_request(req: &HttpRequest, payload: &mut Payload) -> Self::Future {
        CsvExtractFut::new(req, payload)
    }
}

pub enum CsvExtractFut<T> {
    Err(CsvExtractError),
    Ok {
        limit: usize,
        length: Option<usize>,
        bytes_read: usize,
        payload: Payload,
        buf: BytesMut,
        records: Vec<T>,
    },
}

impl<T> Unpin for CsvExtractFut<T> {}

impl<T: DeserializeOwned> CsvExtractFut<T> {
    pub fn new(req: &HttpRequest, payload: &mut Payload) -> Self {
        match req.mime_type() {
            Err(_) | Ok(None) => return Self::Err(CsvExtractError::ContentType),
            Ok(Some(t)) => {
                if t != "text/csv; charset=utf-8" {
                    return Self::Err(CsvExtractError::ContentType);
                }
            }
        }

        let length = req
            .headers()
            .get(&CONTENT_LENGTH)
            .and_then(|x| x.to_str().ok())
            .and_then(|s| s.parse::<usize>().ok());
        let limit = DEFAULT_LIMIT;

        if let Some(length) = length {
            if length > limit {
                return Self::Err(CsvExtractError::TooLong { length, limit });
            }
        }

        let payload = payload.take();

        Self::Ok {
            limit,
            length,
            bytes_read: 0,
            payload,
            buf: BytesMut::with_capacity(8192),
            records: Vec::new(),
        }
    }
}

impl<T: DeserializeOwned> Future for CsvExtractFut<T> {
    type Output = Result<Csv<T>, CsvExtractError>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let this = self.get_mut();

        match this {
            Self::Ok {
                limit,
                length,
                bytes_read,
                payload,
                buf,
                records,
            } => loop {
                let res = ready!(Pin::new(&mut *payload).poll_next(cx));

                match res {
                    Some(chunk) => {
                        let mut chunk = chunk?;

                        if let Some(length) = *length {
                            if *bytes_read + chunk.len() > length {
                                return Poll::Ready(Err(CsvExtractError::OverflowKnownLength {
                                    length,
                                }));
                            }
                        }

                        if *bytes_read + chunk.len() > *limit {
                            return Poll::Ready(Err(CsvExtractError::Overflow { limit: *limit }));
                        }

                        let mut consumed_bytes = 0;
                        let chunk_len = chunk.len();

                        while consumed_bytes < chunk_len {
                            let num_bytes = chunk
                                .iter()
                                .take_while(|b| **b != b'\n' && **b != b'\r')
                                .count();

                            // Did not encounter any line terminator. Add the entire
                            // chunk to `buf` and wait for next chunk.
                            if num_bytes == chunk.len() {
                                buf.extend_from_slice(&chunk);
                                consumed_bytes += chunk.len();
                                break;
                            }

                            let num_bytes_terminator = if num_bytes + 2 <= chunk.len()
                                && chunk[num_bytes] == b'\r'
                                && chunk[num_bytes + 1] == b'\n'
                            {
                                // CRLF line terminator found.
                                2
                            } else {
                                1
                            };
                            let bytes_to_take = num_bytes + num_bytes_terminator;

                            buf.extend_from_slice(&chunk[..bytes_to_take]);

                            match csv::ReaderBuilder::new()
                                .has_headers(false)
                                .from_reader(buf.reader())
                                .deserialize()
                                .next()
                            {
                                None => {}
                                Some(Err(err)) => {
                                    return Poll::Ready(Err(CsvExtractError::Deserialize(err)))
                                }
                                Some(Ok(record)) => records.push(record),
                            }

                            consumed_bytes += bytes_to_take;
                            buf.clear();
                            chunk.advance(bytes_to_take);
                        }

                        *bytes_read += consumed_bytes;
                    }
                    None => {
                        if let Some(res) = csv::ReaderBuilder::new()
                            .has_headers(false)
                            .from_reader(buf.reader())
                            .deserialize()
                            .next()
                        {
                            records.push(res?);
                        };

                        return Poll::Ready(Ok(Csv(std::mem::take(records))));
                    }
                }
            },
            Self::Err(err) => {
                Poll::Ready(Err(std::mem::replace(err, CsvExtractError::ContentType)))
            }
        }
    }
}

#[derive(thiserror::Error, Debug)]
pub enum CsvExtractError {
    #[error(
        "Payload content length ({} bytes) exceeds limit ({} bytes).",
        length,
        limit
    )]
    TooLong { length: usize, limit: usize },

    #[error("Payload is larger than content length ({} bytes).", length)]
    OverflowKnownLength { length: usize },

    #[error("Payload size exceeded limit ({} bytes).", limit)]
    Overflow { limit: usize },

    #[error("Content type error.")]
    ContentType,

    #[error("CSV deserialize error: {}", 0)]
    Deserialize(#[from] csv::Error),

    #[error("Failed to read payload: {}", 0)]
    Payload(#[from] PayloadError),
}

impl ResponseError for CsvExtractError {
    fn status_code(&self) -> StatusCode {
        match self {
            Self::TooLong { .. } | Self::OverflowKnownLength { .. } | Self::Overflow { .. } => {
                StatusCode::PAYLOAD_TOO_LARGE
            }
            Self::Payload(err) => err.status_code(),
            _ => StatusCode::BAD_REQUEST,
        }
    }
}
