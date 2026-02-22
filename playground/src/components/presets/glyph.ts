/**
 * Glyph Playground Preset
 *
 * This preset demonstrates code formatting with intentionally
 * unformatted/minified Vue SFC code:
 * - No proper indentation
 * - Missing spaces around operators
 * - Condensed CSS rules
 * - Non-canonical block order (template first, then script)
 * - Long-form directives (v-bind:, v-on:) to demonstrate normalization
 *
 * Used to showcase the Glyph formatter's capabilities.
 *
 * Note: This file is separate from the Vue component to avoid
 * linting issues with embedded Vue code in template literals.
 */

export const GLYPH_PRESET = `<template>
<div class="container">
<h1>{{ count }}</h1>
<p>Doubled: {{ doubled }}</p>
<div v-bind:class="cls" v-on:click="handle" id="app" ref="el">
<button @click="decrement">-1</button>
<button @click="increment">+1</button>
</div>
</div>
</template>

<script setup lang="ts">
import {ref,computed,watch} from 'vue'

const count=ref(0)
const doubled=computed(()=>count.value*2)

function increment(){count.value++}
function decrement(){count.value--}

watch(count,(newVal,oldVal)=>{
console.log(\`Count changed from \${oldVal} to \${newVal}\`)
})
</script>

<style scoped>
.container{padding:20px;background:#f0f0f0}
h1{color:#333;font-size:2rem}
.buttons{display:flex;gap:10px}
button{padding:8px 16px;cursor:pointer}
</style>
`;
