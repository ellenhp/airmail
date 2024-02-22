<script setup>
import MenuIcon from 'vue-material-design-icons/Menu.vue';

const items = [
  {
    id: 1,
    name: "Open-source",
    des: "Freely available under a permissive license (MIT/Apache 2.0 dual license).",
    icon: "mdi-xml",
    color: "background-color: #6fe7dd",
  },
  {
    id: 2,
    name: "Lightweight",
    des: "Capable of querying a remote index.",
    icon: "mdi-airplane",
    color: "background-color: #ffc93c",
  },
  {
    id: 3,
    name: "International",
    des: "Reasonable support for queries in English, Spanish, Catalan, and likely others. Open an issue on GitHub if you have problems!",
    icon: "mdi-translate",
    color: "background-color: #7f26bf",
  },
  {
    id: 3,
    name: "Typo-tolerant",
    des: "Limited support for typo tolerance.",
    icon: "mdi-undo",
    color: "background-color: #f73859",
  },
];
const total = 4;
</script>

<template>
  <div class="untree_co-section" id="features-section">
    <div class="container">
      <div class="feature-section-header">
        <h1>
          Features
        </h1>
      </div>
      <div class="row justify-content-between">
        <div class="col-lg-5 order-lg-2 js-custom-dots">
          <p v-for="item in items" href="#" class="service link horizontal d-flex active" data-aos="fade-left"
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
          </p>
        </div>
      </div>
    </div>
  </div>
</template>
