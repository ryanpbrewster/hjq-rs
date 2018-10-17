# What do you do with a stupidly large JSON file?

Most easy-to-use JSON utilities assume your payload can fit into memory. If
it's too large, you're gonna have to fall back on some kind of partial,
streaming parser/tokenizer. I think this should be possible with `serde_json`,
but I...am struggling a bit.
