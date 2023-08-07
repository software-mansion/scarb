// https://vitepress.dev/guide/custom-theme
import Theme from "vitepress/theme";
import "./style.css";
import BigLink from "../components/BigLink.vue";
import Layout from "./Layout.vue";

export default {
  extends: Theme,
  Layout,
  enhanceApp({ app, router, siteData }) {
    app.component("BigLink", BigLink);
  },
};
