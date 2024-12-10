import { Route } from ".react-router/types/app/routes/game/+types/game";
import { useParams } from "react-router";

export async function clientLoader({ params }: Route.ClientLoaderArgs) {
  const res = await fetch(
    `${import.meta.env.VITE_API_BASE_URL}/api/matches/${params.id}`
  );
  const match = await res.json();
  return match;
}

export default function Game() {
  const { id } = useParams();
  return <div className="w-full h-full">how did you find this</div>;
}
