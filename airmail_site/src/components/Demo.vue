<script setup>
import Map from "./Map.vue";
import { ref } from 'vue'

const heading = "Demo";
const totalMembers = "50";
const totalTeam = "20";

const pins = ref([])

async function fetchSearchResults(query) {
  const url = `https://api.airmail.rs/search?q=${query}`;
  const response = await fetch(url);
  const data = await response.json();
  var newPins = data.map((poi) => {
    return {
      "type": "Feature",
      "geometry": {
        "type": "Point",
        "coordinates": [poi.lng, poi.lat]
      },
    };
  });
  console.log("Old pins", pins.value);
  console.log("Have pins", newPins);
  pins.value = newPins;
}

</script>

<template>
  <div class="untree_co-section bg-light" id="demo-section">
    <div class="container">
      <div class="row justify-content-between">
        <div class="col-lg-8" data-aos="fade-up" data-aos-delay="100">
          <div id='map' class="img-fluid">
            <Map :pins=pins />
          </div>
        </div>
        <div class="col-lg-4">
          <div class="mb-4" data-aos="fade-up" data-aos-delay="0">
            <h2 class="heading">{{ heading }}</h2>
            <p>
              Airmail is pre-alpha quality software. Data is incomplete and search results may be
              incorrect, missing, or very far away.
            </p>
            <p>
              Try searching for "425 Harvard Ave" or "Seattle Starbucks".
            </p>
          </div>
          <div class="mb-4" data-aos="fade-up" data-aos-delay="100">
            <v-text-field label="Search" @input="async (event) => {
              console.log(event);
              const value = event.target.value;
              if (value.length < 3) {
                return;
              }
              const results = await fetchSearchResults(value);
            }"></v-text-field>
          </div>
        </div>
      </div>
    </div>
  </div>
</template>
