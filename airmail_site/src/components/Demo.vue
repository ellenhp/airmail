<script setup>
import Map from "./Map.vue";
import { ref } from 'vue'
import pDebounce from 'p-debounce';

const heading = "Demo";

const pins = ref([])
const latestSearchSeq = ref(0)
const latestResultSeq = ref(0)
const isSearching = ref(false);

const debouncedPreload = pDebounce(fetchSearchResults, 100);
const debouncedSearch = pDebounce(fetchSearchResults, 500);

async function fetchSearchResults(query, updatePins) {
  const seq = ++latestSearchSeq.value;
  isSearching.value = true;

  const url = `https://api2.airmail.rs/search?q=${query}&lenient=${updatePins ? "true" : "false"}`;
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
  if (seq > latestResultSeq.value && updatePins) {
    latestResultSeq.value = seq;
    pins.value = newPins;
    if (seq == latestSearchSeq.value) {
      isSearching.value = false;
    }
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
            incorrect, missing, or very far away. Airmail currently only indexes addresses and venues, so queries must
            be specific. Administrative areas like cities, states, and countries are not currently indexed. Some road
            types may not have their abbreviations indexed properly.
          </p>
          <p>
            Try searching for "1600 Pennsylvania Ave NW", "Central Park, NYC", or "University District pizza, Seattle".
          </p>
        </div>
        <v-text-field class="searchbar" label="Search" @input="async (event) => {
          debouncedPreload(event.target.value, false);
          debouncedSearch(event.target.value, true);
        }"></v-text-field>
        <div v-if="isSearching">
          <v-progress-circular indeterminate color="primary" style="margin: 10px;"></v-progress-circular>
        </div>
        <Map :pins=pins />

      </div>
    </div>
  </div>
</template>
