import { Route } from ".react-router/types/app/routes/game/+types/all";

export async function clientLoader({ params }: Route.ClientLoaderArgs) {
  const res = await fetch(`${import.meta.env.VITE_API_BASE_URL}/api/matches`);
  const match = await res.json();
  return match;
}

export default function AllGames() {
  return <div className="w-full h-full">all games route</div>;
}
