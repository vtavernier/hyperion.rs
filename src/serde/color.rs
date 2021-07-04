use serde::ser::SerializeSeq;

use crate::models::Color;

pub fn serialize_color_as_array<S: serde::ser::Serializer>(
    color: &Color,
    s: S,
) -> Result<S::Ok, S::Error> {
    let mut seq = s.serialize_seq(Some(3))?;
    seq.serialize_element(&color.red)?;
    seq.serialize_element(&color.green)?;
    seq.serialize_element(&color.blue)?;
    seq.end()
}
