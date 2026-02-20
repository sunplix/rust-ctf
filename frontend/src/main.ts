import { createApp } from "vue";

import App from "./App.vue";
import router from "./router";
import { pinia } from "./stores";
import { useAppStore } from "./stores/app";
import "./assets/main.css";

const app = createApp(App);
app.use(pinia);
app.use(router);

const appStore = useAppStore();
appStore.hydrateFromStorage();
void appStore.loadSiteSettings();

app.mount("#app");
