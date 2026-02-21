<script setup lang="ts">
import Button from './components/Button.vue'
import Input from './components/Input.vue'
import Card from './components/Card.vue'
import Badge from './components/Badge.vue'
</script>

<template>
  <div class="app">
    <h1>Vite + Vue + Musea Example</h1>
    <p>
      Visit <a href="/__musea__">/__musea__</a> to see the component gallery.
    </p>

    <section class="section">
      <h2>Buttons</h2>
      <div class="demo">
        <Button>Default</Button>
        <Button variant="primary">Primary</Button>
        <Button variant="secondary">Secondary</Button>
        <Button disabled>Disabled</Button>
      </div>
    </section>

    <section class="section">
      <h2>Inputs</h2>
      <div class="demo demo--column">
        <Input placeholder="Text input..." />
        <Input type="search" placeholder="Search..." />
        <Input error="This field is required" placeholder="With error..." />
      </div>
    </section>

    <section class="section">
      <h2>Badges</h2>
      <div class="demo">
        <Badge>Default</Badge>
        <Badge variant="success">Success</Badge>
        <Badge variant="warning">Warning</Badge>
        <Badge variant="error">Error</Badge>
        <Badge variant="info">Info</Badge>
      </div>
    </section>

    <section class="section">
      <h2>Cards</h2>
      <div class="demo">
        <Card title="Default Card" description="A simple card component.">
          <Button variant="primary">Action</Button>
        </Card>
        <Card
          title="Featured"
          description="A card with image header."
          image="linear-gradient(135deg, #6b5090 0%, #3a7a70 100%)"
          variant="elevated"
        >
          <Button variant="primary">View</Button>
        </Card>
      </div>
    </section>
  </div>
</template>

<style>
body {
  background: #e6e2d6;
  color: #121212;
  margin: 0;
}

.app {
  max-width: 800px;
  margin: 0 auto;
  padding: 2rem;
  font-family: 'Helvetica Neue', Helvetica, Arial, sans-serif;
}

h1 {
  color: #121212;
  font-weight: 700;
}

h2 {
  color: #3a3a3a;
  font-size: 1.125rem;
  font-weight: 600;
  margin-bottom: 0.75rem;
}

a {
  color: #121212;
  font-weight: 500;
}

a:hover {
  color: #6b5090;
}

.section {
  margin-top: 2rem;
}

.demo {
  display: flex;
  gap: 1rem;
  flex-wrap: wrap;
  align-items: flex-start;
}

.demo--column {
  flex-direction: column;
  max-width: 320px;
}
</style>
