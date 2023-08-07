<script setup>
import DefaultTheme from "vitepress/theme";
import Footer from "../components/Footer.vue";

const { Layout } = DefaultTheme;
</script>

<template>
  <Layout>
    <template #doc-bottom>
      <Footer />
    </template>
    <template #page-bottom>
      <Footer />
    </template>
  </Layout>
</template>
