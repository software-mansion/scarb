<script setup>
defineProps({
  lang: String,
  src: String,
});
</script>

<template>
  <div :class="`language-${$props.lang}`">
    <button class="copy" title="Copy Code"></button>
    <span class="lang">{{ $props.lang }}</span>
    <!-- NOTE: Whitespace is important here. -->
    <pre
      class="shiki"
    ><code v-for="line in $props.src.split('\n')"><span class="line">{{ line }}</span></code></pre>
  </div>
</template>

<style module></style>
