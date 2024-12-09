import {
  type RouteConfig,
  index,
  prefix,
  route,
} from "@react-router/dev/routes";

export default [
  index("routes/index.tsx"),
  ...prefix("game", [
    index("routes/game/index.tsx"),
    route(":id", "routes/game/game.tsx"),
    route("all", "routes/game/all.tsx"),
  ]),
] satisfies RouteConfig;
