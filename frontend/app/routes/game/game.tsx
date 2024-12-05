import { Route } from ".react-router/types/app/routes/game/+types/game";
import { useParams } from "react-router";

export async function clientLoader({ params }: Route.ClientLoaderArgs) {
  const res = await fetch(`http://localhost:8000/api/matches/${params.id}`);
  const match = await res.json();
  console.log(match);
  return match;
}

export default function Game() {
  const { id } = useParams();
  console.log(id);
  return <div className="w-full h-full">game route</div>;
}
