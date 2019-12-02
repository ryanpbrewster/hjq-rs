# What do you do with a stupidly large JSON file?

Most easy-to-use JSON utilities assume your payload can fit into memory. If
it's too large, you're gonna have to fall back on some kind of partial,
streaming parser/tokenizer. This tool uses `serde_json` to stream the
JSON primitives into a flat key-value store. Once it has been indexed,
you can view parts of the data deep within the JSON tree.

## Compression

The JSON is "indexed" into a flat key-value store. I'm currently using
[RocksDB](https://github.com/facebook/rocksdb/), which has support for a
variety of compression types. I tested this on a medium-sized JSON file,
[citylots](https://github.com/zemirco/sf-city-lots-json), to see how those play
out. Seems like Zstd is pretty good.

```
192M	citylots.json
307M	citylots-none/
160M	citylots-bz2/
135M	citylots-lz4/
132M	citylots-snappy/
 94M	citylots-zlib/
 92M	citylots-zstd/
```

## Example usage

Index your giant file (this will take a while, you can monitor progress
via `du -s index-data/`):
```
hjq index --data=my-giant-file.json --data-dir=index-data/
```

Once it's indexed you can explore it.

### Keys
View all the top-level keys:
```
hjq keys --data-dir=index-data/
```

You can also print out keys deeper into the JSON:
```
hjq keys --data-dir=index-data/ --prefix=some/path/into/the/tree/
```

### Full data
View the full data at some location inside your JSON:
```
hjq view --data-dir=index-data/ --prefix=some/path/into/the/tree/
```

Note that this will scale with the size of the JSON being printed, so if you
try to print out the full data at the root of your giant JSON tree it will take
a long, long time.
