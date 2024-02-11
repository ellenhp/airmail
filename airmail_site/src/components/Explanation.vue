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
                        index and run on a relatively affordable VPS. The demo server is running on a t3a.medium with 4GB of
                        RAM. The main search index lives on S3 and is queried with range requests. The total cost of the
                        planet demo server is about $35/month, which is fairly low for a self-hosted planet geocoder on AWS.
                        The smallest x86_64 EC2 instance you could reasonably run a global Nominatim instance on is a
                        r5a.2xlarge, which costs about $325/month, and you have to pay for around 500GB of storage attached
                        to each serving instance as well. Pelias is cheaper than Nominatim, but still costs about $100/month
                        to run on EC2 after storage costs, if you use the recommended 8GB of RAM for Elasticsearch. These
                        cost savings are not without downsides. Latency on an Airmail request is pretty high, anywhere from
                        100-1000ms, depending on how hot the cache is. There's some low hanging fruit for optimization, but
                        S3 range queries are always going to be slower than an SSD random read.
                    </p>
                </div>
            </div>
        </div>
    </div>
</template>

