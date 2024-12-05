import { Button } from "@/components/ui/button";
import { NavLink, Outlet } from "react-router";
import type { Route } from "./+types/index";

export function meta({}: Route.MetaArgs) {
  return [
    { title: "Ultimate Tic Tac Toe" },
    {
      name: "Multiplayer Ultimate Tic Tac Toe",
      content: "Welcome to the MMO version of Ultimate Tic Tac Toe",
    },
  ];
}

export default function Index() {
  return (
    <div className="w-full h-full">
      <NavLink to="/game">
        <Button>Enter</Button>
      </NavLink>
      <Outlet />
    </div>
  );
}
