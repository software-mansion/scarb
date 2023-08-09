<script setup>
import DefaultTheme from "vitepress/theme";
import Footer from "../components/Footer.vue";

const { Layout } = DefaultTheme;
</script>

<template>
  <Layout>
    <template #doc-bottom>
      <Footer class="footer-doc" />
    </template>
    <template #page-bottom>
      <Footer />
    </template>
  </Layout>
</template>

<style scoped>
.footer-doc {
  padding-left: 0;
  padding-right: 0;
}
</style>
