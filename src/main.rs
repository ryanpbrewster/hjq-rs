#![feature(const_string_new)]
extern crate serde;
extern crate serde_json;
extern crate structopt;

use serde::de::{Deserialize, DeserializeSeed, Deserializer, MapAccess, SeqAccess, Visitor};
use std::fmt;
use std::fs::File;
use std::io::BufReader;
use std::path::PathBuf;
use structopt::StructOpt;

fn main() {
    let opts = Options::from_args();
    match opts.cmd {
        Command::Trace { input } => {
            let fin = BufReader::new(File::open(input).expect("open file"));
            serde_json::from_reader::<BufReader<File>, SideEffectingSentinel>(fin).unwrap();
        }
    }
}

#[derive(StructOpt)]
struct Options {
    #[structopt(subcommand)]
    cmd: Command,
}

#[derive(StructOpt)]
enum Command {
    #[structopt(name = "trace")]
    Trace {
        #[structopt(short = "i", long = "input", parse(from_os_str))]
        input: PathBuf,
    },
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
        write!(formatter, "anything vaguely json-like")
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
    fn visit_string<E>(self, v: String) -> Result<(), E> {
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
        while let Some(k) = map.next_key::<String>()? {
            self.prefix.push_str(&k);
            self.prefix.push('/');
            map.next_value_seed(SideEffectingVisitor {
                prefix: self.prefix,
            })?;
            self.prefix.split_off(self.prefix.len() - k.len() - 1);
        }
        Ok(())
    }
}
