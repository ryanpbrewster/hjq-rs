extern crate serde;
extern crate serde_json;

use serde::de::{DeserializeSeed, Deserializer, MapAccess, SeqAccess, Visitor};
use serde_json::json;
use std::fmt;
use std::io;

pub fn for_each_primitive<F>(input: impl io::Read, mut f: F)
where
    F: FnMut(&str, &serde_json::Value),
{
    let mut prefix = String::new();
    let mut de = serde_json::Deserializer::from_reader(input);
    de.deserialize_any(SideEffectingVisitor {
        prefix: &mut prefix,
        consumer: &mut f,
    })
    .expect("deserialize input");
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn count() {
        let js = r#" {"a": 0, "b": 1, "c": "foo"} "#;

        let mut total = 0;
        for_each_primitive(js.as_bytes(), |_, _| {
            total += 1;
        });

        assert_eq!(total, 3);
    }

    #[test]
    fn flatten() {
        let js = r#" {"a": 3, "b": 1, "c": 4} "#;

        let mut buf = Vec::new();
        for_each_primitive(js.as_bytes(), |_, v| {
            buf.push(v.clone());
        });

        assert_eq!(buf, vec![3, 1, 4]);
    }
}

struct SideEffectingVisitor<'a, C> {
    prefix: &'a mut String,
    consumer: &'a mut C,
}
impl<'de, 'a, C> DeserializeSeed<'de> for SideEffectingVisitor<'a, C>
where
    C: KvConsumer,
{
    type Value = ();

    fn deserialize<D>(self, deserializer: D) -> Result<(), D::Error>
    where
        D: Deserializer<'de>,
    {
        deserializer.deserialize_any(SideEffectingVisitor {
            prefix: self.prefix,
            consumer: self.consumer,
        })
    }
}

impl<'de, 'a, C> Visitor<'de> for SideEffectingVisitor<'a, C>
where
    C: KvConsumer,
{
    type Value = ();

    fn expecting(&self, formatter: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        write!(formatter, "anything vaguely json-like")
    }
    fn visit_bool<E>(self, v: bool) -> Result<(), E> {
        self.consumer.accept(self.prefix, &json!(v));
        Ok(())
    }
    fn visit_i64<E>(self, v: i64) -> Result<(), E> {
        self.consumer.accept(self.prefix, &json!(v));
        Ok(())
    }
    fn visit_u64<E>(self, v: u64) -> Result<(), E> {
        self.consumer.accept(self.prefix, &json!(v));
        Ok(())
    }
    fn visit_f64<E>(self, v: f64) -> Result<(), E> {
        self.consumer.accept(self.prefix, &json!(v));
        Ok(())
    }
    fn visit_str<E>(self, v: &str) -> Result<(), E> {
        self.consumer.accept(self.prefix, &json!(v));
        Ok(())
    }
    fn visit_string<E>(self, v: String) -> Result<(), E> {
        self.consumer.accept(self.prefix, &json!(v));
        Ok(())
    }
    fn visit_unit<E>(self) -> Result<(), E> {
        self.consumer.accept(self.prefix, &serde_json::Value::Null);
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
                consumer: self.consumer,
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
        while let Some(k) = map.next_key::<String>()? {
            self.prefix.push_str(&k);
            self.prefix.push('/');
            map.next_value_seed(SideEffectingVisitor {
                prefix: self.prefix,
                consumer: self.consumer,
            })?;
            self.prefix.split_off(self.prefix.len() - k.len() - 1);
        }
        Ok(())
    }
}

trait KvConsumer {
    fn accept(&mut self, k: &str, v: &serde_json::Value);
}

impl<F> KvConsumer for F
where
    F: FnMut(&str, &serde_json::Value),
{
    fn accept(&mut self, k: &str, v: &serde_json::Value) {
        self(k, v)
    }
}
