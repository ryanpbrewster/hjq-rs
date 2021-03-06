extern crate rocksdb;
extern crate serde;
extern crate serde_json;
extern crate structopt;

extern crate hjq;

use std::fs::File;
use std::io::{BufReader, Write};
use std::path::PathBuf;
use structopt::StructOpt;

fn main() {
    let opts = Options::from_args();
    match opts.cmd {
        Command::Trace { input, silent } => {
            let fin = BufReader::new(File::open(input).expect("open file"));
            if silent {
                hjq::for_each_primitive(fin, |_, _| ());
            } else {
                let stdout = std::io::stdout();
                let mut lock = stdout.lock();
                hjq::for_each_primitive(fin, |k, v| {
                    writeln!(lock, "{} = {}", k, v).expect("write to stdout");
                });
            }
        }

        Command::Index { input, data_dir } => {
            let db = {
                let mut db_opts = rocksdb::Options::default();
                db_opts.set_use_fsync(false);
                db_opts.create_if_missing(true);
                db_opts.set_compression_type(rocksdb::DBCompressionType::Zstd);
                db_opts.increase_parallelism(4);
                rocksdb::DB::open(&db_opts, data_dir).expect("open db")
            };
            let fin = BufReader::new(File::open(input).expect("open file"));
            hjq::for_each_primitive(fin, |k, v| {
                // Disable the write-ahead log here. We don't care about disaster recovery, if there's a
                // failure we'll just re-run the operation from scratch. This increases speed by ~6x.
                let write_opts = {
                    let mut opts = rocksdb::WriteOptions::default();
                    opts.disable_wal(true);
                    opts
                };
                db.put_opt(
                    k.as_bytes(),
                    &serde_json::to_vec(v).expect("serialize json"),
                    &write_opts,
                )
                .expect("write to rocksdb");
            });
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
                    .cloned()
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
