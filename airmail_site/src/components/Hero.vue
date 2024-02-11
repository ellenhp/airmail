<script setup>
import { themeColor } from "../data/items";

const buttonSeeFeatures = "See Features";
const buttonRoadmap = "Roadmap";
const buttonGithub = "GitHub";
</script>

<template>
  <div class="untree_co-hero" id="home-section">
    <div class="container">
      <div class="row align-items-center">
        <div class="col-12">
          <div class="dots"></div>
          <div class="row align-items-center">
            <div class="col-lg-7 ml-auto order-lg-2" data-aos="fade-right" data-aos-delay="400">
            </div>
            <div class="col-lg-5">
              <h1 class="heading" data-aos="fade-up" data-aos-delay="0">
                <p>
                  Search the planet with Airmail.
                </p>
              </h1>
              <div class="excerpt" data-aos="fade-up" data-aos-delay="100">
                <p>
                  Airmail is an extremely lightweight, open-source geocoder written in Rust. A search engine for the
                  planet. Airmail is currently able to parse English queries like "1600 Pennsylvania Ave NW" and POI
                  queries like "Seattle Starbucks". Result quality is fair for English queries in North America, and
                  support for more regions and languages is next on the roadmap.
                </p>
              </div>
              <p data-aos="fade-up" data-aos-delay="200">
                <a href="#features-section" class="btn btn-primary smoothscroll"
                  style="margin-right: 10px; margin-bottom: 10px;" :style="[
                    { background: themeColor },
                    { borderColor: themeColor },
                  ]">{{ buttonSeeFeatures }}</a>
                <a href="#roadmap-section" class="btn btn-outline-primary smoothscroll roadmap"
                  style="margin-right: 10px; margin-bottom: 10px;"
                  :style="[{ color: themeColor }, { borderColor: themeColor }]">{{ buttonRoadmap }}</a>
                <a href="#demo-section" class="btn btn-outline-primary smoothscroll roadmap"
                  style="margin-right: 5px; margin-bottom: 10px;"
                  :style="[{ color: themeColor }, { borderColor: themeColor }]">Demo</a>
              </p>
            </div>
          </div>
        </div>
      </div>
    </div>
  </div>
</template>

<style scoped>
.roadmap:hover {
  color: #fff !important;
}
</style>
