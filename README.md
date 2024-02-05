# 📫 Airmail 📫

Airmail is an extremely lightweight geocoder[^1] written in pure Rust. Built on top of [tantivy](https://github.com/quickwit-oss/tantivy), it offers an incredibly low memory footprint (substantially under 1GB) and lightning-quick indexing (10k+ POIs per second). Airmail currently supports English queries based on place names and addresses in North American address formats. Other languages and address formats may work, but have not been systematically tested. It is capable of parsing category queries, and work is ongoing to lookup categories in the index. Support for viewport biasing and restriction is planned.

[^1]: A geocoder is a search engine for places. When you type in "vegan donut shop" into your maps app of choice, a geocoder is what shows you nearby places that fit your query.

### Stay Tuned

I have a repetitive stress injury in my wrist right now so work may be slow, but with any luck there will be progress over the next few months. ✨✨

### License

Dual MIT/Apache 2 license, at your option.
