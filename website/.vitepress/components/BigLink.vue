<script setup>
defineProps(["href"]);
</script>

<template>
  <p :class="$style.p">
    <a :href="$props.href" target="_blank" rel="noreferrer">
      <slot />
      →
    </a>
  </p>
</template>

<style module>
.p {
  font-size: 1.1em;
  line-height: 2.5em !important;
  text-align: center;
}
</style>
