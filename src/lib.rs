extern crate rocksdb;
extern crate serde;
extern crate serde_json;

use serde::de::{DeserializeSeed, Deserializer, MapAccess, SeqAccess, Visitor};
use serde_json::json;
use std::fmt;
use std::io::{StdoutLock, Write};

pub struct SideEffectingVisitor<'a, W> {
    pub prefix: &'a mut String,
    pub writer: &'a mut W,
}
impl<'de, 'a, W> DeserializeSeed<'de> for SideEffectingVisitor<'a, W>
where
    W: KvConsumer,
{
    type Value = ();

    fn deserialize<D>(self, deserializer: D) -> Result<(), D::Error>
    where
        D: Deserializer<'de>,
    {
        deserializer.deserialize_any(SideEffectingVisitor {
            prefix: self.prefix,
            writer: self.writer,
        })
    }
}

impl<'de, 'a, W> Visitor<'de> for SideEffectingVisitor<'a, W>
where
    W: KvConsumer,
{
    type Value = ();

    fn expecting(&self, formatter: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        write!(formatter, "anything vaguely json-like")
    }
    fn visit_bool<E>(self, v: bool) -> Result<(), E> {
        self.writer.accept(self.prefix, &json!(v));
        Ok(())
    }
    fn visit_i64<E>(self, v: i64) -> Result<(), E> {
        self.writer.accept(self.prefix, &json!(v));
        Ok(())
    }
    fn visit_u64<E>(self, v: u64) -> Result<(), E> {
        self.writer.accept(self.prefix, &json!(v));
        Ok(())
    }
    fn visit_f64<E>(self, v: f64) -> Result<(), E> {
        self.writer.accept(self.prefix, &json!(v));
        Ok(())
    }
    fn visit_str<E>(self, v: &str) -> Result<(), E> {
        self.writer.accept(self.prefix, &json!(v));
        Ok(())
    }
    fn visit_string<E>(self, v: String) -> Result<(), E> {
        self.writer.accept(self.prefix, &json!(v));
        Ok(())
    }
    fn visit_unit<E>(self) -> Result<(), E> {
        self.writer.accept(self.prefix, &serde_json::Value::Null);
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
                writer: self.writer,
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
                writer: self.writer,
            })?;
            self.prefix.split_off(self.prefix.len() - k.len() - 1);
        }
        Ok(())
    }
}

pub trait KvConsumer {
    fn accept(&mut self, k: &str, v: &serde_json::Value);
}

impl<'a> KvConsumer for StdoutLock<'a> {
    fn accept(&mut self, k: &str, v: &serde_json::Value) {
        writeln!(self, "{} = {}", k, v).expect("write to stdout");
    }
}

pub struct Noop;
impl KvConsumer for Noop {
    fn accept(&mut self, _k: &str, _v: &serde_json::Value) {}
}

impl KvConsumer for rocksdb::DB {
    fn accept(&mut self, k: &str, v: &serde_json::Value) {
        // Disable the write-ahead log here. We don't care about disaster recovery, if there's a
        // failure we'll just re-run the operation from scratch. This increases speed by ~6x.
        let write_opts = {
            let mut opts = rocksdb::WriteOptions::default();
            opts.disable_wal(true);
            opts
        };
        self.put_opt(
            k.as_bytes(),
            &serde_json::to_vec(v).expect("serialize json"),
            &write_opts,
        )
        .expect("write to rocksdb");
    }
}
