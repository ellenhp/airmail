<template>
    <div class="map-wrap">
        <div class="map" ref="mapContainer"></div>
    </div>
</template>
  
<script>
import { Map } from 'maplibre-gl';
import { shallowRef, onMounted, onUnmounted, markRaw, ref, getCurrentInstance } from 'vue';

export default {
    name: "Map",
    data: () => ({
        toLoadPins: [],
    }),
    props: {
        pins: {
            type: Array,
            default: () => [],
        },
    },
    methods: {
        async loadPins(val) {
            if (this.mapRef === null) {
                return;
            }
            const source = this.mapRef.getSource('point');
            source.setData({
                'type': 'FeatureCollection',
                'features': val,
            });

            let bounds = [[180, 90], [-180, -90]];
            val.forEach((pin) => {
                bounds[0][0] = Math.min(bounds[0][0], pin.geometry.coordinates[0]);
                bounds[0][1] = Math.min(bounds[0][1], pin.geometry.coordinates[1]);
                bounds[1][0] = Math.max(bounds[1][0], pin.geometry.coordinates[0]);
                bounds[1][1] = Math.max(bounds[1][1], pin.geometry.coordinates[1]);
            });
            this.mapRef.fitBounds(bounds, {
                padding: 50,
                maxZoom: 15,
            });
        },
    },
    watch: {
        async pins(val, oldVal) {
            if (this.mapRef === null) {
                this.toLoadPins = val;
                return;
            }
            await this.loadPins(val);
        },
    },
    setup(props) {
        const mapContainer = shallowRef(null);
        const mapRef = ref(null);

        onMounted(() => {
            const map = new Map({
                container: mapContainer.value,
                style: `https://maps.earth/tileserver/styles/basic/style.json`,
            });
            map.on('load', async () => {
                map["dragRotate"].disable();
                map["touchZoomRotate"].disable();
                map["doubleClickZoom"].disable();
                map["scrollZoom"].disable();
                map["boxZoom"].disable();
                map["keyboard"].disable();
                map["dragPan"].disable();
                map["touchPitch"].disable();
                mapRef.value = markRaw(map);
                console.log("Map loaded", map);
                const image = await map.loadImage(`${location.protocol}//${window.location.host}/images/pin.png`);
                console.log("Loaded image", image);
                map.addImage('marker', image.data);
                map.addSource('point', {
                    'type': 'geojson',
                    'data': {
                        'type': 'FeatureCollection',
                        'features': [
                        ]
                    }
                });
                map.addLayer({
                    'id': 'points',
                    'type': 'symbol',
                    'source': 'point',
                    'layout': {
                        'icon-image': 'marker',
                        'icon-size': 0.05,
                        'icon-anchor': 'bottom',
                    }
                });
            });

        });
        onUnmounted(() => {
            mapRef.value?.remove();
        });

        return {
            mapRef, mapContainer
        };
    },
    props: ['pins'],

};
</script>
  
  
<style scoped>
.map-wrap {
    position: relative;
    width: 100%;
    height: calc(100vh - 77px);
    /* calculate height of the screen minus the heading */
}

.watermark {
    position: absolute;
    left: 10px;
    bottom: 10px;
    z-index: 999;
}
</style>