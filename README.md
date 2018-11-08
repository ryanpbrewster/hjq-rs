# What do you do with a stupidly large JSON file?

Most easy-to-use JSON utilities assume your payload can fit into memory. If
it's too large, you're gonna have to fall back on some kind of partial,
streaming parser/tokenizer. I think this should be possible with `serde_json`,
but I...am struggling a bit.

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
