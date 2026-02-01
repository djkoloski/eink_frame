use core::fmt;

use jiff::Zoned;
use serde::{Deserializer, de::Visitor};

pub fn rfc_9557<'de, D>(deserializer: D) -> Result<Zoned, D::Error>
where
    D: Deserializer<'de>,
{
    struct ZonedVisitor;

    impl<'de> Visitor<'de> for ZonedVisitor {
        type Value = Zoned;

        fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
            formatter.write_str("an RFC 9557 formatted date and time")
        }

        fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
        where
            E: serde::de::Error,
        {
            Zoned::strptime("%Y-%m-%dT%H:%M:%S%:Q", v).map_err(E::custom)
        }
    }

    deserializer.deserialize_str(ZonedVisitor)
}
