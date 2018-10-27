extern crate rocksdb;
extern crate serde;
extern crate serde_json;
extern crate structopt;

use serde::de::{DeserializeSeed, Deserializer, MapAccess, SeqAccess, Visitor};
use serde_json::json;
use std::fmt;
use std::fs::File;
use std::io::{BufReader, StdoutLock, Write};
use std::path::PathBuf;
use structopt::StructOpt;

fn main() {
    let opts = Options::from_args();
    match opts.cmd {
        Command::Trace { input, silent } => {
            let mut prefix = String::new();
            let fin = BufReader::new(File::open(input).expect("open file"));
            let mut de = serde_json::Deserializer::from_reader(fin);
            if silent {
                de.deserialize_any(SideEffectingVisitor {
                    prefix: &mut prefix,
                    writer: &mut Noop,
                })
                .expect("deserialize input");
            } else {
                let stdout = std::io::stdout();
                de.deserialize_any(SideEffectingVisitor {
                    prefix: &mut prefix,
                    writer: &mut stdout.lock(),
                })
                .expect("deserialize input");
            }
        }

        Command::Index { input, data_dir } => {
            let mut prefix = String::new();
            let mut db = {
                let mut db_opts = rocksdb::Options::default();
                db_opts.set_use_fsync(false);
                db_opts.create_if_missing(true);
                db_opts.increase_parallelism(4);
                rocksdb::DB::open(&db_opts, data_dir).expect("open db")
            };
            let fin = BufReader::new(File::open(input).expect("open file"));
            let mut de = serde_json::Deserializer::from_reader(fin);
            de.deserialize_any(SideEffectingVisitor {
                prefix: &mut prefix,
                writer: &mut db,
            })
            .expect("deserialize input");
        }

        Command::View { data_dir, prefix } => {
            let db = {
                let mut db_opts = rocksdb::Options::default();
                db_opts.set_prefix_extractor(rocksdb::SliceTransform::create_fixed_prefix(
                    prefix.len(),
                ));
                rocksdb::DB::open(&db_opts, data_dir).expect("open db")
            };
            let mut json = serde_json::Value::Null;
            for (k, v) in db.prefix_iterator(prefix.as_bytes()) {
                let path: Vec<String> = std::str::from_utf8(&k)
                    .expect("parse utf8")
                    .split('/')
                    .filter(|s| !s.is_empty())
                    .map(String::from)
                    .collect();
                set_json(
                    &mut json,
                    &path,
                    serde_json::from_slice(&v).expect("parse json from db"),
                );
            }
            println!(
                "{}",
                serde_json::to_string_pretty(&json).expect("serialize json")
            );
        }

        Command::Keys { data_dir, prefix } => {
            let pre = prefix.into_bytes();
            let db = rocksdb::DB::open_default(data_dir).expect("open db");

            // Skip through the key-space. After finding a key, set the bookend at the
            // end of the range spanned by that key.
            let mut iter = db.raw_iterator();
            let mut bookend = pre.clone();
            loop {
                iter.seek(&bookend);
                if !iter.valid() {
                    break;
                }
                let k = match iter.key() {
                    None => break,
                    Some(k) => k,
                };
                if !k.starts_with(&pre) || k.len() <= pre.len() {
                    break;
                }
                let key: Vec<u8> = k
                    .into_iter()
                    .skip(pre.len())
                    .take_while(|&b| b != b'/')
                    .collect();
                assert!(!key.is_empty(), "make sure your prefix ends in /");
                println!("{}", std::str::from_utf8(&key).expect("parse utf8 from db"));
                bookend = pre.clone();
                bookend.extend(key);
                bookend.push(b'/' + 1);
            }
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
        #[structopt(short = "s", long = "silent")]
        silent: bool,
    },
    #[structopt(name = "index")]
    Index {
        #[structopt(short = "i", long = "input", parse(from_os_str))]
        input: PathBuf,
        #[structopt(short = "d", long = "data-dir", parse(from_os_str))]
        data_dir: PathBuf,
    },

    #[structopt(name = "view")]
    View {
        #[structopt(short = "d", long = "data-dir", parse(from_os_str))]
        data_dir: PathBuf,
        #[structopt(short = "p", long = "prefix", default_value = "")]
        prefix: String,
    },

    #[structopt(name = "keys")]
    Keys {
        #[structopt(short = "d", long = "data-dir", parse(from_os_str))]
        data_dir: PathBuf,
        #[structopt(short = "p", long = "prefix", default_value = "")]
        prefix: String,
    },
}

struct SideEffectingVisitor<'a, W> {
    prefix: &'a mut String,
    writer: &'a mut W,
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

trait KvConsumer {
    fn accept(&mut self, k: &str, v: &serde_json::Value);
}

impl<'a> KvConsumer for StdoutLock<'a> {
    fn accept(&mut self, k: &str, v: &serde_json::Value) {
        writeln!(self, "{} = {}", k, v).expect("write to stdout");
    }
}

struct Noop;
impl KvConsumer for Noop {
    fn accept(&mut self, _k: &str, _v: &serde_json::Value) {}
}

impl KvConsumer for rocksdb::DB {
    fn accept(&mut self, k: &str, v: &serde_json::Value) {
        self.put(
            k.as_bytes(),
            &serde_json::to_vec(v).expect("serialize json"),
        )
        .expect("write to rocksdb");
    }
}

fn set_json(json: &mut serde_json::Value, path: &[String], value: serde_json::Value) {
    if path.is_empty() {
        *json = value;
        return;
    }

    // TODO(rpb): there has got to be a better way of doing this than this
    // hacky two-phase unification+extraction process
    match json {
        serde_json::Value::Object(_) => {}
        _ => {
            *json = serde_json::Value::Object(serde_json::Map::new());
        }
    }

    let children = match json {
        serde_json::Value::Object(children) => children,
        _ => unreachable!(),
    };
    let child = children
        .entry(path[0].clone())
        .or_insert(serde_json::Value::Null);
    set_json(child, &path[1..], value);
}
