import { Route } from ".react-router/types/app/routes/game/+types";
import { Outlet } from "react-router";

export default function Index({}: Route.ComponentProps) {
  return (
    <div className="w-full h-full">
      games route
      <Outlet />
    </div>
  );
}
