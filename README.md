# 📫 Airmail 📫

Airmail is an extremely lightweight geocoder[^1] written in pure Rust. Built on top of [tantivy](https://github.com/quickwit-oss/tantivy), it offers a low memory footprint and fast indexing (index the planet in under 3 hours!). Airmail aims to support international queries in several languages, but in practice it's still very early days and there are definitely bugs preventing correct behavior.

[^1]: A geocoder is a search engine for places. When you type in "vegan donut shop" into your maps app of choice, a geocoder is what shows you nearby places that fit your query.

### Features

Airmail's killer feature is the ability to query remote indices, e.g. on S3. This lets you keep your index hosting costs fixed while you scale horizontally. The baseline cost of a global Airmail deployment is about $5 per month.

### Roadmap

- [x] English/North American query parser for addresses, place names, and place name queries with locality or neighborhood.
- [x] Index OpenStreetMap data.
- [x] Index OpenAddresses data (not currently used in demo).
- [ ] Index WhosOnFirst data.
- [x] API server.
- [x] Address queries.
- [x] Named POI queries.
- [ ] Prefix queries.
- [x] Query remote indices.
- [x] Support and test planet-scale indices.
- [x] International address queries.
- [x] Categorical search, e.g. "coffee shop seattle".
- [x] Typo tolerance (limited to >=8 character input tokens)
- [x] Bounding box biasing and restriction.
- [ ] Systematic/automatic quality testing in CI.

### License

Dual MIT/Apache 2 license, at your option.
