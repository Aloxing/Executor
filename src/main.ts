import { createApp } from "vue";
import "./style.css";
import App from "./App.vue";

// Disable the browser context menu so right-click does nothing in the app.
window.addEventListener("contextmenu", (event) => event.preventDefault());

createApp(App).mount("#app");
