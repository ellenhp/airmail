<script setup>
import Map from "./Map.vue";
import { ref } from 'vue'

const heading = "Demo";
const totalMembers = "50";
const totalTeam = "20";

const pins = ref([])

async function fetchSearchResults(query) {
  if (query.length < 3) {
    pins.value = [];
    return;
  }
  const url = `https://api2.airmail.rs/search?q=${query}`;
  const response = await fetch(url);
  const data = await response.json();
  var newPins = data.features.map((poi) => {
    return {
      "type": "Feature",
      "geometry": {
        "type": "Point",
        "coordinates": [poi.lng, poi.lat]
      },
    };
  });
  pins.value = newPins;
}

</script>

<template>
  <div class="untree_co-section bg-light" id="demo-section">
    <div class="container">
      <div class="row justify-content-between">
        <div class="mb-4" data-aos="fade-up" data-aos-delay="0">
          <h2 class="heading">{{ heading }}</h2>
          <p>
            Airmail is pre-alpha quality software. Data is incomplete and search results may be
            incorrect, missing, or very far away. Airmail currently only indexes addresses and businesses, so queries must
            be specific. Administrative areas like cities, states, and countries are not currently indexed.
          </p>
          <p>
            Try searching for "425 Harvard Ave" or "Seattle Starbucks".
          </p>
        </div>
        <v-text-field class="searchbar" label="Search" @input="async (event) => {
          console.log(event);
          const value = event.target.value;
          if (value.length < 3) {
            return;
          }
          const results = await fetchSearchResults(value);
        }"></v-text-field>
        <Map :pins=pins />

      </div>
    </div>
  </div>
</template>
