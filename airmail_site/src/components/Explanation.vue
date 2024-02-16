<template>
    <div class="untree_co-hero" id="home-section">
        <div class="untree_co-section bg-light" id="features-section">
            <div class="container">
                <div class="feature-section-header">
                    <h1>
                        Airmail is different.
                    </h1>
                </div>
                <div class="col-lg-9">
                    <p>
                        Airmail has a couple really unique features. It's designed to query against a remote
                        index and run on inexpensive hardware. The demo server is running on a VPS with 1GB of
                        RAM. The main search index lives on object storage and is lazy-loaded with range requests. The total
                        cost of the
                        planet demo server is about $10 USD/month, which is fairly low for a planet-scale geocoder.
                        Nominatim, for example, requires at least 64GB of RAM for the planet, and you have to pay for around
                        500GB of storage attached
                        to each serving instance as well. Pelias is cheaper than Nominatim, but still recommends 8GB of RAM
                        for just for Elasticsearch, in addition to block storage for the index attached to each serving
                        instance. Airmail is around 10-30x cheaper, but you don't achieve savings like that without making
                        some compromises. Latency on an Airmail request is pretty high, anywhere from
                        10ms-10s, depending on how hot the cache is. There's some low hanging fruit for optimization, but
                        S3 range queries are always going to be slower than an SSD random read. If you can afford 350GB of
                        block storage, you can consistently get sub-50ms latency, and still only need about a gigabyte of
                        RAM.
                    </p>
                </div>
            </div>
        </div>
    </div>
</template>

