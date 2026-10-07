//! DXF (Drawing Exchange Format) reading and writing.
//!
//! Supports both **ASCII** and **Binary** DXF for versions R12 (AC1009)
//! through R2018+ (AC1032).
//!
//! # Reading
//!
//! ```rust,ignore
//! use opencadcodec::DxfReader;
//!
//! let doc = DxfReader::from_file("drawing.dxf")?.read()?;
//! ```
//!
//! # Writing
//!
//! ```rust,ignore
//! use opencadcodec::DxfWriter;
//!
//! DxfWriter::new(&doc).write_to_file("output.dxf")?;
//! ```

pub mod code_page;
mod dxf_code;
mod group_code_value;
mod reader;
mod spline_flags;
mod writer;

pub use dxf_code::DxfCode;
pub use group_code_value::GroupCodeValueType;
pub use reader::{DxfReader, DxfReaderConfiguration};
pub use writer::{value_type_for_code, write_binary_dxf, write_dxf};
pub use writer::{
    DxfBinaryWriter, DxfStreamWriter, DxfStreamWriterExt, DxfTextWriter, DxfWriter, SectionWriter,
};

pub(crate) fn split_color_book_name(value: &str) -> (Option<String>, Option<String>) {
    let optional = |value: &str| (!value.is_empty()).then(|| value.to_string());
    match value.split_once('$') {
        Some((book_name, color_name)) => (optional(book_name), optional(color_name)),
        None => (None, optional(value)),
    }
}

pub(crate) fn join_color_book_name(
    book_name: Option<&str>,
    color_name: Option<&str>,
) -> Option<String> {
    let book_name = book_name.filter(|value| !value.is_empty());
    let color_name = color_name.filter(|value| !value.is_empty());
    match (book_name, color_name) {
        (Some(book_name), Some(color_name)) => Some(format!("{book_name}${color_name}")),
        (Some(book_name), None) => Some(format!("{book_name}$")),
        (None, Some(color_name)) => Some(color_name.to_string()),
        (None, None) => None,
    }
}

/// Objects whose DXF form carries their extended data itself: XRECORD data
/// may use the 1000-range groups, TABLESTYLE and underlay definitions map
/// their XDATA to fields, FIELD keeps it on `Field::xdata`, and objects replayed from raw DXF groups keep them
/// in those groups. All other objects get their XDATA through
/// `CadDocument::object_xdata`.
pub(crate) fn object_has_own_dxf_xdata(object: &crate::objects::ObjectType) -> bool {
    use crate::objects::ObjectType;
    match object {
        ObjectType::XRecord(_)
        | ObjectType::TableStyle(_)
        | ObjectType::UnderlayDefinition(_)
        | ObjectType::Field(_) => true,
        ObjectType::SortEntitiesTable(table) => table.raw_dxf_codes.is_some(),
        // Unknown objects are written only from their raw groups.
        ObjectType::Unknown { .. } => true,
        _ => false,
    }
}
