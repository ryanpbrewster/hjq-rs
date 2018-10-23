#![feature(const_string_new)]
extern crate serde;
extern crate serde_json;

use serde::de::{Deserialize, DeserializeSeed, Deserializer, MapAccess, SeqAccess, Visitor};
use std::fmt;

const TINY: &str = r#"
{
  "a": 1,
  "b": 2,
  "c": true,
  "d": null,
  "e": {
    "f": "bar"
  },
  "g": {
    "h": {
        "i": -3.141592e7
    }
  },
  "x": {
    "y": ["asdf", 42, true, false, [1,2,3], { "foo": "bar" }]
  }
}
"#;

struct SideEffectingVisitor<'a> {
    prefix: &'a mut String,
}
impl<'de, 'a> DeserializeSeed<'de> for SideEffectingVisitor<'a> {
    type Value = ();

    fn deserialize<D>(self, deserializer: D) -> Result<(), D::Error>
    where
        D: Deserializer<'de>,
    {
        deserializer.deserialize_any(SideEffectingVisitor {
            prefix: self.prefix,
        })
    }
}

impl<'de, 'a> Visitor<'de> for SideEffectingVisitor<'a> {
    type Value = ();

    fn expecting(&self, formatter: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        write!(formatter, "some bespoke garbage")
    }
    fn visit_bool<E>(self, v: bool) -> Result<(), E> {
        println!("{} = {}", self.prefix, v);
        Ok(())
    }
    fn visit_i64<E>(self, v: i64) -> Result<(), E> {
        println!("{} = {}", self.prefix, v);
        Ok(())
    }
    fn visit_u64<E>(self, v: u64) -> Result<(), E> {
        println!("{} = {}", self.prefix, v);
        Ok(())
    }
    fn visit_f64<E>(self, v: f64) -> Result<(), E> {
        println!("{} = {}", self.prefix, v);
        Ok(())
    }
    fn visit_str<E>(self, v: &str) -> Result<(), E> {
        println!("{} = {}", self.prefix, v);
        Ok(())
    }
    fn visit_unit<E>(self) -> Result<(), E> {
        println!("{} = {}", self.prefix, "null");
        Ok(())
    }
    fn visit_seq<A>(self, mut seq: A) -> Result<(), A::Error>
    where
        A: SeqAccess<'de>,
    {
        for i in 0.. {
            let k = i.to_string();
            self.prefix.push_str(&k);
            self.prefix.push('/');
            let tmp = seq.next_element_seed(SideEffectingVisitor {
                prefix: self.prefix,
            })?;
            self.prefix.split_off(self.prefix.len() - k.len() - 1);
            if tmp.is_none() {
                break;
            }
        }
        Ok(())
    }

    fn visit_map<A>(self, mut map: A) -> Result<(), A::Error>
    where
        A: MapAccess<'de>,
    {
        while let Some(k) = map.next_key::<&str>()? {
            self.prefix.push_str(k);
            self.prefix.push('/');
            map.next_value_seed(SideEffectingVisitor {
                prefix: self.prefix,
            })?;
            self.prefix.split_off(self.prefix.len() - k.len() - 1);
        }
        Ok(())
    }
}

struct SideEffectingSentinel;
impl<'de> Deserialize<'de> for SideEffectingSentinel {
    fn deserialize<D>(deserializer: D) -> Result<Self, <D as Deserializer<'de>>::Error>
    where
        D: Deserializer<'de>,
    {
        let mut buf = String::new();
        deserializer.deserialize_any(SideEffectingVisitor { prefix: &mut buf })?;
        Ok(SideEffectingSentinel)
    }
}

fn main() {
    println!("{}", TINY);
    let _ = serde_json::from_str::<SideEffectingSentinel>(TINY).unwrap();
}
