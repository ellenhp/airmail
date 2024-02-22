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
                        index and run on inexpensive hardware. The demo server is running on a VPS with 512MB of
                        RAM. The main search index lives on object storage and is lazy-loaded with range requests. The total
                        cost of the
                        planet demo server is about $5 USD/month, which is fairly low for a planet-scale geocoder.
                        With the index in object storage, latency on an Airmail request is pretty high, anywhere from
                        10ms-10s, depending on how hot the cache is. If you can afford ~60GB of
                        block storage for the planet, you can consistently get sub-50ms latency, and still only need about
                        512MB of RAM.
                    </p>
                </div>
            </div>
        </div>
    </div>
</template>

