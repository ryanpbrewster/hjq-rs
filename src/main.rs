#![feature(const_string_new)]
extern crate serde;
extern crate serde_json;

use serde::de::{Deserialize, Deserializer, MapAccess, Visitor};
use std::fmt;

const TINY: &str = r#"
{
  "a": 1,
  "b": 2,
  "c": true,
  "d": null,
  "e": {
    "f": "bar"
  }
}
"#;

static mut PREFIX: String = String::new();

struct Sentinel;
impl<'de> Visitor<'de> for Sentinel {
    type Value = Sentinel;

    fn expecting(&self, formatter: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        formatter.write_str("some bespoke garbage")
    }

    fn visit_bool<E>(self, v: bool) -> Result<Self::Value, E> {
        println!("{} = {}", unsafe { &PREFIX }, v);
        Ok(Sentinel)
    }
    fn visit_i64<E>(self, v: i64) -> Result<Self::Value, E> {
        println!("{} = {}", unsafe { &PREFIX }, v);
        Ok(Sentinel)
    }
    fn visit_u64<E>(self, v: u64) -> Result<Self::Value, E> {
        println!("{} = {}", unsafe { &PREFIX }, v);
        Ok(Sentinel)
    }
    fn visit_f64<E>(self, v: f64) -> Result<Self::Value, E> {
        println!("{} = {}", unsafe { &PREFIX }, v);
        Ok(Sentinel)
    }
    fn visit_str<E>(self, v: &str) -> Result<Self::Value, E> {
        println!("{} = {}", unsafe { &PREFIX }, v);
        Ok(Sentinel)
    }
    fn visit_unit<E>(self) -> Result<Self::Value, E> {
        println!("{} = {}", unsafe { &PREFIX }, "null");
        Ok(Sentinel)
    }
    fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error>
    where
        A: MapAccess<'de>,
    {
        while let Some(k) = map.next_key::<&str>()? {
            unsafe {
                PREFIX.push_str(k);
                PREFIX.push('/');
            }
            map.next_value::<Sentinel>()?;
            unsafe {
                PREFIX.split_off(PREFIX.len() - k.len() - 1);
            }
        }
        Ok(Sentinel)
    }
}

impl<'de> Deserialize<'de> for Sentinel {
    fn deserialize<D>(deserializer: D) -> Result<Sentinel, D::Error>
    where
        D: Deserializer<'de>,
    {
        deserializer.deserialize_any(Sentinel)
    }
}

fn main() {
    let _: Sentinel = serde_json::from_str(TINY).unwrap();
}
