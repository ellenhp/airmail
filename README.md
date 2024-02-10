# 📫 Airmail 📫

Airmail is an extremely lightweight geocoder[^1] written in pure Rust. Built on top of [tantivy](https://github.com/quickwit-oss/tantivy), it offers a low memory footprint (<100MB) and lightning-quick indexing (~20k POIs per second on my machine). Airmail currently supports English queries based on place names and addresses in North American address formats. Other languages and address formats may work, but have not been systematically tested.

[^1]: A geocoder is a search engine for places. When you type in "vegan donut shop" into your maps app of choice, a geocoder is what shows you nearby places that fit your query.

### Features

Airmail's killer feature is sub-100MB memory consumption for a serving instance. The search index is memory mapped, so while performance suffers on resource-constrained systems, Airmail is able to serve requests under extraordinary memory pressure. Once Airmail is ready for production use, hosting a planet-scale web maps service for a small userbase on a low-end VPS will finally be possible.

### Roadmap

- [x] English/North American query parser for addresses, place names, and place name queries with locality or neighborhood.
- [x] Index OpenStreetMap data.
- [x] Index OpenAddresses data.
- [ ] Index WhosOnFirst data.
- [x] API server.
- [ ] Support and test planet-scale indices.
- [ ] Extend query parser for other locales.
- [ ] Categorical search, e.g. "coffee shops near me".
- [ ] Bounding box biasing and restriction.
- [ ] Minutely updates.
- [ ] Systematic/automatic quality testing in CI.
- [ ] Alternate results, e.g. returning Starbucks locations for "Dunkin Donuts" queries on the US west coast.[^2]

[^2]: This will likely need to be done with a vector database and some machine learning.

### License

Dual MIT/Apache 2 license, at your option.
