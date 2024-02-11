# 📫 Airmail 📫

Airmail is an extremely lightweight geocoder[^1] written in pure Rust. Built on top of [tantivy](https://github.com/quickwit-oss/tantivy), it offers a low memory footprint and fast indexing (~8k POIs per second on my machine). Airmail currently supports English queries based on place names and addresses in North American address formats. Other languages and address formats work, but have not been systematically tested.

[^1]: A geocoder is a search engine for places. When you type in "vegan donut shop" into your maps app of choice, a geocoder is what shows you nearby places that fit your query.

### Features

Airmail's killer feature is the ability to query remote indices, e.g. on S3. This lets you keep your index hosting costs fixed while you scale horizontally, and lowers the baseline costs associated with hosting a planet instance by around 2x-10x compared to other geocoders.

### Roadmap

- [x] English/North American query parser for addresses, place names, and place name queries with locality or neighborhood.
- [x] Index OpenStreetMap data.
- [x] Index OpenAddresses data (not currently used in demo).
- [ ] Index WhosOnFirst data.
- [x] API server.
- [x] Address queries.
- [x] Named POI queries.
- [x] Prefix queries.
- [x] Query remote indices.
- [x] Support and test planet-scale indices.
- [ ] Extend query parser for other locales.
- [ ] Categorical search, e.g. "coffee shops near me".
- [ ] Bounding box biasing and restriction.
- [ ] Minutely updates?
- [ ] Systematic/automatic quality testing in CI.
- [ ] Alternate results, e.g. returning Starbucks locations for "Dunkin Donuts" queries on the US west coast.[^2]

[^2]: This will likely need to be done with a vector database and some machine learning, and may have major hosting cost implications. TBD.

### License

Dual MIT/Apache 2 license, at your option.
