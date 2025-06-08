# Rust-md

![header](./vault/assets/brain_header.jpg)

The goal is to be able to server md file like a rest api

similar to <https://markdowndb.com/> but without the SQL part and written in rust

basically serve markdown files as json

- `GET /` return recents files and stats
- `GET /files?public=true&tags=[test]`
  - return associated assets (img, etc)
  - `{file: ..., assets: []}`
- `GET /files/:id`
- `GET /files/tags/:id` dunno man
- `GET /tags`

btw I dunno how to write RUST so this thing might be a mess

This is a repo that contains multiple projects related to my knowledge management

This is configured to either work with VSCode using some extensions or Obsidian

maybe plug into obsidian ???

- [vault](./vault/README.md) contains the obsidian vault

## start

install rust

```bash
cargo install --locked bacon
```

```bash
bacon run-long
```

## References

- <https://github.com/zoni/obsidian-export/>
- <https://github.com/trashhalo/obsidian-rust-plugin/>
- <https://markdowndb.com/>
- <https://github.com/wooorm/markdown-rs>
- <https://github.com/pulldown-cmark/pulldown-cmark/>
  - <https://docs.rs/pulldown-cmark/latest/pulldown_cmark/>
  - <https://pulldown-cmark.github.io/pulldown-cmark/>
- Zola
  - <https://github.com/getzola/zola/blob/master/components/libs/src/lib.rs>
  - <https://github.com/getzola/zola/blob/c3a3d78dfddd34debf5254d293558e8fa045fd51/src/cmd/serve.rs#L45>
