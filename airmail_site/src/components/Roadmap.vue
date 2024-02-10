<script setup>

const items = [
  {
    id: 1,
    name: "Localization",
    des: "Extend parser to work with more locales.",
    icon: "mdi-translate",
    color: "background-color: #6fe7dd",
  },
  {
    id: 2,
    name: "Categorical search",
    des: "Search for \"Food near me\".",
    icon: "mdi-food",
    color: "background-color: #ffc93c",
  },
  {
    id: 3,
    name: "Viewport biasing",
    des: "Restrict returned POIs with a custom bounding box.",
    icon: "mdi-crop",
    color: "background-color: #f73859",
  },
  {
    id: 4,
    name: "Return alternates",
    des: "Return similar POIs with machine learning if the query is ambiguous or local results are sparse.",
    icon: "mdi-brain",
    color: "background-color: #7f26bf",
  },
];
const total = 4;
</script>

<template>
  <div class="untree_co-section" id="roadmap-section">
    <div class="container">
      <div class="feature-section-header">
        <h1>
          Roadmap
        </h1>
      </div>
      <div class="row justify-content-between">

        <div class="col-lg-5 order-lg-2 js-custom-dots">
          <a v-for="item in items" href="#" class="service link horizontal d-flex active" data-aos="fade-left"
            data-aos-delay="0">
            <div class="service-icon mb-4" :style=item.color>
              <div class="service-icon-center">
                <v-icon :icon=item.icon></v-icon>
              </div>
            </div>
            <div class="service-contents">
              <h3>{{ item.name }}</h3>
              <p>{{ item.des }}</p>
            </div>
          </a>
        </div>
      </div>
    </div>
  </div>
</template>
