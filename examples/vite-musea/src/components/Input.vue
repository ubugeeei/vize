<script setup lang="ts">
defineProps<{
  modelValue?: string
  placeholder?: string
  type?: 'text' | 'email' | 'password' | 'search'
  disabled?: boolean
  error?: string
}>()

defineEmits<{
  'update:modelValue': [value: string]
}>()
</script>

<template>
  <div class="input-wrapper">
    <input
      class="input"
      :class="{ 'input--error': error, 'input--disabled': disabled }"
      :type="type ?? 'text'"
      :value="modelValue"
      :placeholder="placeholder"
      :disabled="disabled"
      @input="$emit('update:modelValue', ($event.target as HTMLInputElement).value)"
    >
    <span v-if="error" class="input-error">{{ error }}</span>
  </div>
</template>

<style scoped>
.input-wrapper {
  display: flex;
  flex-direction: column;
  gap: 0.25rem;
}

.input {
  padding: 0.5rem 0.75rem;
  border: 1px solid #d1d5db;
  border-radius: 6px;
  font-size: 0.875rem;
  outline: none;
  transition: border-color 0.2s, box-shadow 0.2s;
  width: 100%;
}

.input:focus {
  border-color: #3b82f6;
  box-shadow: 0 0 0 3px rgba(59, 130, 246, 0.15);
}

.input--error {
  border-color: #ef4444;
}

.input--error:focus {
  box-shadow: 0 0 0 3px rgba(239, 68, 68, 0.15);
}

.input--disabled {
  opacity: 0.5;
  cursor: not-allowed;
  background: #f9fafb;
}

.input-error {
  color: #ef4444;
  font-size: 0.75rem;
}
</style>

<art title="Input" category="Forms" status="ready" tags="input,form,text">
  <variant name="Default" default>
    <Self placeholder="Enter text..." />
  </variant>
  <variant name="With Value">
    <Self model-value="Hello, Musea!" placeholder="Enter text..." />
  </variant>
  <variant name="Search">
    <Self type="search" placeholder="Search..." />
  </variant>
  <variant name="With Error">
    <Self model-value="bad@" error="Invalid email address" placeholder="Enter email..." />
  </variant>
  <variant name="Disabled">
    <Self model-value="Read only" disabled placeholder="Disabled input" />
  </variant>
</art>
