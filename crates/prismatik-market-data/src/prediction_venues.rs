//! Clean-room response normalization for public prediction-market order books.
//!
//! This module performs no network I/O and accepts no credentials. Provider
//! activation remains gated by an approved source policy.

use crate::{BookAnomaly, BookLevel, BookSnapshot};
use prismatik_domain::ProviderId;
use serde::Deserialize;
use thiserror::Error;

/// Prediction venue response error.
#[derive(Clone, Debug, Error, PartialEq, Eq)]
pub enum VenueBookError {
    /// JSON did not match the documented response.
    #[error("invalid venue response: {0}")]
    InvalidJson(String),
    /// Fixed-point decimal was malformed or out of range.
    #[error("invalid fixed-point decimal")]
    InvalidDecimal,
    /// Required instrument identifier was empty.
    #[error("instrument identifier is empty")]
    EmptyInstrument,
}

#[derive(Deserialize)]
struct PolyBook {
    asset_id: String,
    bids: Vec<PolyLevel>,
    asks: Vec<PolyLevel>,
}
#[derive(Deserialize)]
struct PolyLevel {
    price: String,
    size: String,
}

/// Normalize one documented Polymarket CLOB book response.
pub fn parse_polymarket_orderbook(raw: &str) -> Result<BookSnapshot, VenueBookError> {
    let book: PolyBook = serde_json::from_str(raw)
        .map_err(|error| VenueBookError::InvalidJson(error.to_string()))?;
    if book.asset_id.trim().is_empty() {
        return Err(VenueBookError::EmptyInstrument);
    }
    let mut bids = levels(book.bids)?;
    bids.sort_by_key(|level| std::cmp::Reverse(level.price_micros));
    let mut asks = levels(book.asks)?;
    asks.sort_by_key(|level| level.price_micros);
    Ok(snapshot(ProviderId::POLYMARKET, book.asset_id, bids, asks))
}

fn levels(rows: Vec<PolyLevel>) -> Result<Vec<BookLevel>, VenueBookError> {
    rows.into_iter()
        .map(|row| {
            Ok(BookLevel {
                price_micros: decimal_micros(&row.price)?,
                size_micros: decimal_micros(&row.size)?,
            })
        })
        .collect()
}

#[derive(Deserialize)]
struct KalshiEnvelope {
    orderbook_fp: KalshiBook,
}
#[derive(Deserialize)]
struct KalshiBook {
    yes_dollars: Vec<[String; 2]>,
    no_dollars: Vec<[String; 2]>,
}

/// Normalize a documented Kalshi fixed-point book. Kalshi returns YES and NO
/// bids; NO bids are converted to equivalent YES asks at `1 - no_bid`.
pub fn parse_kalshi_orderbook(ticker: &str, raw: &str) -> Result<BookSnapshot, VenueBookError> {
    if ticker.trim().is_empty() {
        return Err(VenueBookError::EmptyInstrument);
    }
    let envelope: KalshiEnvelope = serde_json::from_str(raw)
        .map_err(|error| VenueBookError::InvalidJson(error.to_string()))?;
    let mut bids = envelope
        .orderbook_fp
        .yes_dollars
        .into_iter()
        .map(|row| {
            Ok(BookLevel {
                price_micros: decimal_micros(&row[0])?,
                size_micros: decimal_micros(&row[1])?,
            })
        })
        .collect::<Result<Vec<_>, VenueBookError>>()?;
    let mut asks = envelope
        .orderbook_fp
        .no_dollars
        .into_iter()
        .map(|row| {
            let no = decimal_micros(&row[0])?;
            if no > 1_000_000 {
                return Err(VenueBookError::InvalidDecimal);
            }
            Ok(BookLevel {
                price_micros: 1_000_000 - no,
                size_micros: decimal_micros(&row[1])?,
            })
        })
        .collect::<Result<Vec<_>, VenueBookError>>()?;
    bids.sort_by_key(|level| std::cmp::Reverse(level.price_micros));
    asks.sort_by_key(|level| level.price_micros);
    Ok(snapshot(ProviderId::KALSHI, ticker.into(), bids, asks))
}

fn snapshot(
    provider: ProviderId,
    instrument_id: String,
    bids: Vec<BookLevel>,
    asks: Vec<BookLevel>,
) -> BookSnapshot {
    let anomalies = if matches!((bids.first(),asks.first()),(Some(b),Some(a)) if b.price_micros>a.price_micros)
    {
        vec![BookAnomaly::CrossedBook]
    } else {
        vec![]
    };
    BookSnapshot {
        provider,
        instrument_id,
        sequence: 0,
        bids,
        asks,
        anomalies,
    }
}

fn decimal_micros(raw: &str) -> Result<u64, VenueBookError> {
    let raw = raw.trim();
    if raw.starts_with('-') || raw.is_empty() {
        return Err(VenueBookError::InvalidDecimal);
    }
    let mut parts = raw.split('.');
    let whole = parts.next().ok_or(VenueBookError::InvalidDecimal)?;
    let fraction = parts.next().unwrap_or("");
    if parts.next().is_some()
        || fraction.len() > 6
        || !whole.bytes().all(|b| b.is_ascii_digit())
        || !fraction.bytes().all(|b| b.is_ascii_digit())
    {
        return Err(VenueBookError::InvalidDecimal);
    }
    let whole = whole
        .parse::<u64>()
        .map_err(|_| VenueBookError::InvalidDecimal)?;
    let fraction = if fraction.is_empty() {
        0
    } else {
        fraction
            .parse::<u64>()
            .map_err(|_| VenueBookError::InvalidDecimal)?
            * 10_u64.pow(
                u32::try_from(6 - fraction.len()).map_err(|_| VenueBookError::InvalidDecimal)?,
            )
    };
    whole
        .checked_mul(1_000_000)
        .and_then(|value| value.checked_add(fraction))
        .ok_or(VenueBookError::InvalidDecimal)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn polymarket_book_normalizes() {
        let book=parse_polymarket_orderbook(r#"{"asset_id":"token","bids":[{"price":"0.45","size":"100"}],"asks":[{"price":"0.46","size":"150"}]}"#).unwrap();
        assert_eq!(book.bids[0].price_micros, 450_000);
    }
    #[test]
    fn kalshi_no_bid_becomes_yes_ask() {
        let book=parse_kalshi_orderbook("KX",r#"{"orderbook_fp":{"yes_dollars":[["0.1500","100.00"]],"no_dollars":[["0.8000","50.00"]]}}"#).unwrap();
        assert_eq!(book.asks[0].price_micros, 200_000);
    }
}
