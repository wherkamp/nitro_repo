import {createRouter, createWebHistory, RouteRecordRaw} from "vue-router";
import Home from "../views/Home.vue";
import Install from "../views/Install.vue";
import Login from "../views/Login.vue";
import Admin from "../views/Admin.vue";
import Browse from "../views/Browse.vue";
import Upload from "../views/Upload.vue";
import Project from "../views/Project.vue";

const routes: Array<RouteRecordRaw> = [
  {
    path: "/",
    name: "Home",
    component: Home,
  },
  {
    path: "/install",
    name: "Install",
    component: Install,
  },
  {
    path: "/admin",
    name: "Admin",
    component: Admin,
  },
  {
    path: "/upload/:storage/:repo",
    name: "Upload",
    component: Upload,
  },
  {
    path: "/login",
    name: "Login",
    component: Login,
  },
  {
    path: "/browse/:storage?/:repo?/:catchAll(.*)?",
    name: "Browse",
    component: Browse,
  },
  {
    path: "/browse/:storage/:repo/:catchAll(.*)",
    name: "Project",
    component: Project,
  },
];

const router = createRouter({
  history: createWebHistory(process.env.BASE_URL),
  routes,
});

export default router;
