<script setup>
import Map from "./Map.vue";
import { ref } from 'vue'
import pDebounce from 'p-debounce';

const heading = "Demo";

const pins = ref([])
const latestSearchSeq = ref(0)
const latestResultSeq = ref(0)

const debouncedSearch = pDebounce(fetchSearchResults, 200);

async function fetchSearchResults(query) {
  const seq = ++latestSearchSeq.value;
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
  if (seq > latestResultSeq.value) {
    latestResultSeq.value = seq;
    let pinCluster = [newPins[0]];
    let max_distance = 0.5;
    for (let i = 1; i < newPins.length; i++) {
      if (Math.abs(newPins[i].geometry.coordinates[0] - pinCluster[0].geometry.coordinates[0]) < max_distance &&
        Math.abs(newPins[i].geometry.coordinates[1] - pinCluster[0].geometry.coordinates[1]) < max_distance) {
        pinCluster.push(newPins[i]);
      }
    }

    pins.value = pinCluster;
  } else {
    return;
  }
}

</script>

<template>
  <div class="untree_co-section" id="demo-section">
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
          await debouncedSearch(event.target.value);
        }"></v-text-field>
        <Map :pins=pins />

      </div>
    </div>
  </div>
</template>
