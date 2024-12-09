import { Route } from ".react-router/types/app/routes/game/+types/all";

export async function clientLoader({ params }: Route.ClientLoaderArgs) {
  const res = await fetch(`http://localhost:8000/api/matches`);
  const match = await res.json();
  console.log(match);
  return match;
}

export default function AllGames() {
  return <div className="w-full h-full">all games route</div>;
}
