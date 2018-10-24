extern crate rocksdb;
extern crate serde;
extern crate serde_json;
extern crate structopt;

use serde::de::{DeserializeSeed, Deserializer, MapAccess, SeqAccess, Visitor};
use std::fmt;
use std::fs::File;
use std::io::{BufReader, StdoutLock, Write};
use std::path::PathBuf;
use structopt::StructOpt;

fn main() {
    let opts = Options::from_args();
    match opts.cmd {
        Command::Trace { input } => {
            let mut prefix = String::new();
            let stdout = std::io::stdout();
            let fin = BufReader::new(File::open(input).expect("open file"));
            let mut de = serde_json::Deserializer::from_reader(fin);
            de.deserialize_any(SideEffectingVisitor {
                prefix: &mut prefix,
                writer: &mut stdout.lock(),
            })
            .expect("deserialize input");
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
            for (k, v) in db.prefix_iterator(prefix.as_bytes()) {
                println!(
                    "{} = {}",
                    std::str::from_utf8(&k).expect("parse utf8"),
                    std::str::from_utf8(&v).expect("parse utf8")
                );
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
        #[structopt(short = "p", long = "prefix")]
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
        self.writer.accept(self.prefix, &v.to_string());
        Ok(())
    }
    fn visit_i64<E>(self, v: i64) -> Result<(), E> {
        self.writer.accept(self.prefix, &v.to_string());
        Ok(())
    }
    fn visit_u64<E>(self, v: u64) -> Result<(), E> {
        self.writer.accept(self.prefix, &v.to_string());
        Ok(())
    }
    fn visit_f64<E>(self, v: f64) -> Result<(), E> {
        self.writer.accept(self.prefix, &v.to_string());
        Ok(())
    }
    fn visit_str<E>(self, v: &str) -> Result<(), E> {
        self.writer.accept(self.prefix, v);
        Ok(())
    }
    fn visit_string<E>(self, v: String) -> Result<(), E> {
        self.writer.accept(self.prefix, &v);
        Ok(())
    }
    fn visit_unit<E>(self) -> Result<(), E> {
        self.writer.accept(self.prefix, "null");
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
    fn accept(&mut self, k: &str, v: &str);
}

impl<'a> KvConsumer for StdoutLock<'a> {
    fn accept(&mut self, k: &str, v: &str) {
        writeln!(self, "{} = {}", k, v).expect("write to stdout");
    }
}

impl KvConsumer for rocksdb::DB {
    fn accept(&mut self, k: &str, v: &str) {
        self.put(k.as_bytes(), v.as_bytes())
            .expect("write to rocksdb");
    }
}
