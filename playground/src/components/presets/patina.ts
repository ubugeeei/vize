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
 * - vue/no-boolean-attr-value: Explicit value on boolean attribute
 *
 * HTML conformance rules:
 * - html/deprecated-element: Deprecated HTML element
 * - html/no-consecutive-br: Consecutive <br> tags
 * - html/deprecated-attr: Deprecated HTML attribute
 * - html/require-datetime: Missing datetime on <time>
 * - html/id-duplication: Duplicate id attributes
 * - html/no-empty-palpable-content: Empty palpable content element
 * - html/no-duplicate-dt: Duplicate <dt> terms in <dl>
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
 * - a11y/heading-levels: Skipped heading level
 * - a11y/use-list: Bullet-like text without list markup
 * - a11y/placeholder-label-option: Placeholder option without disabled
 * - a11y/landmark-roles: Duplicate landmark without label
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

    <!-- vue/no-boolean-attr-value: Explicit value on boolean attribute -->
    <input type="checkbox" disabled="disabled" />
    <button disabled="true">Submit</button>

    <!-- html/deprecated-element: Deprecated HTML element -->
    <center>Centered content</center>
    <font>Styled text</font>

    <!-- html/no-consecutive-br: Consecutive <br> tags -->
    <p>Line 1<br /><br /><br />Line 2</p>

    <!-- html/deprecated-attr: Deprecated attribute (use CSS instead) -->
    <table bgcolor="gray">
      <tr>
        <td align="center" valign="top">Cell</td>
      </tr>
    </table>
    <body background="bg.png">
      <!-- html/require-datetime: Missing datetime on <time> -->
      <time>January 1st</time>

      <!-- html/id-duplication: Duplicate id attributes -->
      <div id="dup-section">First</div>
      <div id="dup-section">Second</div>

      <!-- html/no-empty-palpable-content: Empty palpable content -->
      <p></p>
      <span></span>

      <!-- html/no-duplicate-dt: Duplicate <dt> in <dl> -->
      <dl>
        <dt>Term A</dt>
        <dd>Definition A-1</dd>
        <dt>Term A</dt>
        <dd>Definition A-2</dd>
      </dl>

      <!-- a11y/heading-levels: Skipped heading level (h1 -> h3) -->
      <h1>Title</h1>
      <h3>Subsection</h3>

      <!-- a11y/use-list: Bullet-like text without list markup -->
      <div>
        <p>- Item one</p>
        <p>- Item two</p>
        <p>- Item three</p>
      </div>

      <!-- a11y/placeholder-label-option: Missing disabled on placeholder -->
      <select>
        <option value="">Choose...</option>
        <option value="a">Option A</option>
      </select>

      <!-- a11y/landmark-roles: Duplicate landmarks without aria-label -->
      <nav>Primary nav</nav>
      <nav>Secondary nav</nav>
    </body>

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
