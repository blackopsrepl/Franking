/*! Decoded RFC 5322 header fields.
Headers are exposed two ways: as ordered, human-readable `HeaderField`s for
display, and as typed fields (addresses, date, message identity) for logic.
Both are fully decoded, so RFC 2047 encoded-words and RFC 2231 parameters never
reach the UI or the threading code in raw form. */

use chrono::{DateTime, FixedOffset, NaiveDate, TimeZone};
use mail_parser::{Address as ParsedAddress, HeaderName, HeaderValue, Message};

use super::address::Address;
use super::normalize_newlines;

/// One header rendered to a display string, preserving original order.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HeaderField {
    pub name: String,
    pub value: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct DecodedHeaders {
    /// Every header, decoded, in the order it appeared in the message.
    pub fields: Vec<HeaderField>,
    pub subject: Option<String>,
    pub from: Vec<Address>,
    pub to: Vec<Address>,
    pub cc: Vec<Address>,
    pub bcc: Vec<Address>,
    pub reply_to: Vec<Address>,
    pub date: Option<DateTime<FixedOffset>>,
    pub message_id: Option<String>,
    pub in_reply_to: Vec<String>,
    pub references: Vec<String>,
    pub list_id: Option<String>,
}

impl DecodedHeaders {
    /// First decoded value for a header name, case-insensitive.
    pub fn get(&self, name: &str) -> Option<&str> {
        self.fields
            .iter()
            .find(|field| field.name.eq_ignore_ascii_case(name))
            .map(|field| field.value.as_str())
    }

    /// All decoded values for a header name, case-insensitive.
    pub fn get_all<'a>(&'a self, name: &str) -> Vec<&'a str> {
        self.fields
            .iter()
            .filter(|field| field.name.eq_ignore_ascii_case(name))
            .map(|field| field.value.as_str())
            .collect()
    }

    pub fn from_message(message: &Message<'_>) -> Self {
        let fields = message
            .headers()
            .iter()
            .filter_map(|header| {
                render_header_value(&header.value).map(|value| HeaderField {
                    name: header.name.to_string(),
                    value,
                })
            })
            .collect();

        let subject = message.subject().map(ToOwned::to_owned);
        let from = convert_addresses(message.from());
        let to = convert_addresses(message.to());
        let cc = convert_addresses(message.cc());
        let bcc = convert_addresses(message.bcc());
        let reply_to = convert_addresses(message.reply_to());
        let date = message.date().and_then(convert_datetime);
        let message_id = message.message_id().map(normalize_message_id);
        let in_reply_to = message_id_list(message.in_reply_to());
        let references = message_id_list(message.references());
        let list_id = message
            .header(HeaderName::ListId)
            .and_then(HeaderValue::as_text)
            .map(|value| value.trim().to_string())
            .filter(|value| !value.is_empty());

        Self {
            fields,
            subject,
            from,
            to,
            cc,
            bcc,
            reply_to,
            date,
            message_id,
            in_reply_to,
            references,
            list_id,
        }
    }

    /// All recipients across To, Cc, and Bcc.
    pub fn recipients(&self) -> impl Iterator<Item = &Address> {
        self.to.iter().chain(self.cc.iter()).chain(self.bcc.iter())
    }
}

/// Render a parsed header value to a decoded, unfolded string.
pub(super) fn render_header_value(value: &HeaderValue<'_>) -> Option<String> {
    match value {
        HeaderValue::Text(text) => Some(collapse(&normalize_newlines(text))),
        HeaderValue::TextList(list) => {
            let joined = list
                .iter()
                .map(|item| item.as_ref())
                .collect::<Vec<_>>()
                .join(", ");
            Some(collapse(&normalize_newlines(&joined)))
        }
        HeaderValue::Address(address) => {
            let rendered = render_addresses(address);
            (!rendered.is_empty()).then_some(rendered)
        }
        HeaderValue::DateTime(datetime) => convert_datetime(datetime).map(|date| date.to_rfc2822()),
        HeaderValue::ContentType(content_type) => {
            let subtype = content_type.c_subtype.as_deref().unwrap_or_default();
            Some(format!("{}/{subtype}", content_type.c_type))
        }
        HeaderValue::Received(_) | HeaderValue::Empty => None,
    }
}

fn render_addresses(address: &ParsedAddress<'_>) -> String {
    address
        .iter()
        .map(|addr| {
            Address {
                name: addr.name.as_deref().map(str::to_string),
                email: addr.address.as_deref().map(str::to_string),
            }
            .display()
        })
        .filter(|display| !display.is_empty())
        .collect::<Vec<_>>()
        .join(", ")
}

fn convert_addresses(address: Option<&ParsedAddress<'_>>) -> Vec<Address> {
    address
        .map(|address| {
            address
                .iter()
                .map(|addr| Address {
                    name: addr
                        .name
                        .as_deref()
                        .map(|name| name.trim().to_string())
                        .filter(|name| !name.is_empty()),
                    email: addr
                        .address
                        .as_deref()
                        .map(|email| email.trim().to_string())
                        .filter(|email| !email.is_empty()),
                })
                .filter(|address| !address.is_empty())
                .collect()
        })
        .unwrap_or_default()
}

fn convert_datetime(value: &mail_parser::DateTime) -> Option<DateTime<FixedOffset>> {
    let offset_seconds = i32::from(value.tz_hour) * 3600 + i32::from(value.tz_minute) * 60;
    let offset_seconds = if value.tz_before_gmt {
        -offset_seconds
    } else {
        offset_seconds
    };
    let offset = FixedOffset::east_opt(offset_seconds)?;
    let naive = NaiveDate::from_ymd_opt(
        i32::from(value.year),
        u32::from(value.month),
        u32::from(value.day),
    )?
    .and_hms_opt(
        u32::from(value.hour),
        u32::from(value.minute),
        u32::from(value.second),
    )?;
    offset.from_local_datetime(&naive).single()
}

fn message_id_list(value: &HeaderValue<'_>) -> Vec<String> {
    if let Some(list) = value.as_text_list() {
        return list
            .iter()
            .flat_map(|item| split_message_ids(item.as_ref()))
            .collect();
    }
    value.as_text().map(split_message_ids).unwrap_or_default()
}

fn split_message_ids(value: &str) -> Vec<String> {
    value
        .split_whitespace()
        .map(normalize_message_id)
        .filter(|id| !id.is_empty())
        .collect()
}

fn normalize_message_id(value: &str) -> String {
    value
        .trim()
        .trim_start_matches('<')
        .trim_end_matches('>')
        .trim()
        .to_string()
}

fn collapse(value: &str) -> String {
    value.trim().to_string()
}
