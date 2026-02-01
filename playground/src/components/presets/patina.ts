/**
 * Patina Playground Preset
 *
 * This preset demonstrates the Patina linter with intentional issues:
 *
 * Vue rules:
 * - vue/require-v-for-key: Missing :key in v-for
 * - vue/no-use-v-if-with-v-for: v-if with v-for on same element
 * - vue/use-unique-element-ids: Static id attributes
 * - vue/no-v-html: XSS risk with v-html
 *
 * Accessibility (a11y) rules:
 * - a11y/img-alt: Missing alt attribute
 * - a11y/anchor-has-content: Empty anchor
 * - a11y/heading-has-content: Empty heading
 * - a11y/click-events-have-key-events: Click without keyboard handler
 * - a11y/tabindex-no-positive: Positive tabindex
 * - a11y/form-control-has-label: Input without label
 * - a11y/aria-props: Invalid ARIA attribute
 * - a11y/aria-role: Invalid/abstract ARIA role
 *
 * Note: This file is separate from the Vue component to avoid
 * linting issues with embedded Vue code in template literals.
 */

export const LINT_PRESET = `<script setup lang="ts">
import { ref } from 'vue'

const items = ref([
  { name: 'Item 1' },
  { name: 'Item 2' },
])

const users = ref([
  { id: 1, name: 'Alice', active: true },
  { id: 2, name: 'Bob', active: false },
])

const products = ref([
  { id: 1, name: 'Product A', inStock: true },
  { id: 2, name: 'Product B', inStock: false },
])

const htmlContent = '<b>Hello</b>'
const handleClick = () => {}
</script>

<template>
  <div class="container">
    <!-- vue/require-v-for-key: Missing :key attribute -->
    <ul>
      <li v-for="item in items">{{ item.name }}</li>
    </ul>

    <!-- vue/no-use-v-if-with-v-for: v-if with v-for on same element -->
    <div v-for="user in users" v-if="user.active" :key="user.id">
      {{ user.name }}
    </div>

    <!-- a11y/img-alt: Missing alt attribute -->
    <img src="/logo.png" />

    <!-- a11y/anchor-has-content: Empty anchor -->
    <a href="/home"></a>

    <!-- a11y/heading-has-content: Empty heading -->
    <h1></h1>

    <!-- a11y/click-events-have-key-events: Click without keyboard handler -->
    <div @click="handleClick">Click me</div>

    <!-- a11y/tabindex-no-positive: Positive tabindex -->
    <button tabindex="5">Bad Tab Order</button>

    <!-- a11y/form-control-has-label: Input without label -->
    <input type="text" placeholder="Enter name" />

    <!-- a11y/aria-props: Invalid ARIA attribute (typo) -->
    <input aria-labeledby="label-id" />

    <!-- a11y/aria-role: Invalid ARIA role -->
    <div role="datepicker"></div>

    <!-- a11y/aria-role: Abstract ARIA role -->
    <div role="range"></div>

    <!-- vue/use-unique-element-ids: Static id attribute -->
    <label for="user-input">Username:</label>
    <input id="user-input" type="text" />

    <!-- vue/no-v-html: XSS risk -->
    <div v-html="htmlContent"></div>

    <!-- Valid code for comparison -->
    <template v-for="product in products" :key="product.id">
      <div v-if="product.inStock">
        {{ product.name }}
      </div>
    </template>
  </div>
</template>

<style scoped>
.container {
  padding: 20px;
}
</style>
`;
