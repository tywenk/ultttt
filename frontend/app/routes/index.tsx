import { Button } from "@/components/ui/button";
import { NavLink } from "react-router";
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
    <div className="w-full mx-auto h-full min-h-screen max-w-prose grid place-content-center text-center gap-2">
      <h1 className="font-medium text-xl">
        Welcome to Ultimate Tic Tac Toe MMO
      </h1>
      <NavLink to="/game">
        <Button>Enter</Button>
      </NavLink>
    </div>
  );
}
